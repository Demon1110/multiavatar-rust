//! JPEG 渲染策略：把 SVG 光栅化（白色背景）后编码为 JPEG 字节。

use crate::svg::{SvgParser, SvgRasterizer};

use super::avatar_renderer::AvatarRenderer;
use super::jpg_writer::to_jpg_rgb;
use super::png_writer::to_rgba8;

/// JPEG 渲染器（白底，不透明，适合博客/OA 等场景）。
pub struct JpgAvatarRenderer;

impl AvatarRenderer for JpgAvatarRenderer {
    fn render(&self, svg: &str, size: u32) -> Vec<u8> {
        let doc = SvgParser::parse(svg);
        let pixmap =
            SvgRasterizer::rasterize_with_background(&doc, size, Some((255, 255, 255)))
                .expect("JPEG 光栅化失败");
        // 白底后 alpha 恒为 255，直接取 RGB
        let rgba = to_rgba8(&pixmap);
        let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
        for px in rgba.chunks_exact(4) {
            rgb.extend_from_slice(&px[0..3]);
        }
        to_jpg_rgb(&rgb, size, size)
    }
}
