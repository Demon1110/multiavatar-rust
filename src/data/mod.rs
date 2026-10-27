//! 静态数据层：颜色主题 + SVG 部件模板。

mod multiavatar_data;
mod data_tables;

pub use data_tables::DataTables;

/// 数据资源（与 Java 版同一 gzip 文件，数据单一来源）。
pub const DATA_GZ: &[u8] = include_bytes!("multiavatar-data.gz");
