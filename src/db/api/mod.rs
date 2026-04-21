//! Функции для работы с БД
use crate::db::volume_max::MaxDataArray;
use crate::db::{physical_frame::PhysicalFrameArray, serde_parser::IFromJson};
use crate::entities::Bounds;
use sal_core::{dbg::Dbg, error::Error};
use std::collections::HashMap;

mod client;
pub(crate) use client::*;

pub struct Db {
    dbg: Dbg,
    ship_id: String,
    project_id: String,
    api_client: ApiClient,
}
//
impl Db {
    pub fn new(
        parent: &Dbg,
        ship_id: String,
        project_id: String,
        api_client: ApiClient,
    ) -> Self {
        let dbg = Dbg::new(parent, "Db");
        Self {
            dbg,
            ship_id,
            project_id,
            api_client,
        }
    }
    ///
    /// Получение шпаций из физических фреймов
    pub fn frames(&mut self) -> Result<Vec<f64>, Error> {
        let error = Error::new("ShipModel", "physical_bounds");
        let data = &self
                .api_client.fetch(&format!(
                    "SELECT pos_x, frame_index as index FROM physical_frame WHERE ship_id={} AND project_id IS NOT DISTINCT FROM {} ORDER BY index ASC;",
                    self.ship_id, self.project_id,
                )).map_err(|err| error.pass(err))?;
        let physical_frames: Vec<_> = PhysicalFrameArray::parse(&data)
            .map_err(|err| error.pass(err))?
            .data();
    //    let bounds = Bounds::from_array(&physical_frames, 0.).map_err(|err| error.pass(err))?;
        Ok(physical_frames)
    }
    /// Чтение максимального объема и уровня для отсеков
    /// Возвращает мапу (ид отсека, максимальный объем (нетто))
    pub fn compartments_max(&mut self) -> Result<HashMap<String, (Option<f64>, f64)>, Error> {
        let error = Error::new("ShipModel", "compartments_max");
        let data = MaxDataArray::parse(
            &self
                .api_client
                .fetch(&format!(
                    "SELECT
                    s.code as code, \
                    c.level_max as level_max,
                    c.volume_max as volume_max
                FROM
                    \"space\" AS s 
                INNER JOIN 
                    \"space/compartment\" AS c ON s.compartment_id = c.id 
                WHERE ship_id={} AND project_id IS NOT DISTINCT FROM {};",
                    self.ship_id, self.project_id,
                ))
                .map_err(|err| error.pass_with("api_client.fetch", err))?,
        )
        .map_err(|err| error.pass_with("parse", err))?;
        Ok(data.data())
    }
}
