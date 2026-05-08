use parry3d_f64::math::Pose;
use parry3d_f64::shape::TriMesh;
use sal_core::{dbg::Dbg, error::Error};
use sal_sync::{
    sync::Stack,
    thread_pool::{JoinHandle, ThreadPool},
};
use std::{collections::VecDeque, path::PathBuf, sync::Arc};

use crate::entities::{
    Bounds,
    model_cached::{HullSlicer, create_dir, remove, save},
};

///
/// Provides logic to calculate and store cache used by [super::DisplacementBoundCache].
pub struct BuildBoundCache {
    dbg: Dbg,
    mesh: Arc<TriMesh>,
    level_step: f64,
    use_z_shift: bool,
    thread_pool: Arc<ThreadPool>,
}
//
//
impl BuildBoundCache {
    ///
    /// Crates a new instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        parent: &Dbg,
        mesh: Arc<TriMesh>,
        level_step: f64,
        use_z_shift: bool,
        thread_pool: Arc<ThreadPool>,
    ) -> Self {
        debug_assert!(level_step > 0.);
        Self {
            dbg: Dbg::new(parent, "BuildBoundCache"),
            mesh,
            level_step,
            use_z_shift,
            thread_pool,
        }
    }
    /// Построение кэшей
    fn build(&self, bounds: &Bounds) -> (Vec<(f64, Vec<(f64, f64)>)>, Vec<Error>) {
        log::info!("{}.build | Starting build", &self.dbg);
        let (results, errors) = self._build(bounds);
        let mut vec_results = Vec::new();
        let shift_z = if self.use_z_shift {
            self.mesh.aabb(&Pose::identity()).mins.z
        } else {
            0.
        };
        while !results.is_empty() {
            if let Some((dx, mut result)) = results.pop() {
                if shift_z != 0. {
                    result.iter_mut().for_each(|(z, v)| *z -= shift_z);
                }
                vec_results.push((dx, result));
            }
        }
        let mut vec_errors = Vec::new();
        while !errors.is_empty() {
            if let Some(error) = errors.pop() {
                vec_errors.push(error);
            }
        }
        vec_results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        (vec_results, vec_errors)
    }
    //
    pub fn _build(
        &self,
        bounds: &Bounds,
    ) -> (Arc<Stack<(f64, Vec<(f64, f64)>)>>, Arc<Stack<Error>>) {
        log::info!("{}._build | Starting _build", &self.dbg);
        let error = Error::new(&self.dbg, "_build");
        let mut tasks: VecDeque<JoinHandle<_>> = VecDeque::new();
        let results = Arc::new(Stack::new());
        let errors = Arc::new(Stack::new());
        let pass = |message: &str, err: Error| {
            let error = error.pass_with(message, err);
            log::error!("{:?}", &error);
            errors.push(error);
        };
        let hull = HullSlicer::new(Arc::clone(&self.mesh));
        let frames = bounds.frames();
        let slices = hull.slice(&frames);
        let scheduler = self.thread_pool.scheduler();
        for slice in slices.into_iter() {
            let results = results.clone();
            let _errors = errors.clone();
            let _error = error.clone();
            let center_x = (slice.x_start + slice.x_end) / 2.;
            let level_step = self.level_step;
            let thread_name = format!(
                "BuildBoundCache displacement_by_steps slice_{:.3}",
                center_x
            );
            log::info!("{}.build | Starting thread {thread_name}", &self.dbg);
            //  println!("{}.build | Starting thread {thread_name}", &self.dbg);
            let handle = scheduler
                .spawn_named(thread_name, move || {
                    results.push((center_x, slice.calculate_displacements_by_steps(level_step)));
                    Ok(())
                })
                .map_err(|err| {
                    error.pass_with(
                        format!("spawn task center_x:{:?}", center_x),
                        err.to_string(),
                    )
                });
            match handle {
                Ok(task) => tasks.push_back(task),
                Err(err) => pass("task handle", err),
            };
        }
        for task in tasks {
            log::trace!("join thread {}", task.name());
            if let Err(err) = task.join() {
                pass("task join", err);
            }
        }
        (results, errors)
    }
    //
    pub fn rebuld_and_save(&self, bounds: &Bounds, dir_path: &PathBuf) -> Result<(), Error> {
        let error = Error::new(&self.dbg, "rebuld_and_save");
        let (result, errors) = self.build(bounds);
        if !errors.is_empty() {
            return Err(error.pass(errors.iter().fold(String::new(), |acc, err| {
                format!("{acc}\n\tIn error: {err}")
            })));
        }
        let dir_path = dir_path.join(format!("{}", bounds.len_qnt()));
        create_dir(&self.dbg, &dir_path)?;
        for (i, (_, v)) in result.iter().enumerate() {
            let path = dir_path.join(format!("{i}"));
            if v.is_empty() {
                remove(&self.dbg, &path);
            } else if let Err(err) = save(&self.dbg, &path, v.iter().map(|v| vec![v.0, v.1]).collect()) {
                return Err(error.pass_with("save data", err));
            }
        }
        Ok(())
    }
}
