use parry3d_f64::math::Vec3;
use parry3d_f64::shape::TriMesh;
use sal_3dlib::*;
use sal_3dlib_core::{cache::file_io::save, file_io::create_dir};
use sal_core::{dbg::Dbg, error::Error};
use sal_sync::{
    sync::Stack,
    thread_pool::{JoinHandle, ThreadPool},
};
use std::{
    collections::VecDeque, path::PathBuf, sync::Arc
};

///
/// Provides logic to calculate and store cache used by [super::DisplacementCache].
///
pub struct BuildDisplacementCache {
    dbg: Dbg,
    mesh: Arc<TriMesh>,
    midel_x: f64,
    heel_steps: Vec<f64>,
    trim_steps: Vec<f64>,
    draught_min: f64,
    draught_max: f64,
    draught_step: f64,
    thread_pool: Arc<ThreadPool>,
}
//
//
impl BuildDisplacementCache {
    ///
    /// Crates a new instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        parent: &Dbg,
        mesh: Arc<TriMesh>,
        midel_x: f64,
        heel_steps: Vec<f64>,
        trim_steps: Vec<f64>,
        draught_min: f64,
        draught_max: f64,
        draught_step: f64,
        thread_pool: Arc<ThreadPool>,
    ) -> Self {
        debug_assert!(draught_min < draught_max);
        debug_assert!(draught_step > 0.);
        Self {
            dbg: Dbg::new(parent, "BuildDisplacementCache"),
            mesh,
            midel_x,
            heel_steps,
            trim_steps,
            draught_min,
            draught_max,
            draught_step,
            thread_pool,
        }
    }
    ///
    /// Creates and starts worker for [DisplacementCache::calculate].
    /// results: [[heel, trim, draught, volume, vx, vy, vz, area, ax, ay, az, ix, iy, wx, wy]]
    fn build(&self) -> (Vec<Vec<f64>>, Vec<Error>) {
        log::info!("{}.build | Starting build", &self.dbg);
        let error = Error::new(&self.dbg, "build");
        let mut tasks: VecDeque<JoinHandle<_>> = VecDeque::new();
        let aabb_results = Arc::new(Stack::new());
        let draft_results = Arc::new(Stack::new());
        let mut errors = Vec::new();
        let mut pass = |message: &str, err: Error| {
            let error = error.pass_with(message, err);
            log::error!("{:?}", &error);
            errors.push(error);
        };
        let mesh = Arc::clone(&self.mesh);
        let mut draught_steps = Vec::new();
        let mut draught = self.draught_min;
        loop {
            draught_steps.push(draught);
            if draught >= self.draught_max {
                break;
            }
            draught += self.draught_step;
        }
        let scheduler = self.thread_pool.scheduler();
        for draught in draught_steps {
            {
                let aabb_results = aabb_results.clone();
                let mesh = Arc::clone(&mesh);
                let thread_name =
                    format!("{}.build aabb {draught}", &self.dbg);
                log::info!("thread_name Starting thread");
                let handle = scheduler
                    .spawn_named(thread_name, move || {
                        aabb_results.push((draught, calculate_waterline_size(&mesh, draught)));
                        Ok(())
                    })
                    .map_err(|err| {
                        error.pass_with(
                            format!("spawn task aabb draught:{draught}"),
                            err.to_string(),
                        )
                    });
                match handle {
                    Ok(task) => tasks.push_back(task),
                    Err(err) => pass("task handle", err),
                };
            }
            for &heel in &self.heel_steps {
                for &trim in &self.trim_steps {
                    let draft_results = draft_results.clone();
                    let mesh = Arc::clone(&mesh);
                    let dx = self.midel_x;
                    let thread_name =
                        format!("BuildDisplacementCache displacement {draught} {heel} {trim}");
                    log::info!("{}.build | Starting thread {thread_name}", &self.dbg);
                //    println!("Starting thread {thread_name}");
                    let handle = scheduler
                        .spawn_named(thread_name, move || {
                            let center = Vec3::new(dx, 0., 0.);
                            draft_results.push((
                                heel,
                                trim,
                                draught,
                                calculate_hydrostatic(&mesh, center, heel, trim, draught),                              
                                calculate_waterline(&mesh, center, heel, trim, draught),
                                calculate_inertia(&mesh, center, heel, trim, draught),
                            ));
                            Ok(())
                        })
                        .map_err(|err| {
                            error.pass_with(
                                format!(
                                    "spawn task draught:{} heel:{} trim:{}",
                                    draught, heel, trim
                                ),
                                err.to_string(),
                            )
                        });
                    match handle {
                        Ok(task) => tasks.push_back(task),
                        Err(err) => pass("task handle", err),
                    };
                }
            }
        }
        for task in tasks {
            log::trace!("join thread {}", task.name());
            if let Err(err) = task.join() {
                pass("task join", err);
            }
        }
        let mut vec_aabb = Vec::new();
        while !aabb_results.is_empty() {
            if let Some((draught, data)) = aabb_results.pop() {
                vec_aabb.push((draught, data))
            }
        }
        let mut vec_results = Vec::new();
        while !draft_results.is_empty() {
            if let Some((heel, trim, draught, (volume, v_center), (area, a_center), (i_x, i_y))) = draft_results.pop() {
                if let Some((_, (l_x, l_y))) = vec_aabb.iter().find(|(wl_d, _)| *wl_d == draught) {
                    vec_results.push(vec![
                        heel,
                        trim,
                        draught,
                        volume,
                        v_center.x,
                        v_center.y,
                        v_center.z,
                        area,
                        a_center.x,
                        a_center.y,
                        a_center.z,
                        i_x,
                        i_y,
                        *l_x,
                        *l_y,
                    ]);
                } else {
                    pass(
                        "draft_results",
                        error.err(format!("no aabb for draught:{draught}")),
                    );
                }
            }
        }
        (vec_results, errors)
    }
    //
    pub fn rebuld_and_save(&self, dir_path: &PathBuf) -> Result<(), Error> {
        let error = Error::new(&self.dbg, "rebuld_and_save");
        let (result, errors) = self.build();
        create_dir(&self.dbg, &dir_path)?;
        if !errors.is_empty() {
            return Err(error.pass(
                errors.iter().fold(String::new(), |acc, err| {
                    format!("{acc}\n\tIn error: {err}")
                }),
            ));
        } else if let Err(err) = save(
            &self.dbg,
            &dir_path.join("displacement_cache"),
            result,
        ) {
            return Err(error.pass_with("save data", err));
        }   
        Ok(()) 
    }
}
