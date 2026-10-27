//! SVG 片段着色器：用主题颜色按序替换部件模板中的 `#xxx;` 颜色占位符。
//!
//! 对应 multiavatar.js 的 getFinal()。为保持与 JS 输出逐字符一致，严格复现其
//! `resultFinal.replace(result[i], colors[i]+';')` 语义 —— 每次只替换第一个匹配，
//! 且只替换占位符数量个颜色（多余颜色按 JS 固有行为忽略）。

use regex::Regex;

use crate::data::DataTables;

/// 对应 JS 的 /#(.*?);/g。
const COLOR_PLACEHOLDER: &str = r"#[^;]*;";

/// 对指定部件的模板着色。
///
/// - `part_name`：部件名（env/head/clo/mouth/eyes/top）
/// - `part_v`：初始角色编号（如 "00"）
/// - `theme`：颜色主题（"A"/"B"/"C"）
/// - 返回着色后的 SVG 片段；模板缺失时返回空串
pub fn paint(tables: &DataTables, part_name: &str, part_v: &str, theme: &str) -> String {
    let colors = tables.colors_of(&format!("{}{}", part_v, theme), part_name);
    let svg = tables.template_of(part_v, part_name);
    let svg = match svg {
        None => return String::new(),
        Some(s) => s,
    };

    // 提取所有 #xxx; 占位符（顺序保持）
    let re = Regex::new(COLOR_PLACEHOLDER).expect("占位符正则编译失败");
    let placeholders: Vec<String> = re
        .find_iter(svg)
        .map(|m| m.as_str().to_string())
        .collect();

    let mut out = svg.to_string();
    for (i, placeholder) in placeholders.iter().enumerate() {
        // 每次只替换第一个匹配（JS replace 语义）；颜色按序取用
        let replacement = format!("{};", colors.map(|c| c[i].as_str()).unwrap_or("#000"));
        out = out.replacen(placeholder, &replacement, 1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_replaced() {
        let tables = DataTables::instance();
        let out = paint(tables, "env", "00", "A");
        // 颜色替换后不应再有 #xxx; 占位符
        assert!(!out.contains("#000;"));
        assert!(!out.is_empty());
    }

    #[test]
    fn missing_template_empty() {
        let tables = DataTables::instance();
        // 不存在角色 99 → 空串
        assert_eq!(paint(tables, "env", "99", "A"), "");
    }
}
