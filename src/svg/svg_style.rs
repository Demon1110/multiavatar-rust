//! SVG 展示样式（不可变）：fill、stroke、stroke-width、线帽、线连接与元素级不透明度。

use super::svg_color::SvgColor;

/// 线帽（对应 SVG stroke-linecap）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

/// 线连接（对应 SVG stroke-linejoin）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

/// 未指定时的默认描边宽度（SVG 默认 1）。
const DEFAULT_STROKE_WIDTH: f64 = 1.0;

/// SVG 展示样式（不可变）。
#[derive(Debug, Clone, Copy)]
pub struct SvgStyle {
    pub fill: SvgColor,
    pub stroke: SvgColor,
    pub stroke_width: f64,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub opacity: f64,
}

impl SvgStyle {
    /// 空样式（fill 黑、无 stroke、不透明）。
    pub fn empty() -> SvgStyle {
        SvgStyle {
            fill: SvgColor::parse("#000"),
            stroke: SvgColor::None,
            stroke_width: DEFAULT_STROKE_WIDTH,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            opacity: 1.0,
        }
    }

    /// 解析 style 属性字符串（可为 null/空，此时返回 [empty]）。
    pub fn parse(style: &str) -> SvgStyle {
        if style.trim().is_empty() {
            return SvgStyle::empty();
        }
        let mut fill = SvgColor::parse("#000");
        let mut stroke = SvgColor::None;
        let mut stroke_width = DEFAULT_STROKE_WIDTH;
        let mut line_cap = LineCap::Butt;
        let mut line_join = LineJoin::Miter;
        let mut opacity = 1.0;

        for pair in style.split(';') {
            let Some(idx) = pair.find(':') else { continue };
            let key = pair[..idx].trim();
            let value = pair[idx + 1..].trim();
            match key {
                "fill" => fill = SvgColor::parse(value),
                "stroke" => stroke = SvgColor::parse(value),
                "stroke-width" => stroke_width = parse_length(value, DEFAULT_STROKE_WIDTH),
                "stroke-linecap" => {
                    if value == "round" {
                        line_cap = LineCap::Round;
                    } else if value == "square" {
                        line_cap = LineCap::Square;
                    }
                }
                "stroke-linejoin" => {
                    if value == "round" {
                        line_join = LineJoin::Round;
                    } else if value == "bevel" {
                        line_join = LineJoin::Bevel;
                    }
                }
                "opacity" => opacity = parse_opacity(value),
                _ => {}
            }
        }

        SvgStyle {
            fill,
            stroke,
            stroke_width,
            line_cap,
            line_join,
            opacity,
        }
    }
}

/// 解析长度（去掉 px 等单位后缀）。
fn parse_length(raw: &str, dft: f64) -> f64 {
    let s = raw.trim();
    let end = s
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
        .map(|(i, _)| i + 1)
        .last()
        .unwrap_or(0);
    s[..end].parse::<f64>().unwrap_or(dft)
}

/// 解析 0-1 的不透明度。
fn parse_opacity(raw: &str) -> f64 {
    match raw.trim().parse::<f64>() {
        Ok(v) if v < 0.0 => 0.0,
        Ok(v) if v > 1.0 => 1.0,
        Ok(v) => v,
        Err(_) => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full() {
        let style = SvgStyle::parse("fill:#fff;stroke:#000;stroke-width:3px;stroke-linecap:round;stroke-linejoin:round");
        assert_eq!(style.fill, SvgColor::Rgb(255, 255, 255));
        assert_eq!(style.stroke, SvgColor::Rgb(0, 0, 0));
        assert_eq!(style.stroke_width, 3.0);
        assert_eq!(style.line_cap, LineCap::Round);
        assert_eq!(style.line_join, LineJoin::Round);
    }

    #[test]
    fn parse_opacity_key() {
        let style = SvgStyle::parse("fill:#000;opacity:0.96");
        assert_eq!(style.opacity, 0.96);
    }
}
