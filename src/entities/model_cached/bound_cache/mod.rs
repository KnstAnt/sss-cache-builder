use crate::entities::{model_cached::bound_cache::build_cache::BuildBoundCache};
use parry3d_f64::shape::TriMesh;
use sal_3dlib_core::math::Bounds;
use sal_core::{dbg::Dbg, error::Error};
use sal_sync::thread_pool::ThreadPool;
use std::{path::PathBuf, sync::Arc};

mod build_cache;
//
pub struct BuildCompartmentBoundCache {
    builder: BuildBoundCache,
}
//
impl BuildCompartmentBoundCache {
    ///
    pub fn new(
        parent: &Dbg,
        mesh: Arc<TriMesh>,
        level_step: f64,
        thread_pool: Arc<ThreadPool>,
    ) -> Self {
        debug_assert!(level_step > 0.);
        Self {
            builder: BuildBoundCache::new(parent, mesh, level_step, true, thread_pool),
        }
    }
    //
    pub fn rebuld_and_save(&self, bounds: &Bounds, dir_path: &PathBuf) -> Result<(), Error> {
        self.builder.rebuld_and_save(bounds, dir_path)
    }    
}
//
pub struct BuildDisplacementBoundCache {
    builder: BuildBoundCache,
}
//
impl BuildDisplacementBoundCache {
    ///
    pub fn new(
        parent: &Dbg,
        mesh: Arc<TriMesh>,
        level_step: f64,
        thread_pool: Arc<ThreadPool>,
    ) -> Self {
        debug_assert!(level_step > 0.);
        Self {
            builder: BuildBoundCache::new(parent, mesh, level_step, false, thread_pool),
        }
    }
    //
    pub fn rebuld_and_save(&self, bounds: &Bounds, dir_path: &PathBuf) -> Result<(), Error> {
        self.builder.rebuld_and_save(bounds, dir_path)
    }     
}
