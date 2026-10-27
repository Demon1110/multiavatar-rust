//! SVG 形状模型：path/polygon/rect/line 四种几何 + 统一绘制流程。
//!
//! 绘制流程（模板方法，等价 Java SvgShape.paint）：
//! 先填充后描边；应用元素级不透明度（style.opacity × 帧级 alpha）；
//! 元素矩阵变换复合在视口变换之后。

use tiny_skia::{FillRule, Paint, Path, Pixmap, Stroke, Transform};

use super::svg_style::{LineCap, LineJoin, SvgStyle};
use super::transform;

/// 线段几何（对应 SVG `<line>`）。
#[derive(Debug, Clone, Copy)]
pub struct Line2D {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

/// SVG 形状（组合模式叶子节点）。
#[derive(Debug, Clone)]
pub enum SvgShape {
    Path {
        path: Path,
        style: SvgStyle,
        transform: Option<Transform>,
    },
    Polygon {
        path: Path,
        style: SvgStyle,
        transform: Option<Transform>,
    },
    Rect {
        rect: Path,
        style: SvgStyle,
        transform: Option<Transform>,
    },
    Line {
        line: Line2D,
        style: SvgStyle,
        transform: Option<Transform>,
    },
}

impl SvgShape {
    /// 返回一个应用了帧级不透明度 `alpha`（0-1）的同几何副本（不可变）。
    /// 实际显示透明度 = 元素自身 opacity × alpha。
    pub fn with_alpha(&self, alpha: f64) -> SvgShape {
        let set = |s: &SvgStyle| SvgStyle { opacity: s.opacity * alpha, ..*s };
        match self {
            SvgShape::Path { path, style, transform } => SvgShape::Path {
                path: path.clone(),
                style: set(style),
                transform: *transform,
            },
            SvgShape::Polygon { path, style, transform } => SvgShape::Polygon {
                path: path.clone(),
                style: set(style),
                transform: *transform,
            },
            SvgShape::Rect { rect, style, transform } => SvgShape::Rect {
                rect: rect.clone(),
                style: set(style),
                transform: *transform,
            },
            SvgShape::Line { line, style, transform } => SvgShape::Line {
                line: *line,
                style: set(style),
                transform: *transform,
            },
        }
    }

    /// 元素自身样式。
    pub fn style(&self) -> &SvgStyle {
        match self {
            SvgShape::Path { style, .. }
            | SvgShape::Polygon { style, .. }
            | SvgShape::Rect { style, .. }
            | SvgShape::Line { style, .. } => style,
        }
    }

    /// 元素矩阵变换。
    pub fn transform(&self) -> Option<Transform> {
        match self {
            SvgShape::Path { transform, .. }
            | SvgShape::Polygon { transform, .. }
            | SvgShape::Rect { transform, .. }
            | SvgShape::Line { transform, .. } => *transform,
        }
    }

    /// 绘制本形状到画布（模板方法：先填充后描边，应用不透明度与矩阵变换）。
    ///
    /// `view_transform`：视口（viewBox → 画布）变换；元素变换复合在其后。
    pub fn paint(&self, pixmap: &mut Pixmap, view_transform: Transform) {
        let (geometry, style, element) = match self {
            SvgShape::Path { path, style, transform } => (Geometry::Path(path), style, *transform),
            SvgShape::Polygon { path, style, transform } => (Geometry::Path(path), style, *transform),
            SvgShape::Rect { rect, style, transform } => (Geometry::Path(rect), style, *transform),
            SvgShape::Line { line, style, transform } => (Geometry::Line(*line), style, *transform),
        };

        // 总变换：先元素变换，后视口变换（与 Java Graphics2D 复合顺序一致）
        let total = match element {
            Some(e) => view_transform.pre_concat(e),
            None => view_transform,
        };

        // SVG 语义：先填充，再描边；opacity 同时作用于填充与描边
        let opacity = style.opacity.clamp(0.0, 1.0);

        if !style.fill.is_none() {
            let mut paint = Paint::default();
            let (r, g, b) = match style.fill {
                super::svg_color::SvgColor::Rgb(r, g, b) => (r, g, b),
                super::svg_color::SvgColor::None => (0, 0, 0),
            };
            paint.set_color_rgba8(r, g, b, (255.0 * opacity).round() as u8);
            match &geometry {
                Geometry::Path(path) => {
                    pixmap.fill_path(path, &paint, FillRule::Winding, total, None);
                }
                Geometry::Line(_) => {
                    // 线段无填充面积（Java Line2D fill 无效果），跳过
                }
            }
        }

        if !style.stroke.is_none() && style.stroke_width > 0.0 {
            let mut paint = Paint::default();
            let (r, g, b) = match style.stroke {
                super::svg_color::SvgColor::Rgb(r, g, b) => (r, g, b),
                super::svg_color::SvgColor::None => (0, 0, 0),
            };
            paint.set_color_rgba8(r, g, b, (255.0 * opacity).round() as u8);
            let stroke = Stroke {
                width: style.stroke_width as f32,
                line_cap: match style.line_cap {
                    LineCap::Butt => tiny_skia::LineCap::Butt,
                    LineCap::Round => tiny_skia::LineCap::Round,
                    LineCap::Square => tiny_skia::LineCap::Square,
                },
                line_join: match style.line_join {
                    LineJoin::Miter => tiny_skia::LineJoin::Miter,
                    LineJoin::Round => tiny_skia::LineJoin::Round,
                    LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
                },
                miter_limit: 4.0, // SVG 默认 miter 上限
                dash: None,
            };
            match &geometry {
                Geometry::Path(path) => {
                    pixmap.stroke_path(path, &paint, &stroke, total, None);
                }
                Geometry::Line(line) => {
                    let path = line_path(line);
                    if let Some(p) = path {
                        pixmap.stroke_path(&p, &paint, &stroke, total, None);
                    }
                }
            }
        }
    }
}

enum Geometry<'a> {
    Path(&'a Path),
    Line(Line2D),
}

/// 线段 → tiny-skia Path（描边用）。
fn line_path(line: &Line2D) -> Option<Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(line.x1 as f32, line.y1 as f32);
    pb.line_to(line.x2 as f32, line.y2 as f32);
    pb.finish()
}

