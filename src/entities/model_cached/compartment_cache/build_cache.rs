use parry3d_f64::shape::TriMesh;
use sal_core::{dbg::Dbg, error::Error};
use sal_sync::{
    sync::Stack,
    thread_pool::{JoinHandle, ThreadPool},
};
use std::{
    collections::VecDeque,
    sync::Arc,
};

use crate::entities::{Position, model_cached::{calculate_hydrostatic, calculate_inertia, compartment_center, draught_steps, properties}};

///
/// Provides logic to calculate and store cache used by [super::CompartmentCache].
pub struct BuildCompartmentCache {
    dbg: Dbg,
    mesh: Arc<TriMesh>,
    heel_steps: Vec<f64>,
    trim_steps: Vec<f64>,
    level_step_qnt: usize,
    thread_pool: Arc<ThreadPool>,
}
//
//
impl BuildCompartmentCache {
    ///
    /// Crates a new instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        parent: &Dbg,
        mesh: Arc<TriMesh>,
        heel_steps: Vec<f64>,
        trim_steps: Vec<f64>,
        level_step_qnt: usize,
        thread_pool: Arc<ThreadPool>,
    ) -> Self {
        Self {
            dbg: Dbg::new(parent, "BuildCompartmentCache"),
            mesh,
            heel_steps,
            trim_steps,
            level_step_qnt,
            thread_pool,
        }
    }
    ///
    /// Creates and starts worker for [CompartmentCache::calculate].
    ///
    /// results: [[heel, trim, draught, volume, vx, vy, vz, ix, iy, max_moment, max_volume]]
    pub fn build(&self) -> (Vec<Vec<f64>>, Vec<Error>) {
        //  dbg!("BuildCompartmentCache build begin");
        log::info!("{}.build | Starting build", &self.dbg);
        let mut tasks: VecDeque<JoinHandle<_>> = VecDeque::new();
        let error = Error::new(&self.dbg, "build");
        let results = Arc::new(Stack::new());
        let mut errors = Vec::new();
        let mut pass = |message: &str, err: Error| {
            let error = error.pass_with(message, err);
            log::error!("{:?}", &error);
            errors.push(error);
        };
        let mesh = Arc::clone(&self.mesh);
        let center = compartment_center(&mesh);
        let (volume_max, center_max) = properties(&mesh, 1.);
        let max_heel = self
            .heel_steps
            .iter()
            .fold(0., |acc, v| if acc < v.abs() { v.abs() } else { acc });
        let max_trim = self
            .trim_steps
            .iter()
            .fold(0., |acc, v| if acc < v.abs() { v.abs() } else { acc });
        let (draught_zero, draught_steps) = draught_steps(&mesh, center, self.level_step_qnt, max_heel, max_trim);
        if draught_steps.len() < 2 {
            return (vec![], vec![error.err("draught_steps.len < 2")]);
        }
        let scheduler = self.thread_pool.scheduler();
        assert!(self.heel_steps.len() > 1);
        assert!(self.trim_steps.len() > 1);
        assert!(self.heel_steps.contains(&0.));
        assert!(self.trim_steps.contains(&0.));
        assert!(draught_steps.len() > 1);
        for draught in draught_steps {
            for &heel in &self.heel_steps {
                for &trim in &self.trim_steps {
                    let results = results.clone();
                    let mesh = Arc::clone(&mesh);
                    let center = center.clone();
                    let thread_name =
                        format!("BuildCompartmentCache displacement {heel} {trim} {draught}");
                    log::info!("{}.build | Starting thread {thread_name}", &self.dbg);
                 //   println!("{}.build | Starting thread {thread_name}", &self.dbg);
                    let handle = scheduler
                        .spawn_named(thread_name, move || {
                            results.push((
                                heel,
                                trim,
                                draught,
                                calculate_hydrostatic(&mesh, center, heel, trim, draught),                              
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
        let mut vec_results = Vec::new();
        while !results.is_empty() {
            if let Some((heel, trim, draught, (volume, center), inertia)) = results.pop() {
                let (i_x, i_y) = if volume < volume_max {
                    inertia
                } else {
                    (0., 0.)
                };
                vec_results.push(vec![
                    heel,
                    trim,
                    draught,
                    volume,
                    center.x,
                    center.y,
                    center.z,
                    i_x,
                    i_y,
                    0., // i_x max
                    0., // abs_moment
                ]);
            }
        }
        // для пустого объема значения заполняем руками для нормальной интерполяции
        {
            // берем значения с ненулевым объемом
            let mut tmp: Vec<_> = vec_results.iter().filter(|v| v[3] > 0.).collect();
            // и сортируем чтобы найти значение с минимальными креном/дифферентом и объемом
            tmp.sort_by(|a, b| {
                (a[0].abs() * a[3] + a[1].abs() * a[3])
                    .partial_cmp(&(b[0].abs() * b[3] + b[1].abs() * b[3]))
                    .unwrap()
            });
            let center_min = if let Some(first) = tmp.first() {
                Position::new(first[4], first[5], draught_zero)
            } else {
                Position::new(center_max.x(), center_max.y(), draught_zero)
            };
            // для пустого объема значения меняем значения
            vec_results.iter_mut().filter(|v| v[3] == 0.).for_each(|v| {
                v[4] = center_min.x();
                v[5] = center_min.y();
                v[6] = center_min.z();
                v[7] = 0.;
                v[8] = 0.;
                v[9] = 0.;
                v[10] = 0.;
            });
        }
        for &heel in &self.heel_steps {
            let sin_theta = heel.to_radians().sin();
            let cos_theta = heel.to_radians().cos();
            let mut current_vec: Vec<_> = vec_results
                .iter_mut()
                .filter(|v| v[0] == heel)
                .collect::<Vec<_>>();
           let max_inertia_trans_x = current_vec
                .iter()
                .map(|v| v[7])
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap(); 
            current_vec.iter_mut().for_each(|v| {
                    v[9] = max_inertia_trans_x;
                    v[10] = (v[5] * cos_theta + v[6] * sin_theta) * v[3]; // абсолютный момент жидкости            
                }
            );
        }
        (vec_results, errors)
    }
}
