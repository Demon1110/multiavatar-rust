//! 演示：生成各格式产物到 `demo/` 目录（等价 Java 版 main 演示）。
//!
//! ```bash
//! cargo run --example demo
//! ```
//!
//! 产物：
//! - demo/{input}.svg / .png / .jpg / .gif
//! - demo/favicon.zip / favicon.ico
//! - demo/preview_grid.png（多输入横排预览，兼作批量回归图）
//! - demo/data_uri.txt（data URI 前缀示例）

use std::fs;
use std::path::Path;

use multiavatar::*;

/// 演示输入集合。
const DEMO_INPUTS: &[&str] = &[
    "Binx Bond",
    "demon1110",
    "star",
    "admin",
    "193816",
    "8月份就好好吃饭",
    "hello world",
    "네이버",
    "日本語テスト",
    "😀emoji test",
];

fn main() {
    let dir = Path::new("demo");
    fs::create_dir_all(dir).expect("创建 demo 目录失败");

    // 各格式产物
    for input in DEMO_INPUTS {
        let name = sanitize(input);
        fs::write(dir.join(format!("{}.svg", name)), multiavatar(input)).expect("写 SVG 失败");
        fs::write(dir.join(format!("{}.png", name)), to_png(input)).expect("写 PNG 失败");
        fs::write(dir.join(format!("{}.jpg", name)), to_jpg(input)).expect("写 JPG 失败");
        fs::write(dir.join(format!("{}.gif", name)), to_gif(input)).expect("写 GIF 失败");
    }

    // favicon 包
    fs::write(dir.join("favicon.zip"), to_favicon_zip("Binx Bond")).expect("写 favicon.zip 失败");
    fs::write(dir.join("favicon.ico"), to_favicon_ico("Binx Bond")).expect("写 favicon.ico 失败");

    // data URI 示例
    let uri = to_data_uri("Binx Bond");
    fs::write(
        dir.join("data_uri.txt"),
        format!(
            "data URI 前缀：{}\n完整长度：{} 字符\n",
            &uri[..40],
            uri.len()
        ),
    )
    .expect("写 data_uri.txt 失败");

    // 横排预览（256 间距，多输入拼一张，兼作批量回归图）
    let grid = build_preview_grid(DEMO_INPUTS, 256, 6);
    fs::write(dir.join("preview_grid.png"), grid).expect("写 preview_grid.png 失败");

    // 控制台摘要
    println!("已生成 {} 组产物到 {}/", DEMO_INPUTS.len(), dir.display());
    println!("cache_size = {}（同输入重复调用命中缓存）", cache_size());
    for input in DEMO_INPUTS {
        let name = sanitize(input);
        let len = fs::metadata(dir.join(format!("{}.png", name)))
            .map(|m| m.len())
            .unwrap_or(0);
        println!("  {} -> {} bytes", input, len);
    }
    clear_cache();
}

/// 输入 → 安全文件名。
fn sanitize(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .chars()
        .take(24)
        .collect()
}

/// 横排预览图：把多个输入的头像横向拼到一张 PNG。
///
/// 每个头像 `cell` 像素，间距 8px，白底；调用 [render] 逐格绘制。
fn build_preview_grid(inputs: &[&str], cell: u32, columns: usize) -> Vec<u8> {
    let cols = columns.min(inputs.len());
    let rows = inputs.len().div_ceil(cols);
    let gap: u32 = 8;
    let width = cols as u32 * cell + (cols as u32 + 1) * gap;
    let height = rows as u32 * cell + (rows as u32 + 1) * gap;

    let mut canvas = tiny_skia::Pixmap::new(width, height).expect("画布创建失败");
    canvas.fill(tiny_skia::Color::WHITE);

    for (i, input) in inputs.iter().enumerate() {
        let x = (i % cols) as u32 * (cell + gap) + gap;
        let y = (i / cols) as u32 * (cell + gap) + gap;
        if let Some(avatar_img) = to_image_sized(input, cell) {
            canvas.draw_pixmap(
                x as i32,
                y as i32,
                avatar_img.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                tiny_skia::Transform::identity(),
                None,
            );
        }
    }
    multiavatar::render::png_writer::to_png(&canvas)
}
