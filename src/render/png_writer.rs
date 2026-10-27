//! PNG 编码器：Pixmap → PNG 字节（手写实现，零 png crate 依赖）。
//!
//! 结构：8 字节签名 + IHDR + IDAT + IEND，各 chunk 带 CRC32。
//! IDAT 使用 flate2 的 zlib 流（deflate + adler32 自动），每行 scanline 前缀
//! filter byte 0（None）。tiny-skia Pixmap 为 premultiplied alpha 存储，
//! 输出前转换为 PNG 所需的 straight（直通）RGBA8。

use crc32fast::Hasher;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use tiny_skia::Pixmap;

/// 编码为 PNG 字节。
pub fn to_png(pixmap: &Pixmap) -> Vec<u8> {
    let w = pixmap.width();
    let h = pixmap.height();
    let rgba = to_rgba8(pixmap);

    let mut out = Vec::with_capacity(rgba.len() + 64);
    // PNG 签名
    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
    // IHDR：宽/高/位深8/颜色类型6(RGBA)/压缩0/过滤0/隔行0
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    write_chunk(&mut out, b"IHDR", &ihdr);

    // IDAT：每行前缀 filter 0（None），整体 zlib 压缩
    let mut raw = Vec::with_capacity(rgba.len() + h as usize);
    for row in rgba.chunks_exact(w as usize * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(&raw).expect("zlib 写入失败");
    let idat = enc.finish().expect("zlib 收尾失败");
    write_chunk(&mut out, b"IDAT", &idat);

    // IEND
    write_chunk(&mut out, b"IEND", &[]);
    out
}

/// 便捷：Pixmap → RGBA（straight 直通）字节。
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
            // premultiplied → straight: c = c_pm * 255 / a
            let af = f64::from(a);
            let r = ((f64::from(r) * 255.0 / af).round() as u32).min(255) as u8;
            let g = ((f64::from(g) * 255.0 / af).round() as u32).min(255) as u8;
            let b = ((f64::from(b) * 255.0 / af).round() as u32).min(255) as u8;
            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }
    rgba
}

/// 写一个 PNG chunk：len(4BE) + type(4) + data + crc32(type+data)
fn write_chunk(out: &mut Vec<u8>, ctype: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(ctype);
    out.extend_from_slice(data);
    let mut hasher = Hasher::new();
    hasher.update(ctype);
    hasher.update(data);
    out.extend_from_slice(&hasher.finalize().to_be_bytes());
}

use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;

    /// PNG 魔数 + IHDR 字段 + CRC 校验（自含验证，不依赖 png crate）。
    #[test]
    fn png_structure_valid() {
        let mut pixmap = Pixmap::new(4, 4).unwrap();
        pixmap.fill(tiny_skia::Color::from_rgba8(10, 20, 30, 255));
        let png = to_png(&pixmap);
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10], "PNG 签名");
        // 第一个 chunk：IHDR
        let len = u32::from_be_bytes([png[8], png[9], png[10], png[11]]) as usize;
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(len, 13);
        let data = &png[16..16 + len];
        assert_eq!(u32::from_be_bytes([data[0], data[1], data[2], data[3]]), 4, "width");
        assert_eq!(u32::from_be_bytes([data[4], data[5], data[6], data[7]]), 4, "height");
        assert_eq!(data[8], 8, "bit depth");
        assert_eq!(data[9], 6, "color type RGBA");
        // CRC 校验（type+data）
        let mut hasher = Hasher::new();
        hasher.update(&png[12..16 + len]);
        let crc = hasher.finalize().to_be_bytes();
        assert_eq!(&png[16 + len..20 + len], &crc, "IHDR CRC");
        // 收尾 chunk：IEND（len=0 + "IEND" + crc，type 在末尾 8..4 处）
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }

    /// RGBA demultiply：全不透明像素原样；alpha=0 清零。
    #[test]
    fn rgba_demultiply() {
        let mut pixmap = Pixmap::new(1, 2).unwrap();
        {
            let d = pixmap.data_mut();
            d[..4].copy_from_slice(&[100, 100, 100, 255]); // 不透明
            d[4..].copy_from_slice(&[0, 0, 0, 0]); // 透明
        }
        let rgba = to_rgba8(&pixmap);
        assert_eq!(&rgba[..4], &[100, 100, 100, 255]);
        assert_eq!(&rgba[4..], &[0, 0, 0, 0]);
    }
}
