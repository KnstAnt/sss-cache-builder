use std::{path::PathBuf, sync::Arc};
use debugging::session::debug_session::{DebugSession, LogLevel};
use log::info;
use sal_3dlib_core::math::Bounds;
use sal_core::dbg::Dbg;
use sal_sync::thread_pool::ThreadPool;
use crate::{conf::conf::Conf, entities::{model_cached::{ModelCached, ModelCachedConf}}};
mod entities;
mod conf;

fn main() {
    DebugSession::new()
        .filter(LogLevel::Trace)
        .module("sal_sync", LogLevel::Error)
        .module("ena", LogLevel::Error)
        .init();
    let dbg = Dbg::own("main");
    info!("starting up");
    let conf = "./config.yaml";
    let conf = Conf::new(&dbg, conf);
    let bounds = Bounds::from_array(&conf.model.frames, 0.).unwrap();
    let cache_dir: PathBuf = ("assets/cache/".to_owned() + &conf.model.name).into();
    let model_dir: PathBuf = ("assets/model/".to_owned() + &conf.model.name).into();
    let thread_pool = Arc::new(ThreadPool::new(&dbg, Some(conf.thread_pool.size)));
    let model_cached = ModelCached::new(
        &dbg,
        ModelCachedConf {
            model_dir,
            cache_dir,
            model_scale: conf.model.scale,
            midel_x: conf.model.midel_x,
            hull_heel_steps: conf.model.hull_heel_steps,
            hull_trim_steps: conf.model.hull_trim_steps,
            compartment_heel_steps: conf.model.compartment_heel_steps,
            compartment_trim_steps: conf.model.compartment_trim_steps,
            ship_length_lbp: conf.model.ship_length_lbp,
            draught_min: conf.model.draught_min,
            hull_draught_min: conf.model.hull_draught_min,
            hull_draught_max: conf.model.hull_draught_max,
            hull_draught_step: conf.model.hull_draught_step,
            bounds_level_step: conf.model.bounds_level_step,
            compartment_level_step_qnt: conf.model.compartment_level_step_qnt,
        },
        bounds,
        Arc::clone(&thread_pool),
    )
    .unwrap();
    model_cached.rebuld().unwrap();
}
