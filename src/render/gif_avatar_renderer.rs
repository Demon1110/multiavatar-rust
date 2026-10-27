//! GIF 渲染策略：把 SVG 拆成「部件逐帧淡入」动画并编码为 GIF 字节。
//!
//! 形状按文档顺序即部件组装顺序（env → head → clo → top → eyes → mouth），
//! 对第 k 个形状依次生成「半透明淡入帧 + 全显帧」，每出现一个部件就定格一拍，
//! 形成头像逐步「长出来」的循环动画。
//! 首帧固定为完整头像：外壳预览只取第一帧做缩略图，若首帧是几乎透明的
//! 淡入帧会被渲染成全黑，因此首帧用完整头像保证「不打开也能看到完整图像」。

use crate::svg::{SvgParser, SvgRasterizer};

use super::avatar_renderer::AvatarRenderer;
use super::gif_writer::to_gif;

/// 新部件淡入的起始不透明度。
const FADE_IN_ALPHA: f64 = 0.45;

/// 每帧停留毫秒。
const FRAME_DELAY_MS: u32 = 100;

/// GIF 渲染器。
pub struct GifAvatarRenderer;

impl AvatarRenderer for GifAvatarRenderer {
    fn render(&self, svg: &str, size: u32) -> Vec<u8> {
        let doc = SvgParser::parse(svg);
        let shapes = doc.shapes();
        if shapes.is_empty() {
            return Vec::new();
        }
        // 首帧 = 完整头像（供外壳缩略图预览），随后是部件逐帧淡入动画
        let mut frames = Vec::with_capacity(shapes.len() * 2 + 1);
        frames.push(
            SvgRasterizer::rasterize_partial(&doc, size, shapes.len(), 1.0, None)
                .expect("GIF 首帧渲染失败"),
        );
        for k in 1..=shapes.len() {
            frames.push(
                SvgRasterizer::rasterize_partial(&doc, size, k, FADE_IN_ALPHA, None)
                    .expect("GIF 淡入帧渲染失败"),
            );
            frames.push(
                SvgRasterizer::rasterize_partial(&doc, size, k, 1.0, None)
                    .expect("GIF 全显帧渲染失败"),
            );
        }
        to_gif(&frames, (FRAME_DELAY_MS / 10).max(1) as u16)
    }
}
