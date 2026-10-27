//! SVG 领域层：对象模型 + 解析 + 光栅化前处理。

pub mod svg_color;
pub mod svg_style;
pub mod transform;
pub mod path_data_parser;
pub mod svg_shape;
pub mod svg_document;
pub mod svg_parser;
pub mod svg_rasterizer;

pub use svg_color::SvgColor;
pub use svg_document::SvgDocument;
pub use svg_parser::SvgParser;
pub use svg_rasterizer::SvgRasterizer;
pub use svg_shape::{Line2D, SvgShape};
pub use svg_style::{LineCap, LineJoin, SvgStyle};