/// 构建圆角矩形路径（rx/ry 相等时的圆弧近似，kappa 0.5523）。
pub fn rounded_rect_path(x: f64, y: f64, w: f64, h: f64, r: f64) -> Option<Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    if r <= 0.0 || r * 2.0 > w || r * 2.0 > h {
        pb.move_to(x as f32, y as f32);
        pb.line_to((x + w) as f32, y as f32);
        pb.line_to((x + w) as f32, (y + h) as f32);
        pb.line_to(x as f32, (y + h) as f32);
        pb.close();
        return pb.finish();
    }
    pb.move_to((x + r) as f32, y as f32);
    pb.line_to((x + w - r) as f32, y as f32);
    arc_90(&mut pb, x + w - r, y + r, r, -90.0);
    pb.line_to((x + w) as f32, (y + h - r) as f32);
    arc_90(&mut pb, x + w - r, y + h - r, r, 0.0);
    pb.line_to((x + r) as f32, (y + h) as f32);
    arc_90(&mut pb, x + r, y + h - r, r, 90.0);
    pb.line_to(x as f32, (y + r) as f32);
    arc_90(&mut pb, x + r, y + r, r, 180.0);
    pb.close();
    pb.finish()
}

/// 顺时针 90° 圆弧（三次贝塞尔近似），起点角度 start_deg（度）。
fn arc_90(pb: &mut tiny_skia::PathBuilder, cx: f64, cy: f64, r: f64, start_deg: f64) {
    const KAPPA: f64 = 0.5522847498; // 4/3*(√2-1)
    let th0 = start_deg.to_radians();
    let th1 = th0 + std::f64::consts::FRAC_PI_2;
    let p0x = cx + r * th0.cos();
    let p0y = cy + r * th0.sin();
    let p1x = cx + r * th1.cos();
    let p1y = cy + r * th1.sin();
    let cp1x = p0x - KAPPA * r * th0.sin();
    let cp1y = p0y + KAPPA * r * th0.cos();
    let cp2x = p1x + KAPPA * r * th1.sin();
    let cp2y = p1y - KAPPA * r * th1.cos();
    pb.cubic_to(cp1x as f32, cp1y as f32, cp2x as f32, cp2y as f32, p1x as f32, p1y as f32);
}

/// 生成 rect 几何（圆角归一与 Java RoundRectangle2D 一致：arc = rx>0 ? rx : ry）。
pub fn make_rect_path(x: f64, y: f64, w: f64, h: f64, rx: f64, ry: f64) -> Option<Path> {
    let arc = if rx > 0.0 { rx } else { ry };
    let arc = if arc > 0.0 { arc } else { 0.0 };
    rounded_rect_path(x, y, w, h, arc)
}

/// 便捷构造：解析 transform 字符串（matrix 形式）。
pub fn parse_transform(value: &str) -> Option<Transform> {
    transform::parse(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_no_round() {
        let p = make_rect_path(0.0, 0.0, 10.0, 10.0, 0.0, -1.0).unwrap();
        let _ = p;
    }

    #[test]
    fn rect_rounded() {
        let p = make_rect_path(0.0, 0.0, 10.0, 10.0, 2.0, 2.0).unwrap();
        let _ = p;
    }
}
