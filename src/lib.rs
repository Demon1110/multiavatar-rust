//! # multiavatar
//!
//! 多进制头像生成器（multiavatar.js → Java → Rust 全功能移植，零运行时依赖以外的核心逻辑）。
//!
//! 基于输入字符串的 SHA-256 哈希派生 12 位随机种子，映射 12 个环境/部件槽位，
//! 从内置 12×16×3 主题部件库拼装成 SVG 头像，并可渲染为 PNG / JPEG / GIF /
//! favicon zip / favicon.ico / PNG data URI。
//!
//! 设计：数据层（gzip 内嵌部件库）+ 核心层（哈希/映射/拼装/体积优化）+ SVG 领域层
//! （对象模型/解析/光栅化）+ 渲染策略层（策略模式，四格式编码器）+ 产物层
//! （不可变 Avatar + 惰性解析缓存）+ 门面（全局 LRU 缓存 + 全量 API）。
//!
//! ## 快速上手
//!
//! ```rust
//! use multiavatar::*;
//!
//! let svg = multiavatar("demon1110");
//! assert!(!svg.is_empty());
//!
//! let png = to_png("demon1110"); // 256×256 PNG 字节
//! assert!(png.len() > 8); // PNG 魔数 8 字节
//!
//! let uri = to_data_uri("demon1110");
//! assert!(uri.starts_with("data:image/png;base64,"));
//! ```

pub mod avatar;
pub mod avatar_cache;
pub mod avatar_options;
pub mod core;
pub mod data;
pub mod render;
pub mod svg;
pub mod util;

use std::io;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use avatar::Avatar;
use avatar_cache::{AvatarCache, CacheKey};
use avatar_options::AvatarOptions;
use core::svg_composer::SvgComposer;
use render::AvatarFormat;
use render::Renderers;
use tiny_skia::Pixmap;

/// 门面入口（等价 Java `Multiavatar`）。
pub struct Multiavatar;

/// 单例拼装器（部件库首次使用时惰性加载，gzip 数据嵌入二进制）。
fn composer() -> &'static SvgComposer {
    static COMPOSER: OnceLock<SvgComposer> = OnceLock::new();
    COMPOSER.get_or_init(|| SvgComposer::with_view_box("0 0 231 231"))
}

// ---------------------------------------------------------------------------
// 经典 API（多参数，对齐官方 JS / Java）
// ---------------------------------------------------------------------------

/// 生成头像 SVG（自动派生随机种子）。
///
/// 空输入返回空串。
pub fn multiavatar(input: &str) -> String {
    multiavatar_full(input, false, None, None)
}

/// 生成头像 SVG，可去掉环境部件（背景圆）。
pub fn multiavatar_with(input: &str, sans_env: bool) -> String {
    multiavatar_full(input, sans_env, None, None)
}

/// 生成头像 SVG，可强制角色与颜色主题。
///
/// - `part`：初始角色编号（"00"-"15"），None 表示自动
/// - `theme`：颜色主题（"A"/"B"/"C"），None 表示自动
pub fn multiavatar_full(
    input: &str,
    sans_env: bool,
    part: Option<&str>,
    theme: Option<&str>,
) -> String {
    let options = AvatarOptions::builder().sans_env(sans_env).build();
    let options = match part {
        Some(p) => options_part(&options, p),
        None => options,
    };
    let options = match theme {
        Some(t) => AvatarOptions::builder()
            .sans_env(options.sans_env())
            .part_owned(options.part().map(str::to_string))
            .theme(t)
            .build(),
        None => options,
    };
    avatar_with(input, &options).svg().to_string()
}

// ---------------------------------------------------------------------------
// 产物 API（不可变 Avatar + 全局 LRU 缓存）
// ---------------------------------------------------------------------------

/// 获取头像产物（缓存命中直接返回；未命中生成后入缓存）。
///
/// 产物不可变：SVG 文本缓存 + 解析后文档惰性缓存，多尺寸多格式复用。
pub fn avatar(input: &str) -> Arc<Avatar> {
    avatar_with(input, &AvatarOptions::defaults())
}

