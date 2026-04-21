//! Структуры для ввода/вывода данных
pub mod ship;
pub mod ship_data;
pub mod data_array;
pub mod strength;
pub mod serde_parser;

pub use data_array::*;

pub type MetacentricHeightSubdivisionArray = DataArray<Pair>;


