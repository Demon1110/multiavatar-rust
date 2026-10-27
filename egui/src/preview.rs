//! 预览管线：multiavatar 输出字节 → 解码 → egui 纹理；GIF 帧动画轮播。

use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};
use image::AnimationDecoder;
use std::io::Cursor;

/// PNG/JPEG 静态位图预览（解码为纹理）。
pub struct BitmapPreview {
    pub texture: TextureHandle,
    pub width: usize,
    pub height: usize,
    pub encoded_bytes: usize,
}

/// 解码 PNG 或 JPEG 字节为可显示纹理。
pub fn bitmap_preview(ctx: &Context, bytes: &[u8], kind: &str) -> Option<BitmapPreview> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let color_image = ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw());
    let texture = ctx.load_texture(
        format!("preview-{kind}-{w}x{h}"),
        color_image,
        TextureOptions::LINEAR,
    );
    Some(BitmapPreview {
        texture,
        width: w,
        height: h,
        encoded_bytes: bytes.len(),
    })
}

/// GIF 帧动画播放器（解码全部帧 + 每帧延迟，按累计时间轮播）。
pub struct GifPlayer {
    frames: Vec<ColorImage>,
    delays_ms: Vec<u32>,
    total_ms: u32,
}

impl GifPlayer {
    pub fn new(bytes: &[u8]) -> Option<Self> {
        let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(bytes)).ok()?;
        let frames = decoder.into_frames().collect_frames().ok()?;
        let mut images = Vec::with_capacity(frames.len());
        let mut delays = Vec::with_capacity(frames.len());
        let mut total = 0u32;
        for f in frames {
            let (num, den) = f.delay().numer_denom_ms();
            let ms = if den > 0 { (num as u32 * 1000) / den as u32 } else { 80 };
            let buf = f.buffer();
            let (w, h) = (buf.width() as usize, buf.height() as usize);
            images.push(ColorImage::from_rgba_unmultiplied([w, h], buf.as_raw()));
            delays.push(ms.max(1));
            total = total.saturating_add(ms.max(1));
        }
        if images.is_empty() {
            return None;
        }
        Some(GifPlayer {
            frames: images,
            delays_ms: delays,
            total_ms: total.max(1),
        })
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// 给定当前时间（秒），返回应显示的帧索引（累计延迟轮播）。
    pub fn frame_at(&self, time_secs: f64) -> usize {
        let t = ((time_secs * 1000.0) as u64) % self.total_ms as u64;
        let mut acc = 0u64;
        for (i, ms) in self.delays_ms.iter().enumerate() {
            acc += *ms as u64;
            if t < acc {
                return i;
            }
        }
        self.frames.len() - 1
    }

    pub fn image(&self, idx: usize) -> &ColorImage {
        &self.frames[idx.min(self.frames.len() - 1)]
    }
}

/// 从 ColorImage 建纹理（GIF 轮播时按需加载当前帧）。
pub fn color_image_texture(ctx: &Context, img: &ColorImage, key: &str) -> TextureHandle {
    ctx.load_texture(key, img.clone(), TextureOptions::LINEAR)
}
