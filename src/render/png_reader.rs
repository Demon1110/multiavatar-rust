//! PNG 解码器：PNG 字节 → 直通（straight）RGBA8 图像（手写实现，零 png crate 依赖）。
//!
//! 覆盖：颜色类型 0（灰度）/2（RGB）/3（调色板）/4（灰度+alpha）/6（RGBA），
//! 位深 1/2/4/8/16，tRNS 透明（调色板 / 灰度 / RGB 三种写法），非隔行扫描。
//! 流程：chunk 遍历（含 CRC32 校验）→ zlib 解压 IDAT → 逐行反过滤
//! （None/Sub/Up/Average/Paeth）→ 逐像素展开为 RGBA8。
//! 16 位样本取高字节（等价 `value >> 8`）；低位深按 `2^d - 1` 等比例拉伸到 8 位。
//!
//! 不做 Adam7 隔行：图标/头像输入几乎全为非隔行，遇到隔行文件明确报错，
//! 而不是产出静默错位的图像。

use std::fmt;
use std::io::Read;

use flate2::read::ZlibDecoder;

/// PNG 文件签名（8 字节）。
const SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// 解码像素总量上限（64 MPix ≈ 256 MB RGBA）：挡住解压炸弹，也远超图标所需。
pub const MAX_PIXELS: u64 = 64 * 1024 * 1024;

/// PNG 解码错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngError {
    /// 不是 PNG：签名不符（空数据也归此类）。
    NotPng,
    /// 数据在 chunk 长度/CRC 处截止（文件被截断）。
    Truncated,
    /// chunk CRC32 校验失败（数据损坏）。
    CrcMismatch { chunk: String },
    /// IHDR 字段非法（尺寸为 0、位深/颜色类型组合不存在、压缩方法非 0 等）。
    InvalidHeader(String),
    /// 格式合法但本实现不支持（目前仅隔行扫描）。
    Unsupported(String),
    /// IDAT 内容损坏：解压长度不符、调色板越界、过滤类型非法等。
    InvalidData(String),
    /// 像素总量超过 [`MAX_PIXELS`]。
    TooLarge { width: u32, height: u32 },
}

impl fmt::Display for PngError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PngError::NotPng => write!(f, "不是 PNG 数据（签名不符）"),
            PngError::Truncated => write!(f, "PNG 数据被截断"),
            PngError::CrcMismatch { chunk } => write!(f, "chunk {chunk} 的 CRC32 校验失败"),
            PngError::InvalidHeader(reason) => write!(f, "PNG 头部非法：{reason}"),
            PngError::Unsupported(reason) => write!(f, "不支持的 PNG：{reason}"),
            PngError::InvalidData(reason) => write!(f, "PNG 数据非法：{reason}"),
            PngError::TooLarge { width, height } => write!(
                f,
                "PNG 尺寸过大：{width}×{height} 超过 {MAX_PIXELS} 像素上限"
            ),
        }
    }
}

impl std::error::Error for PngError {}

/// 解码后的图像：直通（非预乘）RGBA8，行优先。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl RgbaImage {
    /// 由直通 RGBA8 像素构造；`data` 长度必须为 `width * height * 4`。
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        debug_assert_eq!(data.len(), width as usize * height as usize * 4);
        Self {
            width,
            height,
            data,
        }
    }

    /// 宽度（像素）。
    pub fn width(&self) -> u32 {
        self.width
    }

    /// 高度（像素）。
    pub fn height(&self) -> u32 {
        self.height
    }

    /// RGBA8 像素（长度 `width * height * 4`）。
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// 取走像素数据。
    pub fn into_data(self) -> Vec<u8> {
        self.data
    }
}

/// IHDR 解析结果（已校验过的字段）。
struct Header {
    width: usize,
    height: usize,
    bit_depth: u8,
    color_type: u8,
    channels: usize,
}

