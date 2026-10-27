//! GIF 编码器：多帧 Pixmap → GIF 字节（gif crate）。
//!
//! 支持每帧延迟与 Netscape 无限循环；帧图带透明通道（GIF 自动处理为透明索引）。
//! 用于部件逐帧淡入的动画头像（[crate::render::GifAvatarRenderer]）。

use std::collections::HashMap;

use gif::{DisposalMethod, Encoder, Frame, Repeat};
use tiny_skia::Pixmap;

/// 编码为 GIF 字节（无限循环）。
///
/// - `frames`：帧序列（非空）
/// - `delay_cs`：每帧停留厘秒（1/100s，最小 1）
pub fn to_gif(frames: &[Pixmap], delay_cs: u16) -> Vec<u8> {
    assert!(!frames.is_empty(), "frames 不能为空");
    let w = frames[0].width() as u16;
    let h = frames[0].height() as u16;

    // 全局调色板用第一帧的（帧级 palette 会覆盖）
    let (_, global_palette) = collect_palette(&frames[0]);
    let mut out = Vec::new();
    {
        let mut encoder = Encoder::new(&mut out, w, h, &global_palette).expect("GIF 编码器创建失败");
        encoder.set_repeat(Repeat::Infinite).expect("GIF 循环设置失败");
        for pixmap in frames {
            let frame = pixmap_to_frame(pixmap, delay_cs);
            encoder.write_frame(&frame).expect("GIF 帧写入失败");
        }
    }
    out
}

/// Pixmap → GIF Frame（自动构建 ≤256 色调色板 + 透明索引）。
fn pixmap_to_frame(pixmap: &Pixmap, delay_cs: u16) -> Frame<'_> {
    let w = pixmap.width() as u16;
    let h = pixmap.height() as u16;
    let (indices, palette) = collect_palette(pixmap);

    let mut frame = Frame::from_palette_pixels(w, h, indices, palette.to_vec(), Some(0));
    frame.delay = delay_cs.max(1);
    frame.dispose = DisposalMethod::Background; // 等价 Java restoreToBackgroundColor
    frame
}

/// 收集帧的唯一色并建立索引缓冲。
///
/// 透明像素（a==0）统一映射到索引 0；palette 索引 0 为透明占位（RGB 000）。
/// 返回 (索引缓冲, 256×3 调色板)。
fn collect_palette(pixmap: &Pixmap) -> (Vec<u8>, [u8; 256 * 3]) {
    let w = pixmap.width() as usize;
    let h = pixmap.height() as usize;
    let data = pixmap.data();

    let mut indices = vec![0u8; w * h];
    let mut palette = [0u8; 256 * 3];
    let mut map: HashMap<[u8; 3], u8> = HashMap::new();
    let mut next: u8 = 1; // 索引 0 留给透明

    for (idx, px) in data.chunks_exact(4).enumerate() {
        let (a, r, g, b) = (px[3], px[0], px[1], px[2]);
        if a == 0 {
            indices[idx] = 0;
            continue;
        }
        let (r, g, b) = if a == 255 {
            (r, g, b)
        } else {
            // premultiplied → straight
            let af = f64::from(a);
            (
                ((f64::from(r) * 255.0 / af).round() as u32).min(255) as u8,
                ((f64::from(g) * 255.0 / af).round() as u32).min(255) as u8,
                ((f64::from(b) * 255.0 / af).round() as u32).min(255) as u8,
            )
        };
        let key = [r, g, b];
        let ci = *map.entry(key).or_insert_with(|| {
            let c = next;
            next = next.saturating_add(1);
            c
        });
        indices[idx] = ci;
    }
    for (color, idx) in map.iter() {
        let off = (*idx as usize) * 3;
        palette[off..off + 3].copy_from_slice(color);
    }
    (indices, palette)
}

/// 便捷：编码器写入辅助（返回字节）。
pub fn to_gif_bytes(frames: &[Pixmap], delay_ms: u32) -> Vec<u8> {
    let delay_cs = (delay_ms.max(1) / 10).max(1) as u16;
    to_gif(frames, delay_cs)
}
