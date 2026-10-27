//! SVG 渲染策略：原样返回 SVG 文本（UTF-8 字节）。

use super::avatar_renderer::AvatarRenderer;

/// SVG 渲染器（原样返回文本）。
pub struct SvgAvatarRenderer;

impl AvatarRenderer for SvgAvatarRenderer {
    fn render(&self, svg: &str, _size: u32) -> Vec<u8> {
        svg.as_bytes().to_vec()
    }
}
