use sal_core::{dbg::Dbg, error::Error};
use sal_sync::{sync::RwLock, thread_pool::ThreadPool};
use std::{
    path::PathBuf,
    sync::{
        Arc, OnceLock, atomic::{AtomicBool, Ordering}
    },
};

use crate::entities::{Bounds, cache::Cache, model_cached::{DisplacementShape, read, save}};
///
/// Pre-calculated cache for bounds
pub struct DisplacementBoundCache {
    dbg: Dbg,
    cache_path: PathBuf,
    level_step: f64,
    center_x: f64,
    frames: Vec<f64>,
    ///
    /// Model representation used for cache calculation.
    shape: Arc<RwLock<DisplacementShape>>,
    /// Cache read from `self.file_path`.
    caches: OnceLock<Vec<(f64, Option<Cache<f64>>)>>,
    thread_pool: Arc<ThreadPool>,
    exit: Arc<AtomicBool>,
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
        frames: Vec<f64>,
        thread_pool: Arc<ThreadPool>,
    ) -> Self {
        let dbg = Dbg::new(parent, format!("DisplacementBoundCache_{:.3}", center_x));
        let cache_path = cache_dir.join(format!("{}", frames.len() - 1));
        Self {
            shape,
            level_step,
            center_x,
            frames,
            caches: OnceLock::new(),
            cache_path,
            dbg,
            thread_pool,
            exit: Arc::new(AtomicBool::new(false)),
        }
    }
    /// Rebuilds a cache
    /// - takes new model
    /// - do calculations
    /// - stores calculated table
    /// - loads recalculated table
    pub fn rebuild(&mut self) -> Result<(), Error> {
        self.clear_exit();
        let errors = self.calculate();
        if errors.is_empty() {
            return Ok(());
        }
        let full_error = errors.into_iter().fold("".to_owned(), |acc, err| acc + ", " + &err.to_string());
        Err(Error::new(self.dbg.clone(), format!("rebuild: {full_error}")))
    }
    //
    fn calculate(&mut self) -> Vec<Error> {
        let error = Error::new(&self.dbg, "calculate");

        self.bounds.
        self.calculate_strength_bounded(frames: &[f64], draughts: &[f64]);



        let (data, mut errors) = super::build_cache::BuildDisplacementBoundCache::new(
            &self.dbg,
            self.shape.clone(),
            self.level_step,
            self.bounds.clone(),
            Arc::clone(&self.thread_pool),
            self.exit.clone(),
        )
        .build();
        let mut caches = Vec::new();
        for (i, (dx, v)) in data.into_iter().enumerate() {
            if self.exit.load(Ordering::Relaxed) {
                errors.push(error.err("exit"));
                return errors;
            }
            let cache = if let Some(v) = v {
                let v: Vec<Vec<f64>> = v.iter().map(|v| vec![v.0, v.1]).collect();
                let cache = Cache::<f64>::new(&self.dbg);
                match cache.init(v.clone()) {
                    Ok(()) => {
                        match save(&self.dbg, &self.cache_path.clone().join(format!("{i}")), v) {
                            Ok(()) => (),
                            Err(err) => {
                                let error = error.pass_with("save cache", err);
                                log::error!("{}", error);
                                errors.push(error);
                                return errors;
                            }
                        }
                    }
                    Err(err) => {
                        let error = error.pass_with("cache.init()", err);
                        log::error!("{}", error);
                        errors.push(error);
                        return errors;
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
            errors.push(error);
        }
        errors
    }
    //
    fn exit(&self) {
        self.exit.store(true, Ordering::SeqCst)
    }
    //
    fn clear_exit(&self) {
        self.exit.store(false, Ordering::SeqCst)
    }
}
