//! favicon 打包器：把同一头像按多个边长渲染为 PNG 并打成 zip / 单文件 ICO。
//!
//! 典型用途：网站 favicon / 应用图标包。viewBox 为正方形时各尺寸均为等比方图，
//! 非正方形视口会等比缩放并居中（边缘留透明边）。
//! ICO 格式（Vista+ PNG 内嵌）手写容器：ICONDIR 头 + 每尺寸 ICONDIRENTRY + 原始 PNG 数据。

use std::io::{Cursor, Write};

use crate::svg::{SvgParser, SvgRasterizer};

use super::png_writer::to_png;

/// 默认打包尺寸（favicon 常见规格）。
pub const DEFAULT_SIZES: [u32; 6] = [16, 32, 48, 64, 128, 256];

/// 打包多尺寸 PNG 为 zip 字节（默认尺寸 [DEFAULT_SIZES]）。
pub fn to_zip(svg: &str, sizes: &[u32]) -> Vec<u8> {
    let sizes = if sizes.is_empty() { &DEFAULT_SIZES } else { sizes };
    let doc = SvgParser::parse(svg);
    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for size in sizes {
            let pixmap = SvgRasterizer::rasterize(&doc, *size).expect("favicon PNG 光栅化失败");
            let png = to_png(&pixmap);
            zip.start_file(format!("avatar-{}.png", size), options)
                .expect("zip 条目创建失败");
            zip.write_all(&png).expect("zip 写入失败");
        }
        zip.finish().expect("zip 收尾失败");
    }
    buf
}

/// 打包多尺寸 PNG 为单文件 ICO 字节（默认尺寸 [DEFAULT_SIZES]）。
///
/// 结构与 PNG 内嵌 ICO（Vista+）一致：ICONDIR + ICONDIRENTRY×N + PNG 数据，
/// 浏览器与 Windows 均直接支持；大于 255 的尺寸按规范以 0 表示。
pub fn to_ico(svg: &str, sizes: &[u32]) -> Vec<u8> {
    let sizes = if sizes.is_empty() { &DEFAULT_SIZES } else { sizes };
    let doc = SvgParser::parse(svg);
    // 先渲染全部 PNG，确定每个条目的字节数与偏移
    let pngs: Vec<Vec<u8>> = sizes
        .iter()
        .map(|size| {
            let pixmap = SvgRasterizer::rasterize(&doc, *size).expect("ICO PNG 光栅化失败");
            to_png(&pixmap)
        })
        .collect();

    let mut offset: u32 = 6 + 16 * sizes.len() as u32; // ICONDIR(6B) + ICONDIRENTRY×N(16B)
    let mut bos = Vec::with_capacity(offset as usize + 64 * 1024);
    // ICONDIR：reserved=0, type=1(icon), count
    write_le16(&mut bos, 0);
    write_le16(&mut bos, 1);
    write_le16(&mut bos, sizes.len() as u16);
    for (i, size) in sizes.iter().enumerate() {
        let s = *size;
        bos.push(if s >= 256 { 0 } else { s as u8 }); // width（0 表示 256）
        bos.push(if s >= 256 { 0 } else { s as u8 }); // height（0 表示 256）
        bos.push(0); // colorCount
        bos.push(0); // reserved
        write_le16(&mut bos, 1); // planes
        write_le16(&mut bos, 32); // bitCount
        write_le32(&mut bos, pngs[i].len() as u32); // bytesInRes
        write_le32(&mut bos, offset); // imageOffset
        offset += pngs[i].len() as u32;
    }
    for png in &pngs {
        bos.extend_from_slice(png);
    }
    bos
}

fn write_le16(out: &mut Vec<u8>, v: u16) {
    out.push((v & 0xFF) as u8);
    out.push((v >> 8) as u8);
}

fn write_le32(out: &mut Vec<u8>, v: u32) {
    out.push((v & 0xFF) as u8);
    out.push((v >> 8) as u8);
    out.push((v >> 16) as u8);
    out.push((v >> 24) as u8);
}