/// 获取头像产物（指定选项）。
pub fn avatar_with(input: &str, options: &AvatarOptions) -> Arc<Avatar> {
    let key = CacheKey::new(
        input,
        options.sans_env(),
        options.part(),
        options.theme(),
        options.svg_width(),
        options.svg_height(),
        options.optimize_svg(),
    );
    if let Some(hit) = AvatarCache::get(&key) {
        return hit;
    }
    let mut svg = composer().compose(
        input,
        options.sans_env(),
        options.part(),
        options.theme(),
    );
    if options.optimize_svg() {
        svg = crate::core::svg_optimizer::optimize(&svg);
    }
    let avatar = Arc::new(Avatar::new(input.to_string(), options.clone(), svg));
    AvatarCache::put(key, avatar.clone());
    avatar
}

/// 当前缓存条目数。
pub fn cache_size() -> usize {
    AvatarCache::size()
}

/// 清空缓存。
pub fn clear_cache() {
    AvatarCache::clear();
}

// ---------------------------------------------------------------------------
// 渲染 API（便捷门面，内部走产物 + 策略层）
// ---------------------------------------------------------------------------

/// 光栅化图像（默认尺寸 256，透明背景）；空输入返回 None。
pub fn to_image(input: &str) -> Option<Pixmap> {
    to_image_sized(input, AvatarOptions::defaults().size())
}

/// 光栅化图像（指定边长，透明背景）。
pub fn to_image_sized(input: &str, size: u32) -> Option<Pixmap> {
    let options = AvatarOptions::builder().size(size).build();
    to_image_with(input, &options)
}

/// 光栅化图像（指定选项）。
pub fn to_image_with(input: &str, options: &AvatarOptions) -> Option<Pixmap> {
    // 显式传尺寸：缓存键不含 size，命中旧 Avatar 时 options.size() 可能是首次调用的尺寸
    avatar_with(input, options).to_image_sized(options.size())
}

/// PNG 字节（默认尺寸 256）；空输入返回空数组。
pub fn to_png(input: &str) -> Vec<u8> {
    to_png_sized(input, AvatarOptions::defaults().size())
}

/// PNG 字节（指定边长）。
pub fn to_png_sized(input: &str, size: u32) -> Vec<u8> {
    let options = AvatarOptions::builder().size(size).build();
    to_png_with(input, &options)
}

/// PNG 字节（指定选项）。
pub fn to_png_with(input: &str, options: &AvatarOptions) -> Vec<u8> {
    avatar_with(input, options).png_sized(options.size())
}

// ---------------------------------------------------------------------------
// 超采样抗锯齿（SSAA）渲染 API —— 边缘更平滑，消除小尺寸放大锯齿
// ---------------------------------------------------------------------------

/// 光栅化图像（**4x 超采样抗锯齿**）：内部按 `size×4` 分辨率渲染后再
/// 双线性缩小到 `size`，曲线边缘显著更平滑；透明背景。
///
/// 等效 SSAA 4x；对 256px 这类小尺寸输出尤其有效（直接放大观看不再有
/// 明显锯齿）。空输入返回 None。
pub fn to_image_smooth(input: &str, size: u32) -> Option<Pixmap> {
    to_image_smooth_with(input, size, 4)
}

/// 光栅化图像（**可调超采样因子**）：`factor` 为内部渲染倍率，
/// `factor = 1` 等价普通渲染；`factor = 4` 是平滑/成本的良好折中。
pub fn to_image_smooth_with(input: &str, size: u32, factor: u32) -> Option<Pixmap> {
    if size == 0 || factor <= 1 {
        return to_image_sized(input, size);
    }
    let big = to_image_sized(input, size * factor)?;
    render::downscale::bilinear(&big, size)
}

/// PNG 字节（**4x 超采样抗锯齿**，默认边长 256）。
pub fn to_png_smooth(input: &str) -> Vec<u8> {
    to_png_smooth_sized(input, AvatarOptions::defaults().size())
}

/// PNG 字节（4x 超采样抗锯齿，指定边长）。
pub fn to_png_smooth_sized(input: &str, size: u32) -> Vec<u8> {
    to_image_smooth(input, size)
        .map(|p| render::png_writer::to_png(&p))
        .unwrap_or_default()
}

/// JPEG 字节（**4x 超采样抗锯齿**，白底，默认边长 256）。
pub fn to_jpg_smooth(input: &str) -> Vec<u8> {
    to_jpg_smooth_sized(input, AvatarOptions::defaults().size())
}

