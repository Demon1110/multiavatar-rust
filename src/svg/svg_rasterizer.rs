//! SVG 光栅化器：把 [SvgDocument] 绘制到 tiny-skia [Pixmap]。
//!
//! 流程：按 viewBox 等比缩放并居中映射到画布 → 按文档顺序绘制每个形状
//! （先填充后描边，支持元素级不透明度与矩阵变换）。
//! 画布默认 ARGB（透明背景）；可指定背景色（如 JPG 白底）。
//! 另提供「部分形状」绘制重载（GIF 帧动画用）：只绘制文档中前 N 个形状，
//! 最后一个可应用帧级不透明度实现淡入。

use tiny_skia::{Pixmap, Transform};

use super::svg_document::SvgDocument;

/// 光栅化器：把 [SvgDocument] 绘制到 tiny-skia [Pixmap]。
pub struct SvgRasterizer;

impl SvgRasterizer {
    /// 光栅化（透明背景）。
    pub fn rasterize(doc: &SvgDocument, size: u32) -> Option<Pixmap> {
        Self::rasterize_partial(doc, size, doc.shapes().len(), 1.0, None)
    }

    /// 光栅化（可指定背景色；None 表示透明背景）。
    pub fn rasterize_with_background(
        doc: &SvgDocument,
        size: u32,
        background: Option<(u8, u8, u8)>,
    ) -> Option<Pixmap> {
        Self::rasterize_partial(doc, size, doc.shapes().len(), 1.0, background)
    }

    /// 光栅化部分形状（GIF 帧动画）：只绘制文档前 `shape_count` 个形状，
    /// 其中最新的一个（最后一个）应用帧级不透明度 `newest_alpha`（0-1，1 为全量显示）。
    pub fn rasterize_partial(
        doc: &SvgDocument,
        size: u32,
        shape_count: usize,
        newest_alpha: f64,
        background: Option<(u8, u8, u8)>,
    ) -> Option<Pixmap> {
        let mut pixmap = Pixmap::new(size, size)?;
        if let Some((r, g, b)) = background {
            pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, 255));
        }

        let vw = doc.view_width();
        let vh = doc.view_height();
        let scale = if vw > 0.0 && vh > 0.0 {
            (size as f64 / vw).min(size as f64 / vh) as f32
        } else {
            1.0
        };
        let tx = ((size as f64 - vw * scale as f64) / 2.0 - doc.view_x() * scale as f64) as f32;
        let ty = ((size as f64 - vh * scale as f64) / 2.0 - doc.view_y() * scale as f64) as f32;
        // 先缩放后平移（点变换 = translate(scale(p))，与 Java g.translate+scale 一致）
        let view = Transform::from_scale(scale, scale).post_translate(tx, ty);

        let shapes = doc.shapes();
        let n = shape_count.min(shapes.len());
        for (i, shape) in shapes.iter().take(n).enumerate() {
            let alpha = if i == n - 1 { newest_alpha } else { 1.0 };
            if alpha >= 1.0 {
                shape.paint(&mut pixmap, view);
            } else {
                shape.with_alpha(alpha).paint(&mut pixmap, view);
            }
        }
        Some(pixmap)
    }
}
