use parry3d_f64::math::Vec3;

pub struct SlicedMesh {
    /// Треугольники, оказавшиеся под плоскостью (полезный объем)
    pub submerged_triangles: Vec<[Vec3; 3]>,
    /// Отрезки, формирующие контур сечения (ватерлинию)
    pub waterline_edges: Vec<[Vec3; 2]>,
}
impl SlicedMesh {
    ///
    /// Для замкнутого меша объем считается как сумма ориентированных объемов тетраэдров.
    /// Требует передачи плоскости `Plane` для закрытия "крышки" разреза.
    pub fn volume(&self, plane: &super::Plane) -> f64 {
        let mut total_volume = 0.0;
        let mut add_tetrahedron = |p1: &Vec3, p2: &Vec3, p3: &Vec3| {
            total_volume += p1.dot(p2.cross(*p3)) / 6.0;
        };
        // 1. Интегрируем погруженные треугольники корпуса
        for [p1, p2, p3] in &self.submerged_triangles {
            add_tetrahedron(p1, p2, p3);
        }
        // 2. ЗАКРЫВАЕМ "КРЫШКУ" (Без этого объем будет неверным!)
        if let Some(first_edge) = self.waterline_edges.first() {
            let p_ref = first_edge[0]; // Центральная точка для веера
            for [a, b] in &self.waterline_edges {
                let cross = (a - p_ref).cross(b - p_ref);
                let is_pointing_out = cross.dot(plane.normal) > 0.0;
                // Сохраняем правильное направление нормали (из воды)
                if is_pointing_out {
                    add_tetrahedron(&p_ref, a, b);
                } else {
                    add_tetrahedron(&p_ref, b, a);
                }
            }
        }
        total_volume.abs()
    }
 //
    pub fn hydrostatics(&self, plane: &super::Plane) -> super::Hydrostatics {
        let mut total_volume = 0.0;
        let mut sum_centroid = Vec3::ZERO;

        // 1. Находим точку на плоскости, которая будет вершиной всех тетраэдров.
        // Это гарантирует, что "крышка" (ватерлиния) имеет нулевой объем
        // и не влияет на итоговую сумму.
        // plane.d в вашей реализации — это normal.dot(point).
        let p_ref = plane.normal * plane.d;

        // 2. Интегрируем только погруженные треугольники корпуса
        for tri in &self.submerged_triangles {
            let p0 = tri[0];
            let p1 = tri[1];
            let p2 = tri[2];

            // Векторы сторон тетраэдра относительно точки на плоскости воды
            let a = p0 - p_ref;
            let b = p1 - p_ref;
            let c = p2 - p_ref;

            // Знаковый объем тетраэдра (смешанное произведение)
            // 1/6 * |(a × b) · c|
            let v_i = a.dot(b.cross(c)) / 6.0;

            total_volume += v_i;

            // Центроид тетраэдра: (p0 + p1 + p2 + p_ref) / 4
            // Взвешиваем центроид объемом тетраэдра
            let centroid_i = (p0 + p1 + p2 + p_ref) * 0.25;
            sum_centroid += centroid_i * v_i;
        }

        // 3. Финальные расчеты. 
        // Если нормаль плоскости смотрит "вверх", объем погруженной части 
        // корпуса (нормали которого "наружу") будет отрицательным. 
        // Это нормально, берем модуль.
        let abs_volume = total_volume.abs();

        let center_of_buoyancy = if abs_volume > f64::EPSILON {
            // Делим на знаковый объем, чтобы сохранить правильную ориентацию центра
            sum_centroid / total_volume
        } else {
            Vec3::ZERO
        };
        super::Hydrostatics {
            volume: abs_volume,
            center_of_buoyancy,
        }
    }
}
