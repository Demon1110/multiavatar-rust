//! SVG 颜色（不可变）。
//!
//! 支持 `#RGB`、`#RRGGBB` 十六进制与 CSS 命名色；`none` 表示不绘制。
//! 未知颜色按黑色处理（与 Java 版一致）。

/// SVG 颜色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvgColor {
    /// 表示 SVG 的 none（不填充/不描边）。
    None,
    /// RGB 颜色（0-255）。
    Rgb(u8, u8, u8),
}

impl SvgColor {
    /// 解析颜色字符串。
    ///
    /// 如 "#fff"、"#884f00"、"black"、"none"（null/空 按 none 处理）。
    pub fn parse(value: &str) -> SvgColor {
        let v = value.trim();
        if v.is_empty() || v.eq_ignore_ascii_case("none") || v.eq_ignore_ascii_case("transparent") {
            return SvgColor::None;
        }
        if let Some(hex) = v.strip_prefix('#') {
            if hex.len() == 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..1], 16),
                    u8::from_str_radix(&hex[1..2], 16),
                    u8::from_str_radix(&hex[2..3], 16),
                ) {
                    return SvgColor::Rgb(r * 17, g * 17, b * 17);
                }
            } else if hex.len() == 6 {
                if let Ok(v) = u32::from_str_radix(hex, 16) {
                    return SvgColor::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
                }
            } else if hex.len() == 8 {
                // 非标准 8 位写法：取前 6 位作为 RGB
                if let Ok(v) = u32::from_str_radix(&hex[0..6], 16) {
                    return SvgColor::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
                }
            }
            // 非法 hex 按黑色处理
            return SvgColor::Rgb(0, 0, 0);
        }
        // CSS 命名色表（本工程 SVG 数据实际用到的命名色）
        let named = match v.to_ascii_lowercase().as_str() {
            "black" => Some((0, 0, 0)),
            "white" => Some((255, 255, 255)),
            "red" => Some((255, 0, 0)),
            "aqua" | "cyan" => Some((0, 255, 255)),
            "magenta" => Some((255, 0, 255)),
            "yellow" => Some((255, 255, 0)),
            "gray" | "grey" => Some((128, 128, 128)),
            "silver" => Some((192, 192, 192)),
            "blue" => Some((0, 0, 255)),
            "green" => Some((0, 128, 0)),
            "orange" => Some((255, 165, 0)),
            "purple" => Some((128, 0, 128)),
            _ => None,
        };
        match named {
            Some((r, g, b)) => SvgColor::Rgb(r, g, b),
            None => SvgColor::Rgb(0, 0, 0), // 未知颜色按黑色
        }
    }

    /// 是否为 none（不绘制）。
    pub fn is_none(&self) -> bool {
        matches!(self, SvgColor::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex() {
        assert_eq!(SvgColor::parse("#fff"), SvgColor::Rgb(255, 255, 255));
        assert_eq!(SvgColor::parse("#884f00"), SvgColor::Rgb(0x88, 0x4f, 0x00));
        assert_eq!(SvgColor::parse("#000"), SvgColor::Rgb(0, 0, 0));
    }

    #[test]
    fn parse_named_and_none() {
        assert_eq!(SvgColor::parse("black"), SvgColor::Rgb(0, 0, 0));
        assert_eq!(SvgColor::parse("none"), SvgColor::None);
        assert_eq!(SvgColor::parse(""), SvgColor::None);
        assert_eq!(SvgColor::parse("transparent"), SvgColor::None);
    }
}
