//! SVG 文档模型（不可变）：viewBox 视口与按文档顺序排列的形状列表。

use super::svg_shape::SvgShape;

/// SVG 文档模型（不可变）。
#[derive(Debug, Clone)]
pub struct SvgDocument {
    view_x: f64,
    view_y: f64,
    view_width: f64,
    view_height: f64,
    shapes: Vec<SvgShape>,
}

impl SvgDocument {
    /// 空文档（无视口、无形状）。
    pub fn empty() -> SvgDocument {
        SvgDocument {
            view_x: 0.0,
            view_y: 0.0,
            view_width: 0.0,
            view_height: 0.0,
            shapes: Vec::new(),
        }
    }

    pub fn new(
        view_x: f64,
        view_y: f64,
        view_width: f64,
        view_height: f64,
        shapes: Vec<SvgShape>,
    ) -> SvgDocument {
        SvgDocument {
            view_x,
            view_y,
            view_width,
            view_height,
            shapes,
        }
    }

    pub fn view_x(&self) -> f64 {
        self.view_x
    }
    pub fn view_y(&self) -> f64 {
        self.view_y
    }
    pub fn view_width(&self) -> f64 {
        self.view_width
    }
    pub fn view_height(&self) -> f64 {
        self.view_height
    }

    /// 按文档顺序排列的形状（只读）。
    pub fn shapes(&self) -> &[SvgShape] {
        &self.shapes
    }
}
