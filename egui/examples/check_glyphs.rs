//! 探测：UI 用到的特殊字符在默认字体（含内置 emoji-icon-font）下是否有字形。
//!
//! 用法：cargo run --example check_glyphs
//! 输出：每个字符 U+XXXX 与 OK/MISSING；MISSING 即界面上显示方框的字符。

use epaint::text::{Fonts, TextOptions};
use epaint::FontId;

fn main() {
    // 从 app.rs / theme.rs 提取的全部非 ASCII 特殊字符
    let chars: &[(char, &str)] = &[
        ('🎲', "随机"),
        ('✏', "输入文字"),
        ('⚡', "生成 / 赛博霓虹"),
        ('🕘', "历史"),
        ('📐', "尺寸"),
        ('🖼', "预览格式 / 生成网格"),
        ('❓', "格式说明"),
        ('💾', "导出"),
        ('📋', "复制 data-URI"),
        ('👤', "中央预览"),
        ('✨', "开始生成"),
        ('📁', "批量导出"),
        ('🌙', "暗夜霓虹"),
        ('☀', "明亮"),
        ('·', "间隔点"),
        ('…', "省略号"),
        ('→', "箭头"),
        ('「', "引号"),
        ('」', "引号"),
    ];

    let mut defs = eframe::egui::FontDefinitions::default();
    // 模拟运行时：注入系统 CJK 字体（覆盖 →、「」 等 CJK 标点）
    if let Ok(bytes) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
        defs.font_data
            .insert("cjk".to_owned(), eframe::egui::FontData::from_owned(bytes).into());
        for family in [
            eframe::egui::FontFamily::Proportional,
            eframe::egui::FontFamily::Monospace,
        ] {
            defs.families.entry(family).or_default().push("cjk".to_owned());
        }
    }
    let mut fonts = Fonts::new(TextOptions::default(), defs);

    let mut missing = 0;
    for (c, label) in chars {
        let has = fonts.has_glyph(&FontId::proportional(14.0), *c);
        println!(
            "U+{:04X} {} {} {}",
            *c as u32,
            c,
            if has { "OK     " } else { "MISSING" },
            label
        );
        if !has {
            missing += 1;
        }
    }
    println!("missing total: {missing}");
}