/// 解码 PNG 字节为 RGBA8 图像。
pub fn decode(bytes: &[u8]) -> Result<RgbaImage, PngError> {
    if bytes.len() < SIGNATURE.len() || bytes[..SIGNATURE.len()] != SIGNATURE {
        return Err(PngError::NotPng);
    }

    let mut pos = SIGNATURE.len();
    let mut header: Option<Header> = None;
    let mut palette: Option<Vec<[u8; 3]>> = None;
    let mut trns: Option<Vec<u8>> = None;
    let mut idat: Vec<u8> = Vec::new();
    let mut seen_idat = false;
    let mut seen_iend = false;

    while pos + 8 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
            as usize;
        let ctype = &bytes[pos + 4..pos + 8];
        let data_start = pos + 8;
        let data_end = data_start.checked_add(len).ok_or(PngError::Truncated)?;
        let crc_end = data_end.checked_add(4).ok_or(PngError::Truncated)?;
        if crc_end > bytes.len() {
            return Err(PngError::Truncated);
        }
        let data = &bytes[data_start..data_end];

        // 每个 chunk 都校验 CRC32（type + data），损坏即拒，避免产出错位图像。
        let expected = u32::from_be_bytes([
            bytes[data_end],
            bytes[data_end + 1],
            bytes[data_end + 2],
            bytes[data_end + 3],
        ]);
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(ctype);
        hasher.update(data);
        if hasher.finalize() != expected {
            return Err(PngError::CrcMismatch {
                chunk: String::from_utf8_lossy(ctype).into_owned(),
            });
        }

        match ctype {
            b"IHDR" => {
                if header.is_some() {
                    return Err(PngError::InvalidData("重复的 IHDR".into()));
                }
                header = Some(parse_header(data)?);
            }
            b"PLTE" => {
                if header.as_ref().is_none_or(|h| h.color_type != 3) {
                    return Err(PngError::InvalidData("PLTE 仅用于索引色 PNG".into()));
                }
                if !len.is_multiple_of(3) {
                    return Err(PngError::InvalidData(format!(
                        "PLTE 长度 {len} 不是 3 的倍数"
                    )));
                }
                palette = Some(data.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect());
            }
            b"tRNS" => trns = Some(data.to_vec()),
            b"IDAT" => {
                seen_idat = true;
                idat.extend_from_slice(data);
            }
            b"IEND" => {
                seen_iend = true;
                break;
            }
            _ => {
                // 首字母大写 = 关键 chunk，未知即无法正确解码；小写为辅助 chunk，忽略。
                if ctype[0].is_ascii_uppercase() {
                    return Err(PngError::Unsupported(format!(
                        "未知关键 chunk {}",
                        String::from_utf8_lossy(ctype)
                    )));
                }
            }
        }
        pos = crc_end;
    }

    let header = header.ok_or_else(|| PngError::InvalidHeader("缺少 IHDR".into()))?;
    if !seen_idat {
        return Err(PngError::InvalidData("缺少 IDAT".into()));
    }
    if !seen_iend {
        return Err(PngError::Truncated);
    }

    let row_bytes = (header.width * header.channels * header.bit_depth as usize).div_ceil(8);
    let expected = header.height * (1 + row_bytes);
    let mut raw = Vec::with_capacity(expected);
    let mut zlib = ZlibDecoder::new(idat.as_slice());
    // 多读 1 字节即可判断「解压数据比 IHDR 声明的多」
    (&mut zlib)
        .take(expected as u64 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| PngError::InvalidData(format!("zlib 解压失败：{e}")))?;
    if raw.len() != expected {
        return Err(PngError::InvalidData(format!(
            "IDAT 解压后长度为 {}，按 IHDR 应为 {}",
            raw.len(),
            expected
        )));
    }

    let bpp = (header.channels * header.bit_depth as usize / 8).max(1);
    let filtered = unfilter(&raw, header.height, row_bytes, bpp)?;
    let palette = if header.color_type == 3 {
        Some(palette.ok_or_else(|| PngError::InvalidData("索引色 PNG 缺少 PLTE".into()))?)
    } else {
        None
    };
    let data = expand_to_rgba(
        &filtered,
        &header,
        row_bytes,
        palette.as_deref(),
        trns.as_deref(),
    )?;
    Ok(RgbaImage::new(
        header.width as u32,
        header.height as u32,
        data,
    ))
}

