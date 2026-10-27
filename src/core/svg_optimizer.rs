//! SVG 体积优化器（纯文本级处理，不解析 DOM，保持其余部分逐字符不变）。
//!
//! 优化项（均近无损，渲染几何等价）：
//! - 坐标精度裁剪：`d="..."` 内数字四舍五入——普通填充形状到 1 位小数，
//!   含描边（`stroke:`）的细线条形状到 2 位小数；`transform="..."` 矩阵与
//!   `points="..."` 多边形坐标统一 1 位；尾零去除、整数不带小数点、负零归一。
//! - style 尾分号删除：`fill:#333;` → `fill:#333`（CSS 允许无尾分号）。

use regex::Regex;

/// 形状标签：<名 前缀… d="…" 中间… style="…" 后缀…>；替换只动 d 与 style 值，保留全部其他属性。
/// 前缀组（1）与中间组（3）均含原始分隔空格，拼接时不再补空格。
const SHAPE_TAG: &str = r#"((?:path|polygon|rect|line)[^>]*?)\bd="([^"]*)"([^>]*?)\bstyle="([^"]*)""#;

/// transform 属性值。
const TRANSFORM_VALUE: &str = r#"\btransform="([^"]*)""#;

/// points 属性值（多边形坐标）。
const POINTS_VALUE: &str = r#"\bpoints="([^"]*)""#;

/// 数字 token：整数 / 小数（**要求整数部分**，与官方 JS / Java 正则一致，
/// 前导小数点形式如 `.99987`、`-.74` 不匹配、原样保留）/ 科学计数法。
const NUMBER: &str = r"-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?";

/// 优化 SVG 文本（近无损）；空串原样返回。
pub fn optimize(svg: &str) -> String {
    if svg.is_empty() {
        return svg.to_string();
    }

    // 1. 形状标签：d 按是否描边取整（1 位 / 描边 2 位）
    let re_shape = Regex::new(SHAPE_TAG).expect("shape 正则编译失败");
    let out1 = re_shape
        .replace_all(svg, |caps: &regex::Captures| {
            let d = round_attribute_value(&caps[2], if caps[4].contains("stroke") { 2 } else { 1 });
            // 与官方 JS / Java 逐字符一致：组 1/3 已含分隔空格，替换模板再补空格
            // → 输出形如 `<path  d="..."  style="..."/>`（双空格，官方原样）
            format!("{} d=\"{}\"{} style=\"{}\"", &caps[1], d, &caps[3], &caps[4])
        })
        .into_owned();

    // 2. transform / points 数值 1 位
    let re_transform = Regex::new(TRANSFORM_VALUE).expect("transform 正则编译失败");
    let out2 = re_transform
        .replace_all(&out1, |caps: &regex::Captures| {
            format!("transform=\"{}\"", round_attribute_value(&caps[1], 1))
        })
        .into_owned();

    let re_points = Regex::new(POINTS_VALUE).expect("points 正则编译失败");
    let out3 = re_points
        .replace_all(&out2, |caps: &regex::Captures| {
            format!("points=\"{}\"", round_attribute_value(&caps[1], 1))
        })
        .into_owned();

    // 3. style 尾分号删除
    out3.replace(";\"", "\"")
}

fn round_attribute_value(value: &str, decimals: usize) -> String {
    let re = Regex::new(NUMBER).expect("数字正则编译失败");
    re.replace_all(value, |caps: &regex::Captures| round_token(&caps[0], decimals))
        .into_owned()
}

