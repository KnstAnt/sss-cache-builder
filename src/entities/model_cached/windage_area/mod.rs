use parry3d_f64::shape::TriMesh;
use sal_3dlib::WindageProfile;
use sal_3dlib_core::file_io::create_dir;
use sal_core::{dbg::Dbg, error::Error};
use std::{fs::File, path::PathBuf, sync::Arc};
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
    fn build(&self) -> WindageProfile {
        WindageProfile::new(
            Arc::clone(&self.mesh), 
            self.midel_x, 
            self.draught_min,
            self.lbp, 
            self.resolution
        )
      }
    //
    pub fn rebuld_and_save(&self, dir_path: &PathBuf) -> Result<(), Error> {
        let error = Error::new(&self.dbg, "rebuld_and_save");
        let result = self.build();
        create_dir(&self.dbg, &dir_path)?;
        result.save(&dir_path.join("windage")).map_err(|err| error.pass(err))
    }  
}

