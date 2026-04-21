use serde::{Deserialize};

///
/// Пути для чтения и записи
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ModelConf {
    pub name: String,
    pub midel_x: f64,    
}
