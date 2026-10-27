//! PNG 渲染策略：把 SVG 光栅化后编码为 PNG 字节。

use crate::svg::{SvgParser, SvgRasterizer};

use super::avatar_renderer::AvatarRenderer;
use super::png_writer::to_png;

/// PNG 渲染器（透明背景）。
pub struct PngAvatarRenderer;

impl AvatarRenderer for PngAvatarRenderer {
    fn render(&self, svg: &str, size: u32) -> Vec<u8> {
        let doc = SvgParser::parse(svg);
        let pixmap = SvgRasterizer::rasterize(&doc, size).expect("PNG 光栅化失败");
        to_png(&pixmap)
    }
}
