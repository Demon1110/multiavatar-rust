//! SVG XML 解析器：把 SVG 字符串解析为 [SvgDocument] 领域模型。
//!
//! 使用 roxmltree（纯只读解析器，不处理 DTD/外部实体，天然无 XXE 面）。

use roxmltree::Node;

use super::path_data_parser;
use super::svg_document::SvgDocument;
use super::svg_shape::{make_rect_path, SvgShape};
use super::svg_style::SvgStyle;
use super::transform;

/// viewBox 缺失时的兜底视口。
const FALLBACK_VIEW: [f64; 4] = [0.0, 0.0, 256.0, 256.0];

/// SVG XML 解析器：把 SVG 字符串解析为 [SvgDocument] 领域模型。
pub struct SvgParser;

impl SvgParser {
    /// 解析 SVG 字符串。
    ///
    /// null/空返回空文档；解析失败 panic（与 Java 抛 IllegalArgumentException 一致）。
    pub fn parse(svg: &str) -> SvgDocument {
        if svg.trim().is_empty() {
            return SvgDocument::empty();
        }
        let doc = roxmltree::Document::parse(svg).unwrap_or_else(|e| {
            panic!("SVG 解析失败: {}", e);
        });
        let root = doc.root_element();
        let view = parse_view_box(root.attribute("viewBox").unwrap_or(""));

        let mut shapes = Vec::new();
        collect(&root, &mut shapes);
        SvgDocument::new(view[0], view[1], view[2], view[3], shapes)
    }
}

/// 递归收集形状元素（保持文档顺序）。
fn collect(node: &Node, out: &mut Vec<SvgShape>) {
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        let el = child;
        match el.tag_name().name() {
            "path" => {
                if let Some(shape) = parse_path(el) {
                    out.push(shape);
                }
            }
            "polygon" => {
                if let Some(shape) = parse_polygon(el) {
                    out.push(shape);
                }
            }
            "line" => out.push(parse_line(el)),
            "rect" => out.push(parse_rect(el)),
            _ => {}
        }
        collect(&el, out);
    }
}

fn parse_path(el: Node) -> Option<SvgShape> {
    let d = el.attribute("d").unwrap_or("");
    let (path, moved) = path_data_parser::parse(d);
    if !moved {
        return None; // 无有效几何（等价 Java getCurrentPoint() == null）
    }
    Some(SvgShape::Path {
        path,
        style: SvgStyle::parse(el.attribute("style").unwrap_or("")),
        transform: transform::parse(el.attribute("transform").unwrap_or("")),
    })
}

fn parse_polygon(el: Node) -> Option<SvgShape> {
    let points = parse_points(el.attribute("points").unwrap_or(""));
    if points.len() < 2 {
        return None;
    }
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(points[0] as f32, points[1] as f32);
    let mut i = 2;
    while i + 1 < points.len() {
        pb.line_to(points[i] as f32, points[i + 1] as f32);
        i += 2;
    }
    pb.close();
    let path = pb.finish().unwrap_or_else(|| {
        // 兜底空路径（points 恒 ≥2 对，正常不会走到）
        let mut pb2 = tiny_skia::PathBuilder::new();
        pb2.move_to(0.0, 0.0);
        pb2.finish().expect("空路径构造失败")
    });
    Some(SvgShape::Polygon {
        path,
        style: SvgStyle::parse(el.attribute("style").unwrap_or("")),
        transform: transform::parse(el.attribute("transform").unwrap_or("")),
    })
}

fn parse_line(el: Node) -> SvgShape {
    SvgShape::Line {
        line: super::svg_shape::Line2D {
            x1: num(el.attribute("x1").unwrap_or("")),
            y1: num(el.attribute("y1").unwrap_or("")),
            x2: num(el.attribute("x2").unwrap_or("")),
            y2: num(el.attribute("y2").unwrap_or("")),
        },
        style: SvgStyle::parse(el.attribute("style").unwrap_or("")),
        transform: transform::parse(el.attribute("transform").unwrap_or("")),
    }
}

fn parse_rect(el: Node) -> SvgShape {
    let x = num(el.attribute("x").unwrap_or(""));
    let y = num(el.attribute("y").unwrap_or(""));
    let w = num(el.attribute("width").unwrap_or(""));
    let h = num(el.attribute("height").unwrap_or(""));
    let mut rx = attr(el, "rx", -1.0);
    let mut ry = attr(el, "ry", -1.0);
    if rx < 0.0 {
        rx = if ry < 0.0 { 0.0 } else { ry };
    }
    if ry < 0.0 {
        ry = rx;
    }
    let rect = make_rect_path(x, y, w, h, rx, ry).unwrap_or_else(|| {
        // 兜底（正常不会走到）
        let mut pb = tiny_skia::PathBuilder::new();
        pb.move_to(0.0, 0.0);
        pb.finish().expect("空 rect 路径构造失败")
    });
    SvgShape::Rect {
        rect,
        style: SvgStyle::parse(el.attribute("style").unwrap_or("")),
        transform: transform::parse(el.attribute("transform").unwrap_or("")),
    }
}

/// 解析 viewBox（"minX minY width height" 或 "width height"）。
fn parse_view_box(vb: &str) -> [f64; 4] {
    let vb = vb.trim();
    if vb.is_empty() {
        return FALLBACK_VIEW;
    }
    let parts: Vec<&str> = vb.split([',', ' ', '\t', '\r', '\n']).filter(|s| !s.is_empty()).collect();
    let parsed: Vec<f64> = parts.iter().filter_map(|p| p.parse::<f64>().ok()).collect();
    match parsed.len() {
        n if n >= 4 => [parsed[0], parsed[1], parsed[2], parsed[3]],
        2 => [0.0, 0.0, parsed[0], parsed[1]],
        _ => FALLBACK_VIEW,
    }
}

/// 解析 points 点列（逗号/空白分隔的坐标对）。
fn parse_points(points: &str) -> Vec<f64> {
    points
        .split([',', ' ', '\t', '\r', '\n'])
        .filter(|s| !s.is_empty())
        .filter_map(|p| p.parse::<f64>().ok())
        .collect()
}

fn attr(el: Node, name: &str, dft: f64) -> f64 {
    match el.attribute(name) {
        Some(v) if !v.is_empty() => num(v),
        _ => dft,
    }
}

fn num(v: &str) -> f64 {
    v.trim().parse::<f64>().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let doc = SvgParser::parse("");
        assert!(doc.shapes().is_empty());
    }

    #[test]
    fn parse_basic_svg() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 231 231">
            <path d="M 10 10 L 20 20" style="fill:#fff;"/>
            <polygon points="0,0 10,0 10,10" style="fill:#000;"/>
            <line x1="1" y1="2" x2="3" y2="4" style="stroke:#000;stroke-width:2"/>
            <rect x="5" y="5" width="10" height="10" rx="2" style="fill:#333;"/>
        </svg>"#;
        let doc = SvgParser::parse(svg);
        assert_eq!(doc.shapes().len(), 4);
        assert_eq!(doc.view_width(), 231.0);
        assert_eq!(doc.view_height(), 231.0);
    }
}
