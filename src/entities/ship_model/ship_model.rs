//! Представление модели судна
use crate::entities::Bounds;
use crate::entities::data::serde_parser::IFromJson;
use crate::entities::data::strength::physical_frame::PhysicalFrameArray;
use crate::entities::model_cached::ModelCached;
use crate::entities::ship_model::volume_max::MaxDataArray;
use crate::infrostructure::ApiClient;
use sal_core::dbg::Dbg;
use sal_core::error::Error;
use std::collections::HashMap;
use std::sync::Arc;
///
///
pub struct ShipModel {
    dbg: Dbg,
    ship_id: String,
    project_id: String,
    bounds: Option<Bounds>,
    model_cached: ModelCached,
    api_client: Arc<ApiClient>,
}
//
//
impl ShipModel {
    ///
    pub fn new(
        parent: impl Into<String>,
        ship_id: String,
        project_id: String,
        model_cached: ModelCached,
        api_client: Arc<ApiClient>,
    ) -> Self {
        let dbg = Dbg::new(parent, "ShipModel");
        Self {
            dbg,
            ship_id,
            project_id,
            bounds: None,
            model_cached,
            api_client,
        }
    }
    ///
    /// TODO: Doc
    pub fn bounds(&self) -> Result<Bounds, Error> {
        self.bounds.clone().ok_or(Error::new(&self.dbg, "bounds"))
    } 
}
///
/// Получение шпаций из физических фреймов
fn get_physical_bounds(
    ship_id: &str,
    project_id: &str,
    api_client: &ApiClient,
) -> Result<Bounds, Error> {
    let error = Error::new("ShipModel", "get_physical_bounds");
    let data = api_client.fetch(&format!(
                "SELECT pos_x, frame_index as index FROM physical_frame WHERE ship_id={ship_id} AND project_id IS NOT DISTINCT FROM {project_id} ORDER BY index ASC;"
            )).map_err(|err| error.pass(err))?;
    let physical_frames: Vec<_> = PhysicalFrameArray::parse(&data)
        .map_err(|err| error.pass(err))?
        .data();
    let bounds = Bounds::from_array(&physical_frames, 0.).map_err(|err| error.pass(err))?;
    Ok(bounds)
}
/// Чтение максимального объема и уровня для отсеков
/// Возвращает мапу (ид отсека, максимальный объем (нетто))
fn compartments_max(
    ship_id: &str,
    project_id: &str,
    api_client: &ApiClient,
) -> Result<HashMap<String, (Option<f64>, f64)>, Error> {
    let error = Error::new("ShipModel", "compartments_max");
    let data = MaxDataArray::parse(
        &api_client
            .fetch(&format!(
                "SELECT
                s.code as code, \
                c.level_max as level_max,
                c.volume_max as volume_max
            FROM
                \"space\" AS s 
            INNER JOIN 
                \"space/compartment\" AS c ON s.compartment_id = c.id 
            WHERE ship_id={ship_id} AND project_id IS NOT DISTINCT FROM {project_id};"
            ))
            .map_err(|err| error.pass_with("api_client.fetch", err))?,
    )
    .map_err(|err| error.pass_with("parse", err))?;
    Ok(data.data())
}
