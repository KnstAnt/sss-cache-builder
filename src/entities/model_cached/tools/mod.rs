mod file_io;
mod hydrostatic;
mod strength;
mod windage;

pub use file_io::*;
pub use hydrostatic::*;
use parry3d_f64::math::Vec3;
use parry3d_f64::shape::TriMesh;
pub use strength::*;
pub use windage::*;

use crate::entities::Position;

/// полный объем модели
pub fn properties(mesh: &TriMesh, density: f64) -> (f64, Position) {
    let properties = parry3d_f64::shape::Shape::mass_properties(mesh, density);
    let mass = if properties.inv_mass > 0. { 1. / properties.inv_mass } else { 0. };
    (
        mass,
        Position::new(
            properties.local_com.x,
            -properties.local_com.y,
            properties.local_com.z,
        ),
    )
}
///
/// Расчет начала координат для отсеков как
/// проекции центра объема модели на ее нижнюю плоскость
pub(crate) fn compartment_center(mesh: &TriMesh) -> Vec3 {
    let properties = parry3d_f64::shape::Shape::mass_properties(mesh, 1.);
    let aabb = mesh.local_aabb();
    Vec3::new(properties.local_com.x, properties.local_com.y, aabb.mins.z)
}

/// Разбиение меша по высоте на draught_qnt_steps шагов.
/// Макимальный и минимальный уровень считаются с учетом наклона
pub fn draught_steps(
    mesh: &TriMesh,
    center: Vec3,
    level_step_qnt: usize,
    max_heel: f64,
    max_trim: f64,
) -> (f64, Vec<f64>) {
    assert!(level_step_qnt > 2);
    let aabb = mesh.local_aabb();
    assert!(aabb.maxs.x >= center.x);
    assert!(aabb.mins.x <= center.x);
    assert!(aabb.maxs.y >= center.y);
    assert!(aabb.mins.y <= center.y);
    let mut result = vec![];
    let max_dx = (aabb.maxs.x - center.x).max(center.x - aabb.mins.x);
    let max_dy = (aabb.maxs.y - center.y).max(center.y - aabb.mins.y);
    let max_dz = max_dx * max_trim.to_radians().tan().abs()
        + max_dy * max_heel.to_radians().tan().abs() * max_trim.to_radians().cos();
    let delta_z = aabb.maxs.z - aabb.mins.z;
    let min_z = -max_dz;
    let max_z = delta_z + max_dz;
    let step = delta_z / (level_step_qnt as f64 - 1.);
    let step_max_dz = if max_dz * 2. > delta_z {
        2. * max_dz / (level_step_qnt as f64 - 1.)
    } else {
        step
    };
    let mut current = min_z;
    let mut current_step = step_max_dz;
    while current + current_step / 2. <= 0. {
        result.push(current);
        current += current_step;
    }
    current = 0.;
    current_step = step;
    while current + current_step / 2. <= delta_z {
        result.push(current);
        current += current_step;
    }
    current = delta_z;
    current_step = step_max_dz;
    while current + current_step / 2. <= max_z {
        result.push(current);
        current += current_step;
    }
    result.push(max_z);
    //  log::debug!("shape draught_steps max_dx:{} max_dy:{} max_dz:{} delta_z:{} step_max_dz:{} min_z:{} max_z:{} aabb.mins.z:{} aabb.maxs.z:{} level_step_qnt:{}",
    //      max_dx, max_dy, max_dz, delta_z, step_max_dz, min_z, max_z, aabb.mins.z, aabb.maxs.z, level_step_qnt);
    //  log::debug!("shape draught_steps {:?}", result);
    (aabb.mins.z, result)
}
