use super::*;
use crate::entities::model_cached::{
    bound_cache::{BuildCompartmentBoundCache, BuildDisplacementBoundCache},
    compartment_cache::build_cache::BuildCompartmentCache,
    displacement_cache::build_cache::BuildDisplacementCache,
};
use indexmap::IndexMap;
use parry3d_f64::math::Vec3;
use sal_3dlib::load_stl;
use sal_3dlib_core::math::Bounds;
use sal_core::{dbg::Dbg, error::Error};
use sal_sync::{
    sync::{RwLock, Stack},
    thread_pool::{JoinHandle, ThreadPool},
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

///
/// See [sal_3dlib::props::Attributes] to get more details about what the attribute type is.
pub struct ModelCached {
    dbg: Dbg,
    cache_dir: PathBuf,
    bounds: Bounds,
    /// Provides a number of calculations:
    /// - cache for model, [heel, trim, draught, volume, x, y, z, area, x, y, z, l_x, l_y ]
    displacement: BuildDisplacementCache,
    /// - cache for compartments, [index of compartments, [heel, trim, level, volume, x, y, z, i_x, i_y ]]
    compartments: IndexMap<String, BuildCompartmentCache>,
    /// - cache for windage area
    windage_area: WindageArea,
    /// - cache for bounds of model, [qnt_bounds, cache]
    displacement_bounded: BuildDisplacementBoundCache,
    /// - cache for bounds of compartments, [qnt_bounds, [code, cache]]
    compartments_bounded: IndexMap<String, BuildCompartmentBoundCache>,
}
//
//
impl ModelCached {
    ///
    /// Creates a new instance.
    pub fn new(
        parent: &Dbg,
        conf: ModelCachedConf,
        bounds: Bounds,
        thread_pool: Arc<ThreadPool>,
    ) -> Result<Self, Error> {
        let dbg = Dbg::new(parent, "ModelCached");
        let error = Error::new(&dbg, "new");
        let hull = load_stl(
            &conf.model_dir.clone().join(PathBuf::from("hull.stl")),
            conf.model_scale,
        )
        .map_err(|err| error.pass(err))?;
        let hull = Arc::new(hull);
        let displacement = BuildDisplacementCache::new(
            &dbg,
            Arc::clone(&hull),
            conf.midel_x,
            conf.hull_heel_steps,
            conf.hull_trim_steps,
            conf.hull_draught_min,
            conf.hull_draught_max,
            conf.hull_draught_step,
            Arc::clone(&thread_pool),
        );
        let displacement_bounded = BuildDisplacementBoundCache::new(
            &dbg,
            Arc::clone(&hull),
            conf.bounds_level_step,
            Arc::clone(&thread_pool),
        );
        let path = conf.model_dir.clone().join(PathBuf::from("compartments"));
        let pathes: Vec<_> = match std::fs::read_dir(&path) {
            Ok(dir) => dir
                .into_iter()
                .filter_map(|f| f.ok())
                .map(|f| f.path())
                .collect(),
            Err(err) => {
                log::error!(
                    "{}",
                    error.pass_with(
                        format!("read additional dir {:?}", path.to_str()),
                        err.to_string()
                    )
                );
                Vec::new()
            }
        };
        let mut compartments = IndexMap::new();
        let mut compartments_bounded = IndexMap::new();
        for path in pathes
            .iter()
            .filter(|path: &&PathBuf| path.file_name().is_some())
        {
            let Some(name) = path.file_stem() else {
                continue;
            };
            let name = name.to_str().unwrap().to_string();
            let mesh =
                Arc::new(load_stl(&path, conf.model_scale).map_err(|err| error.pass(err))?);
            compartments.insert(
                name.clone(),
                BuildCompartmentCache::new(
                    &dbg,
                    Arc::clone(&mesh),
                    conf.compartment_heel_steps.clone(),
                    conf.compartment_trim_steps.clone(),
                    conf.compartment_level_step_qnt,
                    Arc::clone(&thread_pool),
                ),
            );
            compartments_bounded.insert(
                name.clone(),
                BuildCompartmentBoundCache::new(
                    &dbg,
                    Arc::clone(&mesh),
                    conf.bounds_level_step,
                    Arc::clone(&thread_pool),
                ),
            );
        }
        let windage_area = WindageArea::new(
            &dbg,
            Arc::clone(&hull),
            conf.midel_x,
            conf.draught_min,
            conf.ship_length_lbp,
            10000,
        );
        let model_cached = Self {
            dbg: dbg.clone(),
            bounds,
            cache_dir: conf.cache_dir.clone(),
            displacement,
            compartments,
            windage_area,
            displacement_bounded,
            compartments_bounded,
        };
        Ok(model_cached)
    }
    ///
    pub fn rebuld(&self) -> Result<(), Error> {
        log::info!("rebuild begin");
        let error = Error::new(&self.dbg, "rebuild");
        self.rebuild_hull().map_err(|err| error.pass(err))?;
        self.rebuild_compartments().map_err(|err| error.pass(err))?;
        self.rebuild_windage().map_err(|err| error.pass(err))?;
        log::info!("rebuild finish");
        Ok(())
    }
    ///
    /// Пересчет кэшей
    #[allow(dead_code)]
    pub fn rebuild_hull(&self) -> Result<(), Error> {
        log::info!("rebuild_hull begin");
        let error = Error::new(&self.dbg, "rebuild_hull");
        if let Err(err) = self.displacement.rebuld_and_save(&self.cache_dir) {
            let error = error.pass_with("displacement rebuld_and_save", err);
            log::error!("{}", format!("{:?}", error));
            return Err(error);
        }
        if let Err(err) = self
            .displacement_bounded
            .rebuld_and_save(&self.bounds, &self.cache_dir.join("disp_bounded"))
        {
            let error = error.pass_with("displacement_bounded rebuld_and_save", err);
            log::error!("{}", format!("{:?}", error));
            return Err(error);
        }
        log::info!("rebuild_hull finish");
        Ok(())
    }
    ///  Пересчет кэшей отсеков
    #[allow(dead_code)]
    pub fn rebuild_compartments(&self) -> Result<(), Error> {
        log::info!("rebuild_compartments begin");
        let error: Error = Error::new(&self.dbg, "rebuild_compartments");
        let cache_dir = self.cache_dir.join("compartments");
        for (name, compartment) in &self.compartments {
            //        println!("model_cached rebuild compartment:{name}");
            if let Err(err) = compartment.rebuld_and_save(&cache_dir.join(name)) {
                let error = error.pass_with("compartment rebuld_and_save", err);
                log::error!("{}", format!("{:?}", error));
                return Err(error);
            }
        }
        for (name, compartment) in &self.compartments_bounded {
            //        println!("model_cached rebuild compartment:{name}");
            if let Err(err) = compartment.rebuld_and_save(&self.bounds, &cache_dir.join(name)) {
                let error = error.pass_with("compartments_bounded rebuld_and_save", err);
                log::error!("{}", format!("{:?}", error));
                return Err(error);
            }
        }
        log::info!("rebuild_compartments finish");
        Ok(())
    }
    ///
    /// Пересчет кэшей боковой поверхности корпуса
    #[allow(dead_code)]
    pub fn rebuild_windage(&self) -> Result<(), Error> {
        log::info!("rebuild_windage begin");
        let error: Error = Error::new(&self.dbg, "rebuild_windage");
        if let Err(err) = self.windage_area.rebuld_and_save(&self.cache_dir) {
            let error = error.pass_with("windage_area rebuld_and_save", err);
            log::error!("{}", format!("{:?}", error));
            return Err(error);
        }
        log::info!("rebuild_windage finish");
        Ok(())
    }
}
