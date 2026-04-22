use sal_core::{dbg::Dbg, error::Error};
use sal_sync::{sync::RwLock, thread_pool::ThreadPool};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::{AtomicBool, Ordering}},
};

use crate::entities::{Position, cache::Cache, model_cached::{DisplacementCacheResult, DisplacementShape, LocalCache, get_from_volume, save}};
///
/// Pre-calculated cache for floating position algorithm.
/// contains keys: [heel, trim, draught]
/// values:[volume, x, y, z, area, x, y, z, waterline_x, waterline_y]
pub struct DisplacementCache {
    dbg: Dbg,
    cache_path: PathBuf,
    heel_steps: Vec<f64>,
    trim_steps: Vec<f64>,
    /// Draught in meters
    draught_min: f64,
    draught_max: f64,
    /// qnt draught steps for hull
    draught_step: f64,
    /// Model representation used for cache calculation.
    shape: Arc<RwLock<DisplacementShape>>,
    /// Cache read from `self.file_path`.
    cache: Option<Cache<f64>>,
    thread_pool: Arc<ThreadPool>,
    exit: Arc<AtomicBool>,
}
//
//
impl DisplacementCache {
    ///
    /// Creates a new instance.
    /// - cache_dir - folder contains all cache files
    pub fn new(
        parent: &Dbg,
        shape: Arc<RwLock<DisplacementShape>>,
        cache_dir: impl AsRef<Path>,
        heel_steps: Vec<f64>,
        trim_steps: Vec<f64>,
        draught_min: f64,
        draught_max: f64,
        draught_step: f64,
        thread_pool: Arc<ThreadPool>,
    ) -> Self {
        let dbg = Dbg::new(parent, "DisplacementCache");
        let path = cache_dir.as_ref().join("displacement_cache");
        assert!(!heel_steps.is_empty());
        assert!(!trim_steps.is_empty());
        Self {
            shape,
            draught_min,
            draught_max,
            heel_steps,
            trim_steps,
            draught_step,
            cache: None,
            cache_path: path,
            dbg,
            thread_pool,
            exit: Arc::new(AtomicBool::new(false)),
        }
    }
    //
    fn calculate(&mut self) -> Vec<Error> {
        let error = Error::new(&self.dbg, "calculate");       
        let (data, mut errors) = super::build_cache::BuildDisplacementCache::new(
            &self.dbg,
            self.shape.clone(),
            self.heel_steps.clone(),
            self.trim_steps.clone(),
            self.draught_min,
            self.draught_max,
            self.draught_step,
            Arc::clone(&self.thread_pool),
            self.exit.clone(),
        )
        .build();
        let cache = if let Some(cache) = self.cache.take() {
            cache
        } else {
            Cache::<f64>::new(&self.dbg)
        };
        if let Err(err) = cache.init(data.clone()) {
            errors.push(error.pass_with("self.cache.get_mut", err));
        }
        if let Err(err) = save(&self.dbg, &self.cache_path, data) {
            errors.push(error.pass_with("save data", err));
        }
        errors
    }
}