/// JPEG 字节（4x 超采样抗锯齿，白底，指定边长）。
pub fn to_jpg_smooth_sized(input: &str, size: u32) -> Vec<u8> {
    let Some(pixmap) = to_image_smooth(input, size) else {
        return Vec::new();
    };
    // 透明背景合成到白底（premultiplied 空间：白底即 alpha 权重）
    let data = pixmap.data();
    let mut rgb = Vec::with_capacity((size * size) as usize * 3);
    for px in data.chunks(4) {
        let a = px[3];
        if a == 255 {
            rgb.extend_from_slice(&px[..3]);
        } else {
            // 白底合成：c_out = c_pre + 255*(1-a)；premultiplied 通道 + 白
            rgb.push(((px[0] as u32 + 255 * (255 - a as u32)).min(255)) as u8);
            rgb.push(((px[1] as u32 + 255 * (255 - a as u32)).min(255)) as u8);
            rgb.push(((px[2] as u32 + 255 * (255 - a as u32)).min(255)) as u8);
        }
    }
    render::jpg_writer::to_jpg_rgb(&rgb, size, size)
}

/// JPEG 字节（默认尺寸 256，白底）；空输入返回空数组。
pub fn to_jpg(input: &str) -> Vec<u8> {
    to_jpg_sized(input, AvatarOptions::defaults().size())
}

/// JPEG 字节（指定边长，白底）。
pub fn to_jpg_sized(input: &str, size: u32) -> Vec<u8> {
    let options = AvatarOptions::builder().size(size).build();
    to_jpg_with(input, &options)
}

/// JPEG 字节（指定选项，白底）。
pub fn to_jpg_with(input: &str, options: &AvatarOptions) -> Vec<u8> {
    avatar_with(input, options).jpg_sized(options.size())
}

/// GIF 动画字节（默认尺寸 256，部件逐帧淡入，无限循环）；空输入返回空数组。
pub fn to_gif(input: &str) -> Vec<u8> {
    to_gif_sized(input, AvatarOptions::defaults().size())
}

/// GIF 动画字节（指定边长）。
pub fn to_gif_sized(input: &str, size: u32) -> Vec<u8> {
    let options = AvatarOptions::builder().size(size).build();
    to_gif_with(input, &options)
}

/// GIF 动画字节（指定选项）。
pub fn to_gif_with(input: &str, options: &AvatarOptions) -> Vec<u8> {
    let a = avatar_with(input, options);
    if a.is_empty() {
        return Vec::new();
    }
    Renderers::create(AvatarFormat::Gif).render(a.svg(), options.size())
}

/// PNG data URI（默认尺寸 256）：`data:image/png;base64,...`；空输入返回空串。
pub fn to_data_uri(input: &str) -> String {
    to_data_uri_sized(input, AvatarOptions::defaults().size())
}

/// PNG data URI（指定边长）。
pub fn to_data_uri_sized(input: &str, size: u32) -> String {
    let options = AvatarOptions::builder().size(size).build();
    to_data_uri_with(input, &options)
}

/// PNG data URI（指定选项）。
pub fn to_data_uri_with(input: &str, options: &AvatarOptions) -> String {
    avatar_with(input, options).data_uri_sized(options.size())
}

/// favicon 多尺寸 PNG 打包为 zip 字节（默认尺寸 16/32/48/64/128/256）。
pub fn to_favicon_zip(input: &str) -> Vec<u8> {
    to_favicon_zip_sized(input, &[])
}

/// favicon 多尺寸 PNG 打包为 zip 字节（指定尺寸集合；空集合用默认）。
pub fn to_favicon_zip_sized(input: &str, sizes: &[u32]) -> Vec<u8> {
    let svg = multiavatar(input);
    if svg.is_empty() {
        return Vec::new();
    }
    render::favicons::to_zip(&svg, sizes)
}

/// favicon 单文件 ICO 字节（默认尺寸 16/32/48/64/128/256，PNG 内嵌，Vista+ 直接支持）。
pub fn to_favicon_ico(input: &str) -> Vec<u8> {
    to_favicon_ico_sized(input, &[])
}

/// favicon 单文件 ICO 字节（指定尺寸集合；空集合用默认）。
pub fn to_favicon_ico_sized(input: &str, sizes: &[u32]) -> Vec<u8> {
    let svg = multiavatar(input);
    if svg.is_empty() {
        return Vec::new();
    }
    render::favicons::to_ico(&svg, sizes)
}

