//! 通用 PNG → 多尺寸 ICO：解码任意 PNG，按目标尺寸重采样，逐尺寸编码 PNG 并打包 ICO。
//!
//! 重采样在**预乘 alpha** 空间做两趟分离式滤波（先纵向、后横向）：
//! 缩小走面积平均（box，等效整像素覆盖加权），放大走双线性（2 抽头）；
//! 权重和恒为 1，避免透明边缘出现黑边/彩边。非正方形源图按 contain 等比缩放、
//! 居中放置，四周留透明边——ICO 每个条目必须是正方形。
//!
//! 中间缓冲只保留「目标高 × 源宽」的 f32 通道，先把大图降到目标高度再横向缩，
//! 避免「源高 × 源宽」整幅 f32 缓冲（4000×4000 源会吃掉数百 MB）。

use std::fmt;

use super::favicons::{pack_ico, DEFAULT_SIZES};
use super::png_reader::{self, PngError, RgbaImage};
use super::png_writer::to_png_from_rgba8;

/// ICO 单条目边长上限：规范用 0 表示 256，超过 256 无法表达。
pub const MAX_ICON_SIZE: u32 = 256;

/// PNG → ICO 转换错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngToIcoError {
    /// 源 PNG 解码失败。
    Png(PngError),
    /// 尺寸非法：ICO 条目边长必须在 1..=256。
    InvalidSize(u32),
}

impl fmt::Display for PngToIcoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PngToIcoError::Png(e) => write!(f, "PNG 解码失败：{e}"),
            PngToIcoError::InvalidSize(s) => {
                write!(f, "ICO 尺寸 {s} 非法（必须在 1..={MAX_ICON_SIZE} 像素）")
            }
        }
    }
}

impl std::error::Error for PngToIcoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PngToIcoError::Png(e) => Some(e),
            PngToIcoError::InvalidSize(_) => None,
        }
    }
}

impl From<PngError> for PngToIcoError {
    fn from(e: PngError) -> Self {
        PngToIcoError::Png(e)
    }
}

/// 规范化尺寸集合：空集合用 [`DEFAULT_SIZES`]，逐项校验 1..=256，重复项保留首次出现。
pub fn normalize_sizes(sizes: &[u32]) -> Result<Vec<u32>, PngToIcoError> {
    let source: &[u32] = if sizes.is_empty() {
        &DEFAULT_SIZES
    } else {
        sizes
    };
    let mut list = Vec::with_capacity(source.len());
    for &size in source {
        if size == 0 || size > MAX_ICON_SIZE {
            return Err(PngToIcoError::InvalidSize(size));
        }
        if !list.contains(&size) {
            list.push(size);
        }
    }
    Ok(list)
}

/// PNG 字节 → 多尺寸 ICO 字节（PNG 内嵌，Vista+ 的 Windows / 浏览器直接识别）。
///
/// 空 `sizes` 用默认 16/32/48/64/128/256；每条目都等比 contain 到正方形并居中留透明。
pub fn to_ico(png: &[u8], sizes: &[u32]) -> Result<Vec<u8>, PngToIcoError> {
    let sizes = normalize_sizes(sizes)?;
    let image = png_reader::decode(png)?;
    let entries: Vec<(u32, Vec<u8>)> = sizes
        .iter()
        .map(|&size| {
            let rgba = fit_square(&image, size);
            (size, to_png_from_rgba8(size, size, &rgba))
        })
        .collect();
    Ok(pack_ico(&entries))
}

/// 把源图等比缩放到 `size × size` 的正方形画布（contain，居中，空白处透明）。
///
/// 返回直通 RGBA8 像素，长度 `size * size * 4`。`size` 为 0 时返回空数组。
pub fn fit_square(src: &RgbaImage, size: u32) -> Vec<u8> {
    if size == 0 {
        return Vec::new();
    }
    let target = size as usize;
    let (sw, sh) = (src.width() as usize, src.height() as usize);
    if sw == target && sh == target {
        return src.data().to_vec();
    }

    // contain：短边贴满目标，长边按比例
    let scale = f64::min(target as f64 / sw as f64, target as f64 / sh as f64);
    let dw = ((sw as f64 * scale).round() as usize).clamp(1, target);
    let dh = ((sh as f64 * scale).round() as usize).clamp(1, target);
    let ox = (target - dw) / 2;
    let oy = (target - dh) / 2;

    let data = src.data();
    let yw = axis_weights(sh, dh);
    // 纵向：src(sw × sh) → sw × dh，预乘 alpha 的 f32 通道
    let mut mid = vec![0f32; sw * dh * 4];
    for (dy, (start, weights)) in yw.iter().enumerate() {
        let base = dy * sw * 4;
        for x in 0..sw {
            let mut acc = [0f32; 4];
            for (k, w) in weights.iter().enumerate() {
                let i = ((start + k) * sw + x) * 4;
                let px = &data[i..i + 4];
                let a = f32::from(px[3]);
                acc[0] += f32::from(px[0]) * a / 255.0 * w;
                acc[1] += f32::from(px[1]) * a / 255.0 * w;
                acc[2] += f32::from(px[2]) * a / 255.0 * w;
                acc[3] += a * w;
            }
            mid[base + x * 4..base + x * 4 + 4].copy_from_slice(&acc);
        }
    }

    let xw = axis_weights(sw, dw);
    let mut out = vec![0u8; target * target * 4];
    for dy in 0..dh {
        let row = dy * sw * 4;
        for (dx, (start, weights)) in xw.iter().enumerate() {
            let mut acc = [0f32; 4];
            for (k, w) in weights.iter().enumerate() {
                let i = row + (start + k) * 4;
                for (c, slot) in acc.iter_mut().enumerate() {
                    *slot += mid[i + c] * w;
                }
            }
            let o = ((oy + dy) * target + ox + dx) * 4;
            let a = acc[3].clamp(0.0, 255.0);
            out[o] = unpremultiply(acc[0], a);
            out[o + 1] = unpremultiply(acc[1], a);
            out[o + 2] = unpremultiply(acc[2], a);
            out[o + 3] = a.round() as u8;
        }
    }
    out
}

