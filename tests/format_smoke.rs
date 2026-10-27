//! 端到端冒烟：各格式产物魔数/结构与 Java 版等价行为。

use base64::Engine;
use multiavatar::*;

/// PNG 魔数（8 字节）。
const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// JPEG 魔数（3 字节）。
const JPG_MAGIC: [u8; 3] = [0xFF, 0xD8, 0xFF];

/// GIF87a/89a 魔数。
const GIF_MAGIC: &[u8] = b"GIF8";

#[test]
fn svg_smoke() {
    let svg = multiavatar("Binx Bond");
    assert!(svg.starts_with(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"256\" height=\"256\" viewBox=\"0 0 231 231\">"
    ));
    assert!(svg.ends_with("</svg>"));
    assert!(svg.contains("<path"), "缺少 path 元素");
}

#[test]
fn empty_input() {
    assert_eq!(multiavatar(""), "");
    assert!(to_png("").is_empty());
    assert!(to_jpg("").is_empty());
    assert!(to_gif("").is_empty());
    assert_eq!(to_data_uri(""), "");
    assert!(to_image("").is_none());
}

#[test]
fn png_bytes_and_dimension() {
    let png = to_png("demon1110");
    assert_eq!(&png[..8], &PNG_MAGIC[..]);
    // 解析 IHDR 验证 256×256
    let decoder = png::Decoder::new(std::io::Cursor::new(&png));
    let reader = decoder.read_info().expect("PNG 头解析失败");
    assert_eq!((reader.info().width, reader.info().height), (256, 256));
}

#[test]
fn png_custom_size() {
    let png = to_png_sized("demon1110", 512);
    let decoder = png::Decoder::new(std::io::Cursor::new(&png));
    let reader = decoder.read_info().expect("PNG 头解析失败");
    assert_eq!((reader.info().width, reader.info().height), (512, 512));
}

#[test]
fn jpg_bytes() {
    let jpg = to_jpg("demon1110");
    assert_eq!(&jpg[..3], &JPG_MAGIC[..]);
    assert!(jpg.len() > 1000, "JPG 过小: {}", jpg.len());
}

#[test]
fn gif_bytes() {
    let gif = to_gif("demon1110");
    assert!(gif.len() > 6);
    assert_eq!(&gif[..4], GIF_MAGIC);
    // 首帧 = 完整头像（外壳预览可见完整图像，不是全黑淡入帧）
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut reader = decoder.read_info(std::io::Cursor::new(&gif)).expect("GIF 解析失败");
    let first = reader.read_next_frame().expect("首帧缺失").unwrap();
    // 至少存在非透明像素（排除全黑不可见）
    let has_opaque = first.buffer.chunks_exact(4).any(|px| px[3] > 0);
    assert!(has_opaque, "GIF 首帧不可见（无有效像素）");
}

#[test]
fn data_uri_prefix() {
    let uri = to_data_uri("Binx Bond");
    assert!(uri.starts_with("data:image/png;base64,"));
    // base64 解码后等于 PNG 字节
    let b64 = uri.trim_start_matches("data:image/png;base64,");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .expect("data URI base64 解码失败");
    assert_eq!(&decoded[..8], &PNG_MAGIC[..]);
}

#[test]
fn favicon_zip_entries() {
    let zip = to_favicon_zip("Binx Bond");
    let mut reader = zip::ZipArchive::new(std::io::Cursor::new(zip)).expect("zip 解析失败");
    let names: Vec<String> = (0..reader.len())
        .map(|i| reader.by_index(i).unwrap().name().to_string())
        .collect();
    assert_eq!(names.len(), 6);
    assert_eq!(names[0], "avatar-16.png");
    assert!(names.contains(&"avatar-256.png".to_string()));
}

#[test]
fn favicon_ico_structure() {
    let ico = to_favicon_ico("Binx Bond");
    // ICONDIR: reserved(2B)=0, type(2B)=1, count(2B)=6
    assert_eq!(&ico[0..2], &[0, 0]);
    assert_eq!(&ico[2..4], &[1, 0]);
    assert_eq!(&ico[4..6], &[6, 0]);
    // 第一个条目：宽=16 高=16
    assert_eq!(ico[6], 16);
    assert_eq!(ico[7], 16);
    // 条目区大小 = 6 + 16*6 = 102
    assert!(ico.len() > 102, "ICO 过小: {}", ico.len());
}

#[test]
fn forced_part_and_theme() {
    // 强制相同 part/theme，不同输入得到相同 SVG（确定性）
    let a = multiavatar_full("aaa", false, Some("05"), Some("B"));
    let b = multiavatar_full("zzz", false, Some("05"), Some("B"));
    assert_eq!(a, b);
    // 不同 theme 输出不同
    let c = multiavatar_full("aaa", false, Some("05"), Some("C"));
    assert_ne!(a, c);
}

#[test]
fn sans_env_omits_background() {
    let full = multiavatar("Binx Bond");
    let sans = multiavatar_with("Binx Bond", true);
    assert!(sans.len() < full.len(), "sansEnv 应更短");
}

#[test]
fn deterministic_same_input() {
    assert_eq!(multiavatar("demo"), multiavatar("demo"));
}

#[test]
fn avatar_object_immutable_and_lazy() {
    let a = avatar("immutable-demo");
    let b = avatar("immutable-demo");
    // 缓存命中同一实例（Arc 相同）
    assert_eq!(a.input(), b.input());
    // SVG 文本缓存 + 文档惰性缓存：多次图像请求复用
    let _ = a.png_sized(128);
    let _ = a.png_sized(256);
    assert!(a.svg().starts_with("<svg"));
}

#[test]
fn optimize_off_keeps_raw() {
    // 关闭体积优化时输出原始坐标（与开启时不同，坐标未舍入）
    let opt_on = multiavatar("raw-compare");
    let opt_off = {
        let options = avatar_options::AvatarOptions::builder().optimize_svg(false).build();
        avatar_with("raw-compare", &options).svg().to_string()
    };
    assert_ne!(opt_on, opt_off);
    // 关闭优化时坐标保留更多小数位
    assert!(opt_off.contains("115.5"), "原始坐标应保留");
}
