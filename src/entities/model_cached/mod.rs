//!
//! The representation of the ship in terms of its 3D elements.
//
mod model_cached_conf;
mod model_cached;
mod windage_area;
mod draught;
pub mod displacement_cache;
//pub mod displacement_bound_cache;
pub mod compartment_cache;
//pub mod compartment_bound_cache;
pub mod bound_cache;

pub use model_cached_conf::*;
pub(crate) use model_cached::*;
pub(crate) use windage_area::*;