/// 数字 token 四舍五入到指定小数位；整数/负零归一。
///
/// 与 Java `Math.round` + `String.format("%.df", r)` 语义一致：
/// `Math.round(x) = floor(x + 0.5)`（half-up，向正无穷），再用该取整值
/// 重构 d 位小数并剥离尾零；负数取整为 0 时输出 "-0"（保持紧凑负号 token 分隔）。
pub fn round_token(token: &str, decimals: usize) -> String {
    let v: f64 = match token.parse() {
        Ok(v) => v,
        Err(_) => return token.to_string(),
    };
    let mul = 10f64.powi(decimals as i32);
    // Java Math.round 语义：floor(v*mul + 0.5)
    let rounded = (v * mul + 0.5).floor();
    if rounded == 0.0 {
        // 负数取整为 0 时必须保留负号：原文紧凑负号（如 "2.707-0.0428"）中
        // 负号是唯一的 token 分隔符，输出 "0" 会与前一数字粘连成 "2.70"（坐标丢失）；
        // "-0" 是合法 SVG 数字且永远保持分隔。
        return if token.starts_with('-') { "-0".to_string() } else { "0".to_string() };
    }
    // 固定 decimals 位小数，再剥离尾零与尾点（整数不带小数点）
    let neg = rounded < 0.0;
    let scaled_abs = rounded.abs();
    let pow = 10u64.pow(decimals as u32);
    let int_part = (scaled_abs as u64) / pow;
    let frac = (scaled_abs as u64) % pow;
    let mut s = if neg {
        format!("-{}", int_part)
    } else {
        format!("{}", int_part)
    };
    if frac > 0 {
        s.push('.');
        let mut frac_s = format!("{:0width$}", frac, width = decimals);
        while frac_s.ends_with('0') {
            frac_s.pop();
        }
        s.push_str(&frac_s);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_one_decimal() {
        assert_eq!(round_token("12.34", 1), "12.3");
        assert_eq!(round_token("12.35", 1), "12.4"); // HALF_UP（Java 语义）
        assert_eq!(round_token("2.0", 1), "2");
        assert_eq!(round_token("0.05", 1), "0.1");
    }

    #[test]
    fn round_two_decimal() {
        assert_eq!(round_token("1.234", 2), "1.23");
        assert_eq!(round_token("1.235", 2), "1.24");
        assert_eq!(round_token("1.50", 2), "1.5");
    }

    #[test]
    fn negative_zero_keeps_sign() {
        assert_eq!(round_token("-0.0428", 1), "-0");
        assert_eq!(round_token("0.0428", 1), "0");
        assert_eq!(round_token("-0.0428", 2), "-0.04");
    }

    #[test]
    fn negative_values() {
        // Java Math.round = floor(x+0.5)：-23.5 → floor(-23.0) = -23
        assert_eq!(round_token("-2.35", 1), "-2.3");
        assert_eq!(round_token("-2.34", 1), "-2.3");
        assert_eq!(round_token("-63.65", 1), "-63.6"); // -636.5 → floor(-636.0) = -636
    }

    #[test]
    fn optimize_strips_trailing_semicolon() {
        let svg = "<path d=\"1.234 2.345\" style=\"fill:#333;\"/>";
        let out = optimize(svg);
        // 双空格与官方 JS/Java 一致
        assert_eq!(out, "<path  d=\"1.2 2.3\"  style=\"fill:#333\"/>");
    }

    #[test]
    fn optimize_stroke_keeps_two_decimals() {
        let svg = "<path d=\"1.234 2.345\" style=\"fill:none;stroke:#000;stroke-width:2\"/>";
        let out = optimize(svg);
        assert_eq!(
            out,
            "<path  d=\"1.23 2.35\"  style=\"fill:none;stroke:#000;stroke-width:2\"/>"
        );
    }

    #[test]
    fn optimize_transform_and_points() {
        let svg = "<polygon points=\"0.05 1.05, 2.25 3.35\" transform=\"matrix(1 0 0 .99987 4e-5 -3e-5)\" style=\"fill:#fff;\"/>";
        let out = optimize(svg);
        // polygon 无 d 属性，不走形状标签阶段（无双空格）；坐标舍入与尾分号清理；
        // .99987 前导小数点不匹配数字 token，原样保留（与官方 JS/Java 一致）
        assert_eq!(
            out,
            "<polygon points=\"0.1 1.1, 2.3 3.4\" transform=\"matrix(1 0 0 .99987 0 -0)\" style=\"fill:#fff\"/>"
        );
    }
}
