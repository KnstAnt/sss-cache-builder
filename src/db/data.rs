use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::DataArray;

// Структура для парсинга данных параметров судна
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataShip {
    pub key: String,
    pub value: Option<f64>,
}
//
impl std::fmt::Display for DataShip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DataShip(key:{}, value:{:?})", self.key, self.value)
    }
}
//
pub type DataShipArray = DataArray<DataShip>;
//
impl DataShipArray {
    /// Преобразование данных в массив ключ + значение
    pub fn data(&self) -> HashMap<String, f64> {
        self.data
            .iter()
            .filter(|v| v.value.is_some())
            .map(|v| (v.key.clone(), v.value.unwrap()))
            .collect()
    }
}