/// 按格式渲染（统一策略入口）。
///
/// 等价 Java `render(input, format, options)`：Svg → 文本字节；
/// Png/Jpg/Gif → 对应格式编码。
pub fn render(input: &str, format: AvatarFormat, options: &AvatarOptions) -> Vec<u8> {
    let a = avatar_with(input, options);
    if a.is_empty() {
        return Vec::new();
    }
    match format {
        AvatarFormat::Svg => a.svg().as_bytes().to_vec(),
        AvatarFormat::Png => Renderers::create(AvatarFormat::Png).render(a.svg(), options.size()),
        AvatarFormat::Jpg => Renderers::create(AvatarFormat::Jpg).render(a.svg(), options.size()),
        AvatarFormat::Gif => Renderers::create(AvatarFormat::Gif).render(a.svg(), options.size()),
    }
}

// ---------------------------------------------------------------------------
// 文件输出 API
// ---------------------------------------------------------------------------

/// PNG 写文件（默认尺寸 256）。
pub fn write_png(input: &str, path: impl AsRef<Path>) -> io::Result<()> {
    write_png_sized(input, AvatarOptions::defaults().size(), path)
}

/// PNG 写文件（指定边长）。
pub fn write_png_sized(input: &str, size: u32, path: impl AsRef<Path>) -> io::Result<()> {
    let bytes = to_png_sized(input, size);
    std::fs::write(path, bytes)
}

/// favicon zip 写文件（默认尺寸）。
pub fn write_favicon_zip(input: &str, path: impl AsRef<Path>) -> io::Result<()> {
    let bytes = to_favicon_zip(input);
    std::fs::write(path, bytes)
}

/// favicon ico 写文件（默认尺寸）。
pub fn write_favicon_ico(input: &str, path: impl AsRef<Path>) -> io::Result<()> {
    let bytes = to_favicon_ico(input);
    std::fs::write(path, bytes)
}

// ---------------------------------------------------------------------------
// 内部辅助
// ---------------------------------------------------------------------------

fn options_part(options: &AvatarOptions, part: &str) -> AvatarOptions {
    AvatarOptions::builder()
        .sans_env(options.sans_env())
        .part(part)
        .theme_owned(options.theme().map(str::to_string))
        .size(options.size())
        .svg_size(options.svg_width(), options.svg_height())
        .format(options.format())
        .optimize_svg(options.optimize_svg())
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 缓存键不含 size/format：同一产物的多尺寸多格式渲染不新增条目。
    /// （全局缓存断言放在 lib 单测：并行时其他单测不写门面缓存，无噪声）
    #[test]
    fn lru_key_ignores_size_and_format() {
        clear_cache();
        let u = "cache-unit-input";
        let _ = to_png_sized(u, 64);
        let base = cache_size();
        assert!(base >= 1, "首次生成应入缓存");
        let _ = to_png_sized(u, 128);
        let _ = to_png_sized(u, 512);
        let _ = to_gif_sized(u, 256);
        let _ = to_jpg_sized(u, 256);
        assert_eq!(cache_size(), base, "size/format 不应新增缓存键");
        clear_cache();
    }

    /// 空输入各 API 行为。
    #[test]
    fn empty_input_facade() {
        assert_eq!(multiavatar(""), "");
        assert!(to_png("").is_empty());
        assert!(to_jpg("").is_empty());
        assert!(to_gif("").is_empty());
        assert_eq!(to_data_uri(""), "");
        assert!(to_image("").is_none());
        assert!(to_favicon_zip("").is_empty());
        assert!(to_favicon_ico("").is_empty());
        assert!(to_image_smooth("", 256).is_none());
        assert!(to_png_smooth("").is_empty());
        assert!(to_jpg_smooth("").is_empty());
    }

    /// SSAA：输出尺寸正确、与直接渲染结果不同（平滑化生效）、空输入安全。
    #[test]
    fn smooth_ssaa_smaller_and_different() {
        let input = "smooth-unit-input";
        let direct = to_image_sized(input, 64).unwrap();
        let smooth = to_image_smooth(input, 64).unwrap();
        assert_eq!(smooth.width(), 64);
        assert_eq!(smooth.height(), 64);
        // 边缘区域应有过渡差异（直接渲染与超采样后像素不同）
        assert_ne!(smooth.data(), direct.data(), "SSAA 输出应不同于直接渲染");
        // factor=1 等价直接渲染
        let same = to_image_smooth_with(input, 64, 1).unwrap();
        assert_eq!(same.data(), direct.data());
        // PNG/JPEG 门面正常编码
        assert!(!to_png_smooth_sized(input, 64).is_empty());
        assert!(!to_jpg_smooth_sized(input, 64).is_empty());
    }
}