/// 解析 IHDR（13 字节）。
fn parse_header(data: &[u8]) -> Result<Header, PngError> {
    if data.len() != 13 {
        return Err(PngError::InvalidHeader(format!(
            "IHDR 长度应为 13，实际 {}",
            data.len()
        )));
    }
    let width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let bit_depth = data[8];
    let color_type = data[9];
    if width == 0 || height == 0 {
        return Err(PngError::InvalidHeader("宽高不能为 0".into()));
    }
    if data[10] != 0 {
        return Err(PngError::InvalidHeader(
            "压缩方法仅支持 0（deflate）".into(),
        ));
    }
    if data[11] != 0 {
        return Err(PngError::InvalidHeader("过滤方法仅支持 0".into()));
    }
    if data[12] != 0 {
        return Err(PngError::Unsupported(
            "隔行扫描（Adam7）PNG 暂不支持，请另存为非隔行".into(),
        ));
    }
    let channels = match color_type {
        0 => 1, // 灰度
        2 => 3, // RGB
        3 => 1, // 索引
        4 => 2, // 灰度 + alpha
        6 => 4, // RGBA
        other => return Err(PngError::InvalidHeader(format!("颜色类型 {other} 不存在"))),
    };
    let depth_ok = match color_type {
        0 => matches!(bit_depth, 1 | 2 | 4 | 8 | 16),
        3 => matches!(bit_depth, 1 | 2 | 4 | 8),
        _ => matches!(bit_depth, 8 | 16),
    };
    if !depth_ok {
        return Err(PngError::InvalidHeader(format!(
            "颜色类型 {color_type} 不支持位深 {bit_depth}"
        )));
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(PngError::TooLarge { width, height });
    }
    Ok(Header {
        width: width as usize,
        height: height as usize,
        bit_depth,
        color_type,
        channels,
    })
}

/// 逐行反过滤（5 种过滤类型），返回未过滤的原始扫描线数据。
fn unfilter(raw: &[u8], height: usize, row_bytes: usize, bpp: usize) -> Result<Vec<u8>, PngError> {
    let mut out = vec![0u8; row_bytes * height];
    for y in 0..height {
        let base = y * (row_bytes + 1);
        let filter = raw[base];
        let line = &raw[base + 1..base + 1 + row_bytes];
        let out_base = y * row_bytes;
        for i in 0..row_bytes {
            let x = i32::from(line[i]);
            // left / up / upper-left 三个邻像素（越界取 0）
            let a = if i >= bpp {
                i32::from(out[out_base + i - bpp])
            } else {
                0
            };
            let b = if y > 0 {
                i32::from(out[out_base - row_bytes + i])
            } else {
                0
            };
            let c = if y > 0 && i >= bpp {
                i32::from(out[out_base - row_bytes + i - bpp])
            } else {
                0
            };
            let v = match filter {
                0 => x,
                1 => x + a,
                2 => x + b,
                3 => x + (a + b) / 2,
                4 => x + paeth(a, b, c),
                other => {
                    return Err(PngError::InvalidData(format!(
                        "第 {y} 行过滤类型 {other} 不存在"
                    )))
                }
            };
            out[out_base + i] = (v & 0xFF) as u8;
        }
    }
    Ok(out)
}

