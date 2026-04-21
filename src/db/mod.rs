use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_parser::IFromJson;

pub mod api;
mod data;
mod serde_parser;
pub mod physical_frame;
pub mod volume_max;


/// Массив ключ + значение
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataArray<T> {
    pub data: Vec<T>,
    pub error: HashMap<String, String>,
}
//
impl<T> IFromJson for DataArray<T> {
    fn error(&self) -> Option<&String> {
        self.error.values().next()
    }
}

