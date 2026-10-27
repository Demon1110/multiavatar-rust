//! PNG 编码器：Pixmap → PNG 字节（png crate）。

use tiny_skia::Pixmap;

/// 编码为 PNG 字节。
///
/// tiny-skia Pixmap 为 premultiplied alpha 存储，此处转换为 PNG 所需的
/// straight（直通）RGBA8。
pub fn to_png(pixmap: &Pixmap) -> Vec<u8> {
    let w = pixmap.width();
    let h = pixmap.height();
    let data = pixmap.data();

    let mut rgba = Vec::with_capacity(data.len());
    for px in data.chunks_exact(4) {
        let (a, r, g, b) = (px[3], px[0], px[1], px[2]);
        if a == 255 {
            rgba.extend_from_slice(&[r, g, b, a]);
        } else if a == 0 {
            rgba.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            // premultiplied → straight: c = c_pm * 255 / a
            let af = f64::from(a);
            let r = ((f64::from(r) * 255.0 / af).round() as u32).min(255) as u8;
            let g = ((f64::from(g) * 255.0 / af).round() as u32).min(255) as u8;
            let b = ((f64::from(b) * 255.0 / af).round() as u32).min(255) as u8;
            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }

    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .expect("PNG 头写入失败");
        writer
            .write_image_data(&rgba)
            .expect("PNG 像素写入失败");
    }
    out
}

/// 便捷：Pixmap → RGBA（straight）字节。
pub fn to_rgba8(pixmap: &Pixmap) -> Vec<u8> {
    let data = pixmap.data();
    let mut rgba = Vec::with_capacity(data.len());
    for px in data.chunks_exact(4) {
        let (a, r, g, b) = (px[3], px[0], px[1], px[2]);
        if a == 255 {
            rgba.extend_from_slice(&[r, g, b, a]);
        } else if a == 0 {
            rgba.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            let af = f64::from(a);
            let r = ((f64::from(r) * 255.0 / af).round() as u32).min(255) as u8;
            let g = ((f64::from(g) * 255.0 / af).round() as u32).min(255) as u8;
            let b = ((f64::from(b) * 255.0 / af).round() as u32).min(255) as u8;
            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }
    rgba
}
