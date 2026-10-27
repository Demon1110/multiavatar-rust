//! SVG 奇偶校验锚点：与 Java（官方 JS）基准逐字符一致的回归快照。
//!
//! 这些字符串来自 multiavatar-java 的 GenBaseline 输出（默认选项，体积优化开启），
//! Rust 输出必须逐字符一致；任何坐标/格式偏差都会在这里暴露。

use multiavatar::*;

/// 锚点：与 Java 基准逐字符一致的完整 SVG。
#[test]
fn svg_parity_anchor_binx_bond() {
    let expected = include_str!("snapshots/binx_bond.svg");
    let actual = multiavatar("Binx Bond");
    assert_eq!(actual, expected, "Binx Bond SVG 与 Java 基准不一致");
}

/// 锚点：中文/emoji 输入（UTF-16 命名路径、非 ASCII 数据）。
#[test]
fn svg_parity_anchor_cjk() {
    let expected = include_str!("snapshots/cjk.svg");
    let actual = multiavatar("8月份就好好吃饭");
    assert_eq!(actual, expected, "中文输入 SVG 与 Java 基准不一致");
}

/// 锚点：多语言/emoji 输入。
#[test]
fn svg_parity_anchor_emoji() {
    let expected = include_str!("snapshots/emoji.svg");
    let actual = multiavatar("😀emoji test");
    assert_eq!(actual, expected, "emoji 输入 SVG 与 Java 基准不一致");
}

/// 锚点：强制 part/theme 的确定性输出。
#[test]
fn svg_parity_anchor_forced() {
    let expected = include_str!("snapshots/forced.svg");
    let actual = multiavatar_full("abc123XYZ", false, Some("12"), Some("C"));
    assert_eq!(actual, expected, "强制 part/theme SVG 与 Java 基准不一致");
}
