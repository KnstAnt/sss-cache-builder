//!
//! Defines a shape trait
mod area_shape;
mod displacement_shape;
mod utils;
mod hydrostatic;
mod strength;

pub(crate) use area_shape::*;
pub(crate) use displacement_shape::*;
pub(crate) use utils::*;

use parry3d_f64::bounding_volume::Aabb;
use parry3d_f64::shape::TriMesh;
use parry3d_f64::math::{Pose3, Vec3, Rot3};

use sal_core::dbg::Dbg;
use sal_core::error::Error;

pub trait Shape {
    //
    fn dbg(&self) -> &Dbg;
    //
    fn mesh(&self) -> Option<&TriMesh>;
    //
    fn center(&self) -> Option<&Vec3>;
    /// Init shape, load geometry
    fn init(&mut self) -> Result<(), Error>;
    ///
    /// Расчет положения корпуса
    fn position(&self, heel: f64, trim: f64, draught: f64) -> Result<Pose3, Error> {
        let center = self
            .center()
            .ok_or(Error::new(self.dbg(), "position").err("no center"))?;
        Ok(position(center, heel, trim, draught))
    }

    /// Разбиение меша по высоте на draught_qnt_steps шагов.
    /// Макимальный и минимальный уровень считаются с учетом наклона
    fn draught_steps(&self, level_step_qnt: usize, max_heel: f64, max_trim: f64) -> Result<Vec<f64>, Error> {
        assert!(level_step_qnt > 2);
        let error = Error::new(self.dbg(), "draught_steps");
        let mesh = self.mesh().ok_or(error.err("no mesh"))?;
        let aabb = mesh.local_aabb();
        let center = if let Some(center) = self.center() {
            center
        } else {
            &compartment_center(mesh)
        };
        assert!(aabb.maxs.x >= center.x);
        assert!(aabb.mins.x <= center.x);
        assert!(aabb.maxs.y >= center.y);
        assert!(aabb.mins.y <= center.y);   
        let mut result = vec![]; 
        let max_dx = (aabb.maxs.x - center.x).max(center.x - aabb.mins.x);
        let max_dy = (aabb.maxs.y - center.y).max(center.y - aabb.mins.y);
        let max_dz = max_dx*max_trim.to_radians().tan().abs() + max_dy*max_heel.to_radians().tan().abs()*max_trim.to_radians().cos();
        let delta_z = aabb.maxs.z - aabb.mins.z;
        let min_z = -max_dz;
        let max_z = delta_z + max_dz;
        let step = delta_z/(level_step_qnt as f64 - 1.);    
        let step_max_dz = if max_dz*2. > delta_z {
            2.*max_dz/(level_step_qnt as f64 - 1.)
        } else {
            step
        };  
        let mut current = min_z;
        let mut current_step = step_max_dz;
        while current + current_step/2. <= 0. {
            result.push(current);
            current += current_step;
        }
        current = 0.;
        current_step = step;
        while current + current_step/2. <= delta_z {
            result.push(current);
            current += current_step;
        }
        current = delta_z;
        current_step = step_max_dz;
        while current + current_step/2. <= max_z {
            result.push(current);
            current += current_step;
        }
        result.push(max_z);
      //  log::debug!("shape draught_steps max_dx:{} max_dy:{} max_dz:{} delta_z:{} step_max_dz:{} min_z:{} max_z:{} aabb.mins.z:{} aabb.maxs.z:{} level_step_qnt:{}", 
      //      max_dx, max_dy, max_dz, delta_z, step_max_dz, min_z, max_z, aabb.mins.z, aabb.maxs.z, level_step_qnt);
      //  log::debug!("shape draught_steps {:?}", result);
        Ok(result)
    }
  /*  fn draught_steps(&self, draught_qnt_steps: usize) -> Result<Vec<f64>, Error> {
        let error = Error::new(self.dbg(), "draught_steps");
        let aabb = self.mesh().ok_or(error.err("no mesh"))?.local_aabb();
        if draught_qnt_steps <= 1 {
            return Err(error.err("draught_qnt_steps <= 1"));
        }
        if aabb.mins.z >= aabb.maxs.z {
            return Err(error.err("aabb.mins.z >= aabb.maxs.z"));
        }
        let mut result = vec![];
        let mut current = aabb.mins.z;
        let step = (aabb.maxs.z - aabb.mins.z)/(draught_qnt_steps as f64 - 1.);
        while current < aabb.maxs.z {
            result.push(current);
            current += step;
        }
        result.push(aabb.maxs.z);
        Ok(result)
    }*/
}
///
/// Расчет начала координат для отсеков как
/// проекции центра объема модели на ее нижнюю плоскость
pub(crate) fn compartment_center(mesh: &TriMesh) -> Vec3 {
    let properties = parry3d_f64::shape::Shape::mass_properties(mesh, 1.);
    let aabb: Aabb = mesh.local_aabb();
    Vec3::new(properties.local_com.x, properties.local_com.y, aabb.mins.z)
}

pub fn position(center: &Vec3, heel: f64, trim: f64, draught: f64) -> Pose3 {
    let heel_rad = -heel.to_radians();
    let trim_rad = trim.to_radians();

    // 1. Вращение по дифференту (trim) вокруг оси Y
    let trim_rotation = Rot3::from_axis_angle(Vec3::Y, trim_rad);

    // 2. Находим трансформированную ось X для крена (heel)
    let transformed_x_axis = trim_rotation * Vec3::X;
    
    // 3. Вращение по крену вокруг новой оси X
    let heel_rotation = Rot3::from_axis_angle(transformed_x_axis.normalize(), heel_rad);
    
    // Итоговое вращение
    let rotation = heel_rotation * trim_rotation;

    // 4. Смещение центра
    let mut center_offset = *center;
    center_offset.z += draught;

    // 5. Вычисляем позицию (в glam вращение точки делается через оператор *)
    let point = rotation * center_offset;

    // 6. Создаем Isometry (в Parry с фичей glam это структура с полями translation и rotation)
    Pose3::from_parts(-point, rotation)
}

pub fn normal(heel: f64, trim: f64) -> Vec3 {
    let heel_rad = heel.to_radians();
    let trim_rad = -trim.to_radians();
    
    // 1. Вращение по дифференту вокруг оси Y
    let trim_rotation = Rot3::from_axis_angle(Vec3::Y, trim_rad);
    
    // 2. Получаем трансформированную ось X
    let transformed_x_axis = trim_rotation * Vec3::X;
    
    // 3. Вращение по крену вокруг новой оси X
    let heel_rotation = Rot3::from_axis_angle(transformed_x_axis.normalize(), heel_rad);
    
    // 4. Итоговое вращение
    let rotation = heel_rotation * trim_rotation;
    
    // 5. Трансформируем вектор нормали (Z-up)
    rotation * Vec3::Z
}