/// 预乘通道 → 直通通道（alpha 为 0 时无色可还原，直接输出 0）。
fn unpremultiply(c: f32, a: f32) -> u8 {
    if a <= 0.0 {
        0
    } else {
        (c / a * 255.0).round().clamp(0.0, 255.0) as u8
    }
}

/// 单轴权重表：每个目标像素 = (起始源索引, 各抽头权重)。
///
/// `dst_len <= src_len` 时按区间覆盖做面积平均（权重 = 重叠长度占比）；
/// 放大时做双线性（2 抽头，索引钳制到边界）。权重总和归一化为 1。
fn axis_weights(src_len: usize, dst_len: usize) -> Vec<(usize, Vec<f32>)> {
    let ratio = src_len as f64 / dst_len as f64;
    let mut table = Vec::with_capacity(dst_len);
    for d in 0..dst_len {
        let mut weights: Vec<f32> = Vec::with_capacity(2);
        let start;
        if ratio >= 1.0 {
            let begin = d as f64 * ratio;
            let end = (d + 1) as f64 * ratio;
            let i0 = begin.floor() as usize;
            let i1 = (end.ceil() as usize).min(src_len);
            let span = end - begin;
            for i in i0..i1 {
                let lo = (i as f64).max(begin);
                let hi = ((i + 1) as f64).min(end);
                weights.push(((hi - lo).max(0.0) / span) as f32);
            }
            start = i0.min(src_len - 1);
        } else {
            // 放大：目标像素中心映射回源坐标，取相邻两像素线性插值
            let center = (d as f64 + 0.5) * ratio - 0.5;
            let i0 = center.floor();
            let frac = (center - i0) as f32;
            let left = (i0 as isize).clamp(0, src_len as isize - 1) as usize;
            let right = ((i0 + 1.0) as isize).clamp(0, src_len as isize - 1) as usize;
            if left == right {
                weights.push(1.0);
                start = left;
            } else {
                weights.push(1.0 - frac);
                weights.push(frac);
                start = left;
            }
        }
        let total: f32 = weights.iter().sum();
        if total > 0.0 && (total - 1.0).abs() > f32::EPSILON {
            for w in &mut weights {
                *w /= total;
            }
        }
        table.push((start, weights));
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32, rgba: [u8; 4]) -> RgbaImage {
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..width * height {
            data.extend_from_slice(&rgba);
        }
        RgbaImage::new(width, height, data)
    }

    /// 纯色缩小/放大后仍为纯色（权重和为 1，无亮度漂移）。
    #[test]
    fn solid_survives_rescale() {
        for size in [8u32, 16, 32] {
            let out = fit_square(&solid(64, 64, [30, 60, 90, 255]), size);
            assert_eq!(out.len(), (size * size * 4) as usize);
            for px in out.chunks_exact(4) {
                assert_eq!(&px[..3], &[30, 60, 90], "尺寸 {size} 颜色漂移");
                assert_eq!(px[3], 255);
            }
        }
        // 放大（8 → 16）
        let out = fit_square(&solid(8, 8, [200, 10, 10, 255]), 16);
        for px in out.chunks_exact(4) {
            assert_eq!(&px[..3], &[200, 10, 10]);
        }
    }

    /// 非正方形：等比 contain，上下留透明边，不拉伸。
    #[test]
    fn non_square_is_letterboxed() {
        let src = solid(40, 20, [255, 0, 0, 255]);
        let out = fit_square(&src, 32);
        // 40:20 = 2:1 → 32×16 居中，上下各 8 行透明
        assert_eq!(&out[..4], &[0, 0, 0, 0], "首行应为透明");
        let mid = 16 * 32 * 4; // 第 16 行（位于图像区内）
        assert_eq!(&out[mid..mid + 4], &[255, 0, 0, 255]);
    }

    /// 半透明像素重采样后不应变暗（预乘空间滤波的回归点）。
    #[test]
    fn translucent_keeps_color() {
        let src = solid(32, 32, [255, 0, 0, 128]);
        let out = fit_square(&src, 16);
        let px = &out[(8 * 16 + 8) * 4..(8 * 16 + 9) * 4];
        assert_eq!(px[0], 255, "红色通道应保持 255，实际 {}", px[0]);
        assert!((i32::from(px[3]) - 128).abs() <= 1, "alpha 应保持 ~128");
    }

    #[test]
    fn sizes_are_normalized_and_validated() {
        assert_eq!(normalize_sizes(&[]).unwrap().len(), DEFAULT_SIZES.len());
        assert_eq!(normalize_sizes(&[16, 16, 32]).unwrap(), vec![16, 32]);
        assert_eq!(
            normalize_sizes(&[0]).unwrap_err(),
            PngToIcoError::InvalidSize(0)
        );
        assert_eq!(
            normalize_sizes(&[300]).unwrap_err(),
            PngToIcoError::InvalidSize(300)
        );
    }
}
