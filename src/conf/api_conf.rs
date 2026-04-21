use serde::{Deserialize};

///
/// Данные для инициализации api-server
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ApiAddress {
    pub host: String,
    pub port: String,
    pub database: String,
}
///
/// Данные для выборки из БД
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Params {
    #[serde(alias = "ship-id")]
    pub ship_id: String,
    #[serde(alias = "project-id")]
    pub project_id: String,
}
///
/// Данные для доступа к БД
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ApiConf {
    #[serde(alias = "api-address")]
    pub address: ApiAddress,
    pub params: Params,
}