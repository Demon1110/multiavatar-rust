//! SVG transform 属性解析器。
//!
//! 本工程 SVG 数据仅使用 `matrix(a b c d e f)`（参数可能为科学计数法，
//! 如 `matrix(1 0 0 .99987 4e-5 -3e-5)`）。

/// 解析 transform 属性；null/空/未知形式返回 None（表示无变换）。
pub fn parse(value: &str) -> Option<tiny_skia::Transform> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    if let Some(inner) = v.strip_prefix("matrix(") {
        let inner = inner.strip_suffix(')')?;
        let parts: Vec<&str> = inner.split([',', ' ', '\t', '\r', '\n']).filter(|s| !s.is_empty()).collect();
        if parts.len() >= 6 {
            let nums: Vec<f64> = parts[..6].iter().map(|p| p.parse::<f64>().unwrap_or(0.0)).collect();
            return Some(tiny_skia::Transform::from_row(
                nums[0] as f32, nums[1] as f32, nums[2] as f32,
                nums[3] as f32, nums[4] as f32, nums[5] as f32,
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_matrix() {
        let t = parse("matrix(1 0 0 .99987 4e-5 -3e-5)").unwrap();
        assert!((t.sx - 1.0).abs() < 1e-9);
        assert!((t.sy - 0.99987).abs() < 1e-9);
        assert!((t.tx - 4e-5).abs() < 1e-9);
        assert!((t.ty + 3e-5).abs() < 1e-9);
    }

    #[test]
    fn parse_empty_none() {
        assert!(parse("").is_none());
        assert!(parse("translate(1,2)").is_none());
    }
}
