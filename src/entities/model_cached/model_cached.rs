use super::*;
use crate::entities::{
    Bounds,
    model_cached::{
        compartment_cache::build_cache::BuildCompartmentCache,
        displacement_bound_cache::build_cache::BuildDisplacementBoundCache,
        displacement_cache::build_cache::BuildDisplacementCache,
    },
};
use core::f64;
use indexmap::IndexMap;
use parry2d_f64::math::Vec3;
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
    compartments_bounded: IndexMap<String, BuildDisplacementBoundCache>,
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
        let hull = load_stl(&conf.model_dir.clone().join(PathBuf::from("hull.stl")), conf.model_scale);
        let hull = Arc::new(hull);
        let displacement = BuildDisplacementCache::new(
            &dbg,
            Arc::clone(&hull),
            conf.model_x,
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
            bounds.clone(),
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
            let Some(name) = path.file_stem() else { continue };
            let name = name.to_str().unwrap().to_string();
            let mesh = Arc::new(load_stl(&path, conf.model_scale));
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
                BuildDisplacementBoundCache::new(
                    &dbg,
                    Arc::clone(&mesh),
                    conf.bounds_level_step,
                    bounds.clone(),
                    Arc::clone(&thread_pool),
                ),
            );
        }
        let windage_area = WindageArea::new(
            &dbg,
            Arc::clone(&hull),
            conf.model_x,
            conf.draught_min,
            conf.ship_length_lbp,
            10000,
        );
        let model_cached = Self {
            dbg: dbg.clone(),
            cache_dir: conf.cache_dir.clone(),
            displacement,
            compartments,
            windage_area,
            displacement_bounded,
            compartments_bounded,
        };
        //   dbg!(model_cached.compartments.len());
        Ok(model_cached)
    }
    ///
    pub fn rebuld(&self) -> Result<(), Error> {
        log::info!("rebuild begin");
        let error = Error::new(&self.dbg, "rebuild");
        self.rebuild_hull().map_err(|err| error.pass(err))?;
        self.rebuild_compartments().map_err(|err| error.pass(err))?;
        self.rebuild_windage().map_err(|err| error.pass(err))?;
        Ok(())
    }
    ///
    /// Пересчет кэшей
    #[allow(dead_code)]
    pub fn rebuild_hull(&self) -> Result<(), Error> {
        log::info!("rebuild_hull begin");
        let error = Error::new(&self.dbg, "rebuild_hull");
        let (result, errors) = self.displacement.build();
        if !errors.is_empty() {
            return Err(error.pass_with(
                "rebuild_hull",
                errors.iter().fold(String::new(), |acc, err| {
                    format!("{acc}\n\tIn error: {err}")
                }),
            ));
        }
        let (result, errors) = self.displacement_bounded.build();
        if !errors.is_empty() {
            return Err(error.pass_with(
                "rebuild_hull",
                errors.iter().fold(String::new(), |acc, err| {
                    format!("{acc}\n\tIn error: {err}")
                }),
            ));
        }
        log::info!("rebuild_hull finish");
        Ok(())
    }
    ///  Пересчет кэшей отсеков
    #[allow(dead_code)]
    pub fn rebuild_compartments(&self) -> Result<(), Error> {
        let error: Error = Error::new(&self.dbg, "rebuild_compartments");
        for (name, compartment) in &self.compartments {
            //        println!("model_cached rebuild compartment:{name}");
            let (result, errors) = compartment.build();
            if !errors.is_empty() {
                return Err(error.pass_with(
                    "rebuild compartments",
                    errors.iter().fold(String::new(), |acc, err| {
                        format!("{acc}\n\tIn {name} error: {err}")
                    }),
                ));
            }
        }
        for (name, compartment) in &self.compartments_bounded {
            //        println!("model_cached rebuild compartment:{name}");
            let (result, errors) = compartment.build();
            if !errors.is_empty() {
                return Err(error.pass_with(
                    "rebuild compartments_bounded",
                    errors.iter().fold(String::new(), |acc, err| {
                        format!("{acc}\n\tIn {name} error: {err}")
                    }),
                ));
            }
        }
        Ok(())
    }
    ///
    /// Пересчет кэшей боковой поверхности корпуса
    #[allow(dead_code)]
    pub fn rebuild_windage(&self) -> Result<(), Error> {
        let result = self.windage_area.build();
        Ok(())
    }
}
