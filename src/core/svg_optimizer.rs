//! SVG 体积优化器（纯文本级处理，不解析 DOM，保持其余部分逐字符不变）。
//!
//! 优化项（均近无损，渲染几何等价）：
//! - 坐标精度裁剪：`d="..."` 内数字四舍五入——普通填充形状到 1 位小数，
//!   含描边（`stroke:`）的细线条形状到 2 位小数；`transform="..."` 矩阵与
//!   `points="..."` 多边形坐标统一 1 位；尾零去除、整数不带小数点、负零归一。
//! - style 尾分号删除：`fill:#333;` → `fill:#333`（CSS 允许无尾分号）。
//!
//! 实现为手写线性扫描器（零第三方依赖），逐字符复刻官方 JS/Java 正则语义：
//! - 形状标签 `((?:path|polygon|rect|line)[^>]*?)\bd="([^"]*)"([^>]*?)\bstyle="([^"]*)"`
//! - 属性 `\btransform="..."` / `\bpoints="..."`
//! - 数字 token `-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?`（要求整数部分，前导小数点不匹配）

/// 优化 SVG 文本（近无损）；空串原样返回。
pub fn optimize(svg: &str) -> String {
    if svg.is_empty() {
        return svg.to_string();
    }

    // 1. 形状标签：d 按是否描边取整（1 位 / 描边 2 位）
    let out1 = replace_shapes(svg);
    // 2. transform / points 数值 1 位
    let out2 = replace_attribute(&out1, "transform", 1);
    let out3 = replace_attribute(&out2, "points", 1);
    // 3. style 尾分号删除
    out3.replace(";\"", "\"")
}

/// 形状标签内 `d=`/`style=` 双属性替换（复刻 SHAPE_TAG 正则）。
/// 匹配语义：`<` 后 tag 名（path/polygon/rect/line）→ 同一标签内（不跨 `>`）
/// 依次找 `\bd="..."` 与 `\bstyle="..."`；二者齐备才替换，否则标签原样保留。
/// 重建模板与官方一致：前缀组已含原始空格，再补空格 → 输出双空格（官方原样）。
fn replace_shapes(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(pos) = rest.find('<') {
        out.push_str(&rest[..pos]);
        let after_lt = &rest[pos + 1..];
        if let Some((tag, after_tag)) = shape_tag(after_lt) {
            if let Some((pre1, d_val, tail1)) = find_attr(after_tag, "d", true) {
                if let Some((pre2, style_val, tail2)) = find_attr(tail1, "style", true) {
                    let d_rounded =
                        round_attribute_value(d_val, if style_val.contains("stroke") { 2 } else { 1 });
                    out.push('<');
                    out.push_str(tag);
                    out.push_str(pre1);
                    out.push_str(" d=\"");
                    out.push_str(&d_rounded);
                    out.push('"');
                    out.push_str(pre2);
                    out.push_str(" style=\"");
                    out.push_str(style_val);
                    out.push('"');
                    rest = tail2;
                    continue;
                }
            }
        }
        out.push('<');
        rest = after_lt;
    }
    out.push_str(rest);
    out
}

/// 在字符串开头识别形状标签名（前缀匹配，与正则交替组一致）。
fn shape_tag(s: &str) -> Option<(&str, &str)> {
    for tag in ["polygon", "path", "rect", "line"] {
        if s.starts_with(tag) {
            return Some((tag, &s[tag.len()..]));
        }
    }
    None
}

/// 属性替换：复刻 `\b{attr}="([^"]*)"`，值内数字舍入后重建 `{attr}="..."`。
/// 词边界语义：属性名首字符前一字符不得为 ASCII 字母/数字/下划线。
fn replace_attribute(svg: &str, attr: &str, decimals: usize) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    loop {
        // 属性正则 `\b{attr}="..."` 无 [^>] 限制，可跨标签扫描整个文档
        match find_attr(rest, attr, false) {
            Some((pre, val, tail)) => {
                out.push_str(pre);
                out.push_str(attr);
                out.push_str("=\"");
                out.push_str(&round_attribute_value(val, decimals));
                out.push('"');
                rest = tail;
            }
            None => {
                out.push_str(rest);
                break;
            }
        }
    }
    out
}

/// 查找 `{attr}="..."` 属性：返回 (到属性名前的原文, 属性值, 值闭合引号后的剩余)。
/// 语义：属性名前需词边界、属性名后需 `="`；`stop_at_gt = true` 时同一标签内
/// （遇到 `>` 视为越界失败，对应 shape 正则的 `[^>]*?`），否则跨标签扫描全文。
fn find_attr<'a>(s: &'a str, attr: &str, stop_at_gt: bool) -> Option<(&'a str, &'a str, &'a str)> {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if stop_at_gt && b[i] == b'>' {
            return None;
        }
        if s[i..].starts_with(attr) && word_boundary_before(s, i) {
            let after = i + attr.len();
            if s[after..].starts_with("=\"") {
                let vstart = after + 2;
                if let Some(vend_rel) = s[vstart..].find('"') {
                    let vend = vstart + vend_rel;
                    return Some((&s[..i], &s[vstart..vend], &s[vend + 1..]));
                }
                return None;
            }
        }
        i += 1;
    }
    None
}

/// `\b` 语义：位置 i 之前无 ASCII word 字符（字母/数字/下划线）；i=0 视为边界。
fn word_boundary_before(s: &str, i: usize) -> bool {
    if i == 0 {
        return true;
    }
    let c = s.as_bytes()[i - 1];
    !(c.is_ascii_alphanumeric() || c == b'_')
}

/// 属性值内数字 token 四舍五入（复刻 NUMBER 正则 replace_all）。
/// token：`-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?` —— 必须整数开头，前导小数点不匹配。
fn round_attribute_value(value: &str, decimals: usize) -> String {
    let b = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut i = 0;
    while i < b.len() {
        let neg = b[i] == b'-' && i + 1 < b.len() && b[i + 1].is_ascii_digit();
        if b[i].is_ascii_digit() || neg {
            let start = i;
            let mut j = i;
            if neg {
                j += 1;
            }
            // \d+
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            // (?:\.\d+)?
            if j + 1 < b.len() && b[j] == b'.' && b[j + 1].is_ascii_digit() {
                j += 1;
                while j < b.len() && b[j].is_ascii_digit() {
                    j += 1;
                }
            }
            // (?:[eE][+-]?\d+)?
            if j < b.len() && (b[j] == b'e' || b[j] == b'E') {
                let mut k = j + 1;
                if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
                    k += 1;
                }
                if k < b.len() && b[k].is_ascii_digit() {
                    j = k;
                    while j < b.len() && b[j].is_ascii_digit() {
                        j += 1;
                    }
                }
            }
            out.push_str(&round_token(&value[start..j], decimals));
            i = j;
        } else {
            let ch = value[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
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
