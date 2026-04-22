use sal_core::{dbg::Dbg, error::Error};
use sal_sync::{sync::RwLock, thread_pool::ThreadPool};
use std::{
    path::PathBuf,
    sync::{
        Arc, OnceLock,
    },
};

use crate::entities::{
    Bounds,
    cache::Cache,
    model_cached::{DisplacementShape, save},
};
///
/// Pre-calculated cache for bounds
pub struct DisplacementBoundCache {
    dbg: Dbg,
    cache_dir: PathBuf,
    level_step: f64,
    center_x: f64,
    ///
    /// Model representation used for cache calculation.
    shape: Arc<RwLock<DisplacementShape>>,
    /// Cache read from `self.file_path`.
    caches: OnceLock<Vec<(f64, Option<Cache<f64>>)>>,
    thread_pool: Arc<ThreadPool>,
}
//
//
impl DisplacementBoundCache {
    ///
    /// Creates a new instance.
    /// - cache_dir - folder contains all cache files
    pub fn new(
        parent: &Dbg,
        shape: Arc<RwLock<DisplacementShape>>,
        cache_dir: PathBuf,
        level_step: f64,
        center_x: f64,
        thread_pool: Arc<ThreadPool>,
    ) -> Self {
        let dbg = Dbg::new(parent, format!("DisplacementBoundCache_{:.3}", center_x));
        Self {
            shape,
            level_step,
            center_x,
            caches: OnceLock::new(),
            cache_dir,
            dbg,
            thread_pool,
        }
    }
    //
    fn calculate(&mut self, bounds: Bounds, draughts: &[f64]) -> Result<(), Error> {
        let error = Error::new(&self.dbg, "calculate");
        let frames = bounds.frames();
        let centers = bounds.centers();
        let data = self
            .shape
            .read()
            .calculate_strength_bounded(&frames, draughts)
            .map_err(|err| error.pass(err))?;
        let mut caches = Vec::new();
        let cache_path = self.cache_dir.join(format!("{}", bounds.len_qnt()));
        for (i, (dx, v)) in centers.iter().zip(data.iter()).enumerate() {
            let cache = if !v.is_empty() {
                let v: Vec<Vec<f64>> = v.iter().map(|v| vec![v.0, v.1]).collect();
                let cache = Cache::<f64>::new(&self.dbg);
                match cache.init(v.clone()) {
                    Ok(()) => match save(&self.dbg, &cache_path.clone().join(format!("{i}")), v) {
                        Ok(()) => (),
                        Err(err) => {
                            let error = error.pass_with("save cache", err);
                            log::error!("{}", error);
                            return Err(error);
                        }
                    },
                    Err(err) => {
                        let error = error.pass_with("cache.init()", err);
                        log::error!("{}", error);
                        return Err(error);
                    }
                }
                Some(cache)
            } else {
                None
            };
            caches.push((dx - self.center_x, cache));
        }
        if let Err(error) = self.caches.set(caches).map_err(|_| error.err("caches.set")) {
            log::error!("{}", error);
            return Err(error);
        }
        Ok(())
    }
}
