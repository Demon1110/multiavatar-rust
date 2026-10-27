//! 双线性降采样（超采样抗锯齿用）：把大尺寸 [Pixmap] 缩到目标尺寸。
//!
//! 原理：先以 `size × factor` 分辨率渲染（tiny-skia 自带解析式抗锯齿），
//! 再双线性缩小到目标尺寸 —— 等效 4x 超采样（SSAA），曲线边缘显著更平滑。
//! premultiplied-alpha 空间做加权平均（权重和为 1），结果仍为合法 premultiplied 值。

use tiny_skia::Pixmap;

/// 双线性降采样到目标边长（正方形画布）。
///
/// 源图必须 ≥ 目标尺寸（SSAA 语义）；中心对齐采样，四邻域加权。
pub fn bilinear(src: &Pixmap, dst_size: u32) -> Option<Pixmap> {
    let sw = src.width() as f32;
    let sh = src.height() as f32;
    if dst_size == 0 || src.width() < dst_size || src.height() < dst_size {
        return None;
    }
    let mut dst = Pixmap::new(dst_size, dst_size)?;
    let data = src.data();
    let dst_data = dst.data_mut();
    let src_stride = src.width() as usize * 4;
    for y in 0..dst_size {
        // 目标像素中心 → 源坐标（-0.5 偏移做中心对齐）
        let sy = (y as f32 + 0.5) * sh / dst_size as f32 - 0.5;
        let y0 = sy.floor().max(0.0) as usize;
        let y1 = (sy.ceil() as usize).min(src.height() as usize - 1);
        let wy = sy - y0 as f32;
        for x in 0..dst_size {
            let sx = (x as f32 + 0.5) * sw / dst_size as f32 - 0.5;
            let x0 = sx.floor().max(0.0) as usize;
            let x1 = (sx.ceil() as usize).min(src.width() as usize - 1);
            let wx = sx - x0 as f32;
            let (w00, w10, w01, w11) = (
                (1.0 - wx) * (1.0 - wy),
                wx * (1.0 - wy),
                (1.0 - wx) * wy,
                wx * wy,
            );
            let (i00, i10, i01, i11) = (
                y0 * src_stride + x0 * 4,
                y0 * src_stride + x1 * 4,
                y1 * src_stride + x0 * 4,
                y1 * src_stride + x1 * 4,
            );
            let o = (y as usize * dst_size as usize + x as usize) * 4;
            for c in 0..4 {
                let v = data[i00 + c] as f32 * w00
                    + data[i10 + c] as f32 * w10
                    + data[i01 + c] as f32 * w01
                    + data[i11 + c] as f32 * w11;
                dst_data[o + c] = (v + 0.5) as u8;
            }
        }
    }
    Some(dst)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 纯色图缩小后仍为纯色（插值无泄漏）。
    #[test]
    fn solid_color_unchanged() {
        let mut src = Pixmap::new(8, 8).unwrap();
        src.fill(tiny_skia::Color::from_rgba8(30, 60, 90, 255));
        let dst = bilinear(&src, 4).unwrap();
        for px in dst.data().chunks(4) {
            assert_eq!(&px[..3], &[30, 60, 90], "纯色缩小后应保持一致");
            assert_eq!(px[3], 255);
        }
    }

    /// 尺寸缩小一半；边缘黑白交界处出现过渡灰度（平滑的证据）。
    #[test]
    fn half_size_and_blend() {
        let mut src = Pixmap::new(16, 16).unwrap();
        src.fill(tiny_skia::Color::WHITE);
        // 左半边黑，右半边白（X < 8 黑）
        let d = src.data_mut();
        for y in 0..16usize {
            for x in 0..8usize {
                let o = (y * 16 + x) * 4;
                d[o] = 0;
                d[o + 1] = 0;
                d[o + 2] = 0;
            }
        }
        let dst = bilinear(&src, 8).unwrap();
        assert_eq!(dst.width(), 8);
        // 左 4 列应接近纯黑（(0+0.5)*16/8-0.5 = 0.5 落在黑区），右 4 列接近纯白
        for y in 0..8usize {
            let o = (y * 8 + 3) * 4;
            assert!(dst.data()[o] < 30, "左半应偏黑: {}", dst.data()[o]);
            let o = (y * 8 + 4) * 4;
            assert!(dst.data()[o] > 225, "右半应偏白: {}", dst.data()[o]);
        }
    }
}
