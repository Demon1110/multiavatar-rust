//! JPEG 编码器：RGB 字节 → JPEG 字节（jpeg-encoder crate）。
//!
//! JPEG 不支持透明通道，带 alpha 的图像应先铺到白底再转 RGB 编码
//! （见 [crate::render::JpgAvatarRenderer]）。

use jpeg_encoder::{ColorType, Encoder};

/// 编码为 JPEG 字节（质量 75，与 Java ImageIO 默认一致）。
pub fn to_jpg_rgb(rgb: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let encoder = Encoder::new(&mut out, 75);
    encoder
        .encode(rgb, width as u16, height as u16, ColorType::Rgb)
        .expect("JPEG 编码失败");
    out
}
