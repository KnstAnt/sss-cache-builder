use serde::{Deserialize};

///
/// Пути для чтения и записи
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct DataConf {
    pub dir: String,
    pub name: String,
}
