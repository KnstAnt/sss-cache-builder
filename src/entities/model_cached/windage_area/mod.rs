mod file_io;
use parry3d_f64::shape::TriMesh;
use sal_core::dbg::Dbg;
use std::{sync::Arc};
use crate::entities::model_cached::*;

///
///
/// Площадь парусности корпуса и конструкций
pub struct WindageArea {
    dbg: Dbg,
    mesh: Arc<TriMesh>,
    midel_x: f64,
    draught_min: f64,
    lbp: f64,
    resolution: u32,
}
//
//
impl WindageArea {
    ///
    /// Creates a new instance.
    /// - cache_path - the folder contains all caches
    ///
    pub fn new(
        parent: &Dbg,
        mesh: Arc<TriMesh>,
        midel_x: f64,
        draught_min: f64,
        lbp: f64,
        resolution: u32,
    ) -> Self {
        let dbg = Dbg::new(parent, "WindageArea");
        Self {
            dbg,
            mesh, 
            midel_x, 
            draught_min,
            lbp, 
            resolution,
          }
    }
    /// пересчет для заданных значений
    pub fn build(&self) -> WindageProfile {
        WindageProfile::new(
            Arc::clone(&self.mesh), 
            self.midel_x, 
            self.draught_min,
            self.lbp, 
            self.resolution
        )
    }
}
