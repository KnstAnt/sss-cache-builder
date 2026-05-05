use crate::entities::model_cached::{build_cache::BuildDisplacementCache, compartment_cache::build_cache::BuildCompartmentCache, displacement_bound_cache::build_cache::BuildDisplacementBoundCache};

use super::*;
use core::f64;
use indexmap::IndexMap;
use parry2d_f64::math::Vec3;
use sal_core::{dbg::Dbg, error::Error};
use sal_sync::{
    sync::{RwLock, Stack},
    thread_pool::{JoinHandle, ThreadPool},
};
use std::{collections::HashMap, fmt::Display, path::PathBuf, sync::Arc};

///
/// See [sal_3dlib::props::Attributes] to get more details about what the attribute type is.
pub struct ModelCached {
    dbg: Dbg,
    /// Ship length between perpendiculars
    ship_length_lbp: f64,
    /// 3d model initial position in 3D space (midel).
    model_x: f64,
    /// Waterline coord Z in 3D space (midel) initial position.
    draught_min: f64,
    /// Draught step for hull
    hull_draught_step: f64,
    /// Level step for bounds
    bounds_level_step: f64,
    /// Directory containing [super::ModelCached] caches.
    cache_dir: PathBuf,
    /// Provides a number of calculations:
    /// - cache for model, [heel, trim, draught, volume, x, y, z, area, x, y, z, l_x, l_y ]
    displacement: BuildDisplacementCache,
    /// - cache for compartments, [index of compartments, [heel, trim, level, volume, x, y, z, i_x, i_y ]]
    compartments: IndexMap<String, Arc<RwLock<CompartmentCache>>>,
    /// Композитные отсеки трюмов
    hold_compartments: IndexMap<String, Arc<RwLock<HoldCompartmentCache>>>,
    /// - cache for damaged compartments, [index of compartments, [heel, trim, draught, volume, x, y, z ]]
    damaged_compartments: IndexMap<String, Arc<RwLock<DamagedCompartmentCache>>>,
    /// - cache for windage area
    windage_area: WindageArea,
    /// - cache for bounds of model, [qnt_bounds, cache]
    displacement_bounded: IndexMap<usize, Arc<RwLock<DisplacementBoundCache>>>,
    /// - cache for bounds of compartments, [qnt_bounds, [code, cache]]
    compartments_bounded: IndexMap<usize, IndexMap<String, Arc<RwLock<CompartmentBoundCache>>>>,
    /// Композитные отсеки трюмов, разбиение по шпациям
    hold_compartments_bounded:  IndexMap<usize, IndexMap<String, Arc<RwLock<HoldCompartmentBoundCache>>>>,
    thread_pool: Arc<ThreadPool>,
}
//
//
impl ModelCached {
    ///
    /// Creates a new instance.
    pub fn new(
        parent: &Dbg,
        conf: ModelCachedConf,
        thread_pool: Arc<ThreadPool>,
    ) -> Result<Self, Error> {
        let dbg = Dbg::new(parent, "ModelCached");
        let error = Error::new(&dbg, "new");     
        let hull = load_stl(&conf.model_dir.clone().join(PathBuf::from("hull.stl")))
            .scaled(Vec3::new(conf.model_scale, conf.model_scale, conf.model_scale));
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
            Arc::clone(&thread_pool)
        );
        let bound_displacement = BuildDisplacementBoundCache::new(
            &dbg,
            Arc::clone(&hull),
            conf.bounds_level_step,
            conf.bounds,
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
        let compartments = pathes
            .iter()
            .filter(|path: &&PathBuf| path.file_name().is_some())
            .filter_map(|path| {
                let name = path.file_stem()?;
                let mesh = load_stl(&conf.model_dir.clone().join(PathBuf::from(name)))
                    .scaled(Vec3::new(conf.model_scale, conf.model_scale, conf.model_scale));
                Some((
                    name.clone(),
                    BuildCompartmentCache::new(
                        &dbg,
                        Arc::new(mesh),
                        conf.compartment_heel_steps.clone(),
                        conf.compartment_trim_steps.clone(),
                        conf.compartment_level_step_qnt,
                        Arc::clone(&thread_pool),
                    )))              
            })
            .collect();

        let bound_compartments = 

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
            ship_length_lbp: conf.ship_length_lbp,
            model_x: conf.model_x,
            draught_min: conf.draught_min,
            hull_draught_step: conf.hull_draught_step,
            bounds_level_step: conf.bounds_level_step,
            cache_dir: conf.cache_dir.clone(),
            displacement_shapes,
            windage_shape,
            displacement: DisplacementCache::new(
                &dbg,
                displacement_shape.clone(),
                conf.cache_dir.clone(),
                conf.hull_heel_steps.clone(),
                conf.hull_trim_steps.clone(),
                conf.hull_draught_min,
                conf.hull_draught_max,
                conf.hull_draught_step,
                Arc::clone(&thread_pool),
            ),
            compartments,
            hold_compartments: IndexMap::new(),
            damaged_compartments,
            windage_area,
            displacement_bounded: IndexMap::new(),
            compartments_bounded: IndexMap::new(),
            hold_compartments_bounded: IndexMap::new(),
            thread_pool,
        };
        //   dbg!(model_cached.compartments.len());
        Ok(model_cached)
    }
    /// Пересчет композитных отсеков трюма. Каждый расчет список таких отсеков обновляется
    /// и пересчитывается. К имеющимся отсекам добавляются новые. Старые не удаляются.
    pub fn update_hold_compartments(
        &mut self,
        new_hold_compartments: &Vec<(String, Vec<String>)>,
    ) -> Result<(), Error> {
        //    dbg!(self.dbg.clone(), "update_hold_compartments");
        let error = Error::new(self.dbg.clone(), "update_hold_compartments");
        for (code, codes_array) in new_hold_compartments {
            if !self.hold_compartments.contains_key(code) {
                let compartments: Vec<_> = codes_array
                    .iter()
                    .filter_map(|code| self.compartments.get(code))
                    .map(Arc::clone)
                    .collect();
                let new_hold_compartment = Arc::new(RwLock::new(
                    HoldCompartmentCache::new(&self.dbg, code, compartments)
                        .map_err(|err| error.pass(err))?,
                ));
                self.hold_compartments
                    .insert(code.to_owned(), new_hold_compartment);
            }
            for (qnt_bounds, compartments_bounded) in self.compartments_bounded.iter() {
                let mut hold_compartments_bounded = if let Some(hold_compartments_bounded) =
                    self.hold_compartments_bounded.get(qnt_bounds)
                {
                    hold_compartments_bounded.to_owned()
                } else {
                    IndexMap::new()
                };
                if !hold_compartments_bounded.contains_key(code) {
                    let compartments_bounded: Vec<_> = codes_array
                        .iter()
                        .filter_map(|code| compartments_bounded.get(code))
                        .map(Arc::clone)
                        .collect();
                    let new_hold_compartment_bounded = Arc::new(RwLock::new(
                        HoldCompartmentBoundCache::new(&self.dbg, code, compartments_bounded),
                    ));
                    hold_compartments_bounded.insert(code.to_owned(), new_hold_compartment_bounded);
                }
                self.hold_compartments_bounded
                    .insert(*qnt_bounds, hold_compartments_bounded);
            }
        }
        Ok(())
    }
    ///
    /// Пересчет кэшей корпуса
    #[allow(dead_code)]
    pub fn rebuild_hull(&mut self, bounds: &Bounds) -> Result<(), Error> {
        log::info!("rebuild_hull begin");
        let error = Error::new(&self.dbg, "rebuild_hull");
        let mut errors = Vec::new();
        // Считаем кэши, они сами по себе многопоточны, поэтому делить на потоки нет смысла
        if let Err(error) = self.displacement.calculate() {
            errors.push(("displacement".to_owned(), error));
        }
        let displacement_shape = self
            .displacement_shapes
            .get("hull")
            .ok_or(error.err("no displacement_shape"))?;
        let mut displacement_bound = DisplacementBoundCache::new(
            &self.dbg,
            displacement_shape.clone(),
            self.cache_dir.clone().join("disp_bounded"),
            self.bounds_level_step,
            self.model_x,
            Arc::clone(&self.thread_pool),
        );
        displacement_bound
            .rebuild()
            .map_err(|err| error.pass_with("displacement_bound.rebuild", err))?;
        self.displacement_bounded
            .insert(bounds.len_qnt(), Arc::new(RwLock::new(displacement_bound)));
        self.windage_area
            .rebuild(bounds, self.ship_length_lbp)
            .map_err(|err| error.pass_with("windage_area.rebuild", err))?;
        if !errors.is_empty() {
            return Err(error.pass_with(
                "rebuild_hull",
                errors.iter().fold(String::new(), |acc, (key, err)| {
                    format!("{acc}\n\tIn cache {:?} was error: {err}", key)
                }),
            ));
        }
        log::info!("rebuild_hull finish");
        Ok(())
    }
    ///  Пересчет кэшей отсеков
    #[allow(dead_code)]
    pub fn rebuild_compartments(
        &mut self,
        bounds: &Bounds,
        compartments_max: HashMap<String, (Option<f64>, f64)>,
    ) -> Result<(), Error> {
        let error: Error = Error::new(&self.dbg, "rebuild_compartments");
       let mut errors = Vec::new();
   /*      let mut cache_map = IndexMap::new();
        for (name, compartment) in &mut self.compartments {
            //        println!("model_cached rebuild compartment:{name}");
            let mut guard = compartment.write();
            if let Err(error) = guard.rebuild() {
                errors.push((("compartment ".to_owned() + name), error));
            }
            //   }
            //  for (name, compartment) in &self.compartments {
            //      println!("model_cached build_bounded compartment:{code}");
            //        guard.init().map_err(|err| error.pass_with("compartment_bounded.build_bounded", err))?;
            let (level_max, volume_max) =
                if let Some((level_max, volume_max)) = compartments_max.get(name) {
                    (*level_max, Some(*volume_max))
                } else {
                    (None, None)
                };
            guard
                .calc_coeff(volume_max, level_max)
                .map_err(|err| error.pass_with(format!("compartment:{name}.calc_coeff"), err))?;
            let mut compartment_bounded = guard
                .build_bounded(bounds.clone(), self.bounds_level_step)
                .map_err(|err| error.pass_with("compartment_bounded.build_bounded", err))?;
            compartment_bounded
                .rebuild()
                .map_err(|err| error.pass_with("compartment_bounded.rebuild", err))?;
            cache_map.insert(name.clone(), Arc::new(RwLock::new(compartment_bounded)));
        }
        self.compartments_bounded
            .insert(bounds.len_qnt(), cache_map);
  */      for (name, compartment) in &mut self.damaged_compartments {
            let mut guard = compartment.write();
            let volume_max = if let Some((_, volume_max)) = compartments_max.get(name) {
                Some(*volume_max)
            } else {
                None
            };
            if let Err(error) = guard.rebuild() {
                errors.push((("damaged_compartment ".to_owned() + name), error));
            }
            guard
                .calc_coeff(volume_max)
                .map_err(|err| error.pass_with(format!("compartment:{name}.calc_coeff"), err))?;
        }
        if !errors.is_empty() {
            return Err(error.pass_with(
                "rebuild_compartments",
                errors.iter().fold(String::new(), |acc, (key, err)| {
                    format!("{acc}\n\tIn cache {:?} was error: {err}", key)
                }),
            ));
        }
        Ok(())
    }
    ///
    /// Пересчет кэшей боковой поверхности корпуса
    #[allow(dead_code)]
    pub fn rebuild_windage(&mut self, bounds: &Bounds) -> Result<(), Error> {
        let error: Error = Error::new(&self.dbg, "rebuild_windage");
        self.windage_area
            .rebuild(bounds, self.ship_length_lbp)
            .map_err(|err| error.pass_with("windage_area.rebuild", err))?;
        Ok(())
    }
}