/// PNG Paeth 预测器。
fn paeth(a: i32, b: i32, c: i32) -> i32 {
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// 取一行里第 `index` 个样本的原始值（0..2^depth-1）。
fn sample_at(row: &[u8], index: usize, depth: u8) -> u16 {
    match depth {
        8 => u16::from(row[index]),
        16 => {
            let o = index * 2;
            u16::from_be_bytes([row[o], row[o + 1]])
        }
        bits => {
            let bits = bits as usize;
            let bit_pos = index * bits;
            let byte = row[bit_pos / 8];
            let shift = 8 - bits - (bit_pos % 8);
            ((byte >> shift) & ((1u8 << bits) - 1)) as u16
        }
    }
}

/// 原始样本 → 8 位：16 位取高字节，低位深按最大值等比例拉伸（四舍五入）。
fn scale8(v: u16, depth: u8) -> u8 {
    match depth {
        8 => v as u8,
        16 => (v >> 8) as u8,
        bits => {
            let max = (1u32 << bits) - 1;
            ((u32::from(v) * 255 + max / 2) / max) as u8
        }
    }
}

/// 反过滤后的扫描线 → 直通 RGBA8。
fn expand_to_rgba(
    filtered: &[u8],
    header: &Header,
    row_bytes: usize,
    palette: Option<&[[u8; 3]]>,
    trns: Option<&[u8]>,
) -> Result<Vec<u8>, PngError> {
    let depth = header.bit_depth;
    let ct = header.color_type;
    let (width, height) = (header.width, header.height);

    // tRNS 语义按颜色类型校验：灰度 2 字节、RGB 6 字节、索引每项 1 字节；
    // 4/6 自带 alpha，tRNS 无意义（忽略）。
    if let Some(t) = trns {
        match ct {
            0 if t.len() != 2 => {
                return Err(PngError::InvalidData(format!(
                    "灰度 tRNS 长度应为 2，实际 {}",
                    t.len()
                )))
            }
            2 if t.len() != 6 => {
                return Err(PngError::InvalidData(format!(
                    "RGB tRNS 长度应为 6，实际 {}",
                    t.len()
                )))
            }
            3 => {
                if let Some(p) = palette {
                    if t.len() > p.len() {
                        return Err(PngError::InvalidData(format!(
                            "索引 tRNS 条目 {} 多于调色板 {}",
                            t.len(),
                            p.len()
                        )));
                    }
                }
            }
            _ => {}
        }
    }
    let trns_gray = trns
        .filter(|_| ct == 0)
        .map(|t| u16::from_be_bytes([t[0], t[1]]));
    let trns_rgb = trns.filter(|_| ct == 2).map(|t| {
        (
            u16::from_be_bytes([t[0], t[1]]),
            u16::from_be_bytes([t[2], t[3]]),
            u16::from_be_bytes([t[4], t[5]]),
        )
    });

    let mut rgba = vec![0u8; width * height * 4];
    for y in 0..height {
        let row = &filtered[y * row_bytes..(y + 1) * row_bytes];
        for x in 0..width {
            let o = (y * width + x) * 4;
            match ct {
                0 => {
                    let raw = sample_at(row, x, depth);
                    let g = scale8(raw, depth);
                    let a = if trns_gray == Some(raw) { 0 } else { 255 };
                    rgba[o] = g;
                    rgba[o + 1] = g;
                    rgba[o + 2] = g;
                    rgba[o + 3] = a;
                }
                2 => {
                    let (r, g, b) = (
                        sample_at(row, x * 3, depth),
                        sample_at(row, x * 3 + 1, depth),
                        sample_at(row, x * 3 + 2, depth),
                    );
                    let a = if trns_rgb == Some((r, g, b)) { 0 } else { 255 };
                    rgba[o] = scale8(r, depth);
                    rgba[o + 1] = scale8(g, depth);
                    rgba[o + 2] = scale8(b, depth);
                    rgba[o + 3] = a;
                }
                3 => {
                    let pal = palette.expect("索引色已在调用前保证有 PLTE");
                    let idx = sample_at(row, x, depth) as usize;
                    let px = pal.get(idx).ok_or_else(|| {
                        PngError::InvalidData(format!(
                            "调色板索引 {idx} 越界（共 {} 项）",
                            pal.len()
                        ))
                    })?;
                    rgba[o] = px[0];
                    rgba[o + 1] = px[1];
                    rgba[o + 2] = px[2];
                    rgba[o + 3] = trns.and_then(|t| t.get(idx).copied()).unwrap_or(255);
                }
                4 => {
                    let g = scale8(sample_at(row, x * 2, depth), depth);
                    rgba[o] = g;
                    rgba[o + 1] = g;
                    rgba[o + 2] = g;
                    rgba[o + 3] = scale8(sample_at(row, x * 2 + 1, depth), depth);
                }
                6 => {
                    rgba[o] = scale8(sample_at(row, x * 4, depth), depth);
                    rgba[o + 1] = scale8(sample_at(row, x * 4 + 1, depth), depth);
                    rgba[o + 2] = scale8(sample_at(row, x * 4 + 2, depth), depth);
                    rgba[o + 3] = scale8(sample_at(row, x * 4 + 3, depth), depth);
                }
                _ => unreachable!("IHDR 已校验颜色类型"),
            }
        }
    }
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 手写一个最小的 8 位灰度 PNG（单 IDAT、filter None），用于解码路径单测。
    fn tiny_gray_png(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        fn chunk(out: &mut Vec<u8>, ctype: &[u8; 4], data: &[u8]) {
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            out.extend_from_slice(ctype);
            out.extend_from_slice(data);
            let mut h = crc32fast::Hasher::new();
            h.update(ctype);
            h.update(data);
            out.extend_from_slice(&h.finalize().to_be_bytes());
        }

        let mut png = Vec::new();
        png.extend_from_slice(&SIGNATURE);
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, 0, 0, 0, 0]); // 灰度 8 位、非隔行
        chunk(&mut png, b"IHDR", &ihdr);

        let mut raw = Vec::new();
        for row in pixels.chunks_exact(width as usize) {
            raw.push(0);
            raw.extend_from_slice(row);
        }
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
        enc.write_all(&raw).unwrap();
        let idat = enc.finish().unwrap();
        chunk(&mut png, b"IDAT", &idat);
        chunk(&mut png, b"IEND", &[]);
        png
    }

    #[test]
    fn decodes_gray8() {
        let png = tiny_gray_png(2, 2, &[0, 128, 255, 7]);
        let img = decode(&png).unwrap();
        assert_eq!((img.width(), img.height()), (2, 2));
        assert_eq!(&img.data()[..4], &[0, 0, 0, 255]);
        assert_eq!(&img.data()[4..8], &[128, 128, 128, 255]);
        assert_eq!(&img.data()[8..12], &[255, 255, 255, 255]);
        assert_eq!(&img.data()[12..], &[7, 7, 7, 255]);
    }

    #[test]
    fn rejects_non_png() {
        assert_eq!(decode(b"hello world").unwrap_err(), PngError::NotPng);
        assert_eq!(decode(b"").unwrap_err(), PngError::NotPng);
    }

    #[test]
    fn rejects_truncated() {
        let png = tiny_gray_png(1, 1, &[42]);
        for cut in [20usize, 30, png.len() - 1] {
            assert!(
                matches!(decode(&png[..cut]).unwrap_err(), PngError::Truncated),
                "截断到 {cut} 字节应报 Truncated"
            );
        }
        // 只有签名：连 IHDR 都没有
        assert!(matches!(
            decode(&png[..SIGNATURE.len()]).unwrap_err(),
            PngError::InvalidHeader(_)
        ));
    }

    #[test]
    fn rejects_bad_crc() {
        let mut png = tiny_gray_png(1, 1, &[42]);
        // 改动 IDAT 数据字节但不改 CRC → 校验失败
        let idat_pos = png
            .windows(4)
            .position(|w| w == b"IDAT")
            .expect("IDAT 存在")
            + 4;
        png[idat_pos] ^= 0xFF;
        assert!(matches!(
            decode(&png).unwrap_err(),
            PngError::CrcMismatch { .. }
        ));
    }
}
