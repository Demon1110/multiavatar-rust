//! 端到端：任意 PNG → 多尺寸 ICO（含多种颜色类型、透明、非正方形、非法输入）。
//!
//! 源 PNG 用 dev-dependency 的 `png` crate 独立编码，产物 ICO 的容器结构与
//! 每个内嵌 PNG 也用 `png` crate 独立解析——两侧都不复用被测代码，验证才有效。

use std::io::Cursor;

use multiavatar::{png_to_ico, render::png_ico::PngToIcoError, write_png_ico};

/// 用 png crate 编一张源图；`palette`/`trns` 仅索引色需要。
fn encode(
    width: u32,
    height: u32,
    color: png::ColorType,
    depth: png::BitDepth,
    data: &[u8],
    palette: Option<&[u8]>,
    trns: Option<&[u8]>,
) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(color);
        encoder.set_depth(depth);
        if let Some(p) = palette {
            encoder.set_palette(p);
        }
        if let Some(t) = trns {
            encoder.set_trns(t);
        }
        let mut writer = encoder.write_header().expect("源图 PNG 头部写入失败");
        writer
            .write_image_data(data)
            .expect("源图 PNG 数据写入失败");
    }
    out
}

fn encode_rgba(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    encode(
        width,
        height,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        rgba,
        None,
        None,
    )
}

fn solid_rgba(width: u32, height: u32, px: [u8; 4]) -> Vec<u8> {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..width * height {
        data.extend_from_slice(&px);
    }
    data
}

/// ICO 条目（已从容器切出的内嵌 PNG）。
struct Entry {
    size: u8,
    png: Vec<u8>,
}

/// 解析 ICO 容器：校验 ICONDIR 与每个 ICONDIRENTRY 的偏移/长度。
fn parse_ico(ico: &[u8]) -> Vec<Entry> {
    assert_eq!(&ico[0..2], &[0, 0], "ICONDIR.reserved 应为 0");
    assert_eq!(
        u16::from_le_bytes([ico[2], ico[3]]),
        1,
        "ICONDIR.type 应为 1（icon）"
    );
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let p = 6 + i * 16;
        let size = ico[p];
        assert_eq!(ico[p + 1], size, "width/height 应一致");
        assert_eq!(ico[p + 2], 0, "colorCount 应为 0（PNG 内嵌）");
        assert_eq!(ico[p + 3], 0, "reserved 应为 0");
        assert_eq!(
            u16::from_le_bytes([ico[p + 4], ico[p + 5]]),
            1,
            "planes 应为 1"
        );
        assert_eq!(
            u16::from_le_bytes([ico[p + 6], ico[p + 7]]),
            32,
            "bitCount 应为 32"
        );
        let len = u32::from_le_bytes([ico[p + 8], ico[p + 9], ico[p + 10], ico[p + 11]]) as usize;
        let offset =
            u32::from_le_bytes([ico[p + 12], ico[p + 13], ico[p + 14], ico[p + 15]]) as usize;
        assert!(offset + len <= ico.len(), "条目 {i} 偏移越界");
        entries.push(Entry {
            size,
            png: ico[offset..offset + len].to_vec(),
        });
    }
    entries
}

/// 用 png crate 解码内嵌 PNG，返回 (宽, 高, RGBA8)。
fn decode_png(png: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(Cursor::new(png));
    let mut reader = decoder.read_info().expect("内嵌 PNG 头解析失败");
    let mut buf = vec![0u8; reader.output_buffer_size().expect("输出缓冲大小未知")];
    let info = reader.next_frame(&mut buf).expect("内嵌 PNG 解码失败");
    assert_eq!(info.color_type, png::ColorType::Rgba, "内嵌 PNG 应为 RGBA8");
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

fn px_at(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let o = ((y * width + x) * 4) as usize;
    [rgba[o], rgba[o + 1], rgba[o + 2], rgba[o + 3]]
}

fn approx(actual: &[u8; 4], expected: [u8; 4], what: &str) {
    for c in 0..4 {
        assert!(
            (i32::from(actual[c]) - i32::from(expected[c])).abs() <= 2,
            "{what}: 通道 {c} 期望 {}，实际 {}（像素 {:?}）",
            expected[c],
            actual[c],
            actual
        );
    }
}

#[test]
fn default_sizes_produce_six_entries() {
    let src = encode_rgba(64, 64, &solid_rgba(64, 64, [30, 60, 90, 255]));
    let ico = png_to_ico(&src, &[]).expect("默认尺寸应成功");
    let entries = parse_ico(&ico);
    assert_eq!(entries.len(), 6, "默认应为 6 个条目");
    // 16/32/48/64 直接写字节，128 直接写字节，256 写成 0（规范表示 256）
    let expected_sizes = [16u8, 32, 48, 64, 128, 0];
    for (entry, expected) in entries.iter().zip(expected_sizes) {
        assert_eq!(entry.size, expected, "ICONDIRENTRY 宽字节");
        let size = if expected == 0 {
            256
        } else {
            u32::from(expected)
        };
        let (w, h, rgba) = decode_png(&entry.png);
        assert_eq!((w, h), (size, size), "内嵌 PNG 尺寸");
        approx(
            &px_at(&rgba, w, w / 2, h / 2),
            [30, 60, 90, 255],
            "纯色应无漂移",
        );
    }
}

#[test]
fn specified_sizes_keep_order_and_dedupe() {
    let src = encode_rgba(32, 32, &solid_rgba(32, 32, [200, 40, 40, 255]));
    let ico = png_to_ico(&src, &[32, 16, 32, 64]).expect("指定尺寸应成功");
    let sizes: Vec<u8> = parse_ico(&ico).iter().map(|e| e.size).collect();
    assert_eq!(sizes, vec![32, 16, 64], "顺序保留，重复项去重");
}

#[test]
fn non_square_is_letterboxed_not_stretched() {
    // 40×20 纯红 → 32 宽高：按 contain 落在 32×16，上下各 8 行透明
    let src = encode_rgba(40, 20, &solid_rgba(40, 20, [255, 0, 0, 255]));
    let ico = png_to_ico(&src, &[32]).expect("非正方形应成功");
    let entries = parse_ico(&ico);
    let (w, h, rgba) = decode_png(&entries[0].png);
    assert_eq!((w, h), (32, 32));
    assert_eq!(px_at(&rgba, w, 0, 0)[3], 0, "左上角应透明");
    assert_eq!(px_at(&rgba, w, w - 1, 0)[3], 0, "右上角应透明");
    assert_eq!(px_at(&rgba, w, 0, h - 1)[3], 0, "左下角应透明");
    // 图像区（第 8..24 行）内：横向铺满、纵向居中
    approx(&px_at(&rgba, w, 0, 16), [255, 0, 0, 255], "图像区左边缘");
    approx(&px_at(&rgba, w, 31, 16), [255, 0, 0, 255], "图像区右边缘");
    approx(&px_at(&rgba, w, 16, 16), [255, 0, 0, 255], "图像区中心");
}

#[test]
fn translucent_pixels_are_not_darkened() {
    // 左半不透明红、右半全透明 → 缩小后左侧仍应是红色而不是暗红（预乘滤波）
    let mut rgba = vec![0u8; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let o = (y * 32 + x) * 4;
            if x < 16 {
                rgba[o] = 255;
                rgba[o + 3] = 128; // 半透明红
            }
        }
    }
    let src = encode_rgba(32, 32, &rgba);
    let ico = png_to_ico(&src, &[16]).expect("半透明源图应成功");
    let entries = parse_ico(&ico);
    let (w, h, out) = decode_png(&entries[0].png);
    let p = px_at(&out, w, 3, h / 2);
    assert!(p[0] >= 250, "红色通道不应被 alpha 拉暗，实际 {}", p[0]);
    assert!(
        (i32::from(p[3]) - 128).abs() <= 2,
        "alpha 应保持 ~128，实际 {}",
        p[3]
    );
}

#[test]
fn grayscale_source_becomes_rgba() {
    let gray: Vec<u8> = vec![128u8; 16 * 16];
    let src = encode(
        16,
        16,
        png::ColorType::Grayscale,
        png::BitDepth::Eight,
        &gray,
        None,
        None,
    );
    let ico = png_to_ico(&src, &[16]).expect("灰度源图应成功");
    let entries = parse_ico(&ico);
    let (w, h, out) = decode_png(&entries[0].png);
    approx(
        &px_at(&out, w, 8, h / 2),
        [128, 128, 128, 255],
        "灰度应展开为三通道相等",
    );
}

#[test]
fn sixteen_bit_source_takes_high_byte() {
    // 灰度 16 位，样本 0x8000（大端 [0x80, 0x00]），按 16→8 取高字节应为 128
    let gray: Vec<u8> = (0..16 * 16).flat_map(|_| [0x80u8, 0x00]).collect();
    let src = encode(
        16,
        16,
        png::ColorType::Grayscale,
        png::BitDepth::Sixteen,
        &gray,
        None,
        None,
    );
    let ico = png_to_ico(&src, &[16]).expect("16 位源图应成功");
    let entries = parse_ico(&ico);
    let (w, h, out) = decode_png(&entries[0].png);
    approx(
        &px_at(&out, w, 8, h / 2),
        [128, 128, 128, 255],
        "16 位应取高字节",
    );
}

#[test]
fn indexed_source_applies_trns() {
    // 调色板：0=红、1=绿；tRNS 让索引 0 全透明
    let palette = [255u8, 0, 0, 0, 255, 0];
    let indices: Vec<u8> = (0..16 * 16).map(|i| (i % 2) as u8).collect();
    let src = encode(
        16,
        16,
        png::ColorType::Indexed,
        png::BitDepth::Eight,
        &indices,
        Some(&palette),
        Some(&[0u8, 255]),
    );
    let ico = png_to_ico(&src, &[16]).expect("索引色源图应成功");
    let entries = parse_ico(&ico);
    let (w, h, out) = decode_png(&entries[0].png);
    let red = px_at(&out, w, 0, h / 2);
    assert_eq!(red[3], 0, "索引 0 应被 tRNS 置为透明");
    let green = px_at(&out, w, 1, h / 2);
    approx(&green, [0, 255, 0, 255], "索引 1 应保留绿色且不透明");
}

#[test]
fn upscale_small_source() {
    let src = encode_rgba(8, 8, &solid_rgba(8, 8, [12, 200, 44, 255]));
    let ico = png_to_ico(&src, &[16, 32]).expect("放大应成功");
    for entry in parse_ico(&ico) {
        let size = u32::from(entry.size);
        let (w, h, out) = decode_png(&entry.png);
        assert_eq!((w, h), (size, size));
        approx(
            &px_at(&out, w, w / 2, h / 2),
            [12, 200, 44, 255],
            "放大后颜色保持",
        );
    }
}

#[test]
fn invalid_inputs_are_errors() {
    assert!(matches!(
        png_to_ico(b"not a png", &[]).unwrap_err(),
        PngToIcoError::Png(_)
    ));
    assert!(matches!(
        png_to_ico(&[], &[]).unwrap_err(),
        PngToIcoError::Png(_)
    ));
    let ok = encode_rgba(4, 4, &solid_rgba(4, 4, [0, 0, 0, 255]));
    assert_eq!(
        png_to_ico(&ok, &[0]).unwrap_err(),
        PngToIcoError::InvalidSize(0)
    );
    assert_eq!(
        png_to_ico(&ok, &[300]).unwrap_err(),
        PngToIcoError::InvalidSize(300)
    );
    // 截断的 PNG（去掉尾部 IEND）
    let truncated = &ok[..ok.len() - 12];
    assert!(matches!(
        png_to_ico(truncated, &[]).unwrap_err(),
        PngToIcoError::Png(_)
    ));
}

#[test]
fn file_api_roundtrip() {
    let dir = std::env::temp_dir().join(format!("multiavatar-ico-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("创建临时目录失败");
    let png_path = dir.join("source.png");
    let ico_path = dir.join("icon.ico");
    let src = encode_rgba(24, 24, &solid_rgba(24, 24, [0, 120, 255, 255]));
    std::fs::write(&png_path, &src).expect("写入源 PNG 失败");

    write_png_ico(&png_path, &ico_path, &[16, 32, 48]).expect("写 ICO 失败");
    let ico = std::fs::read(&ico_path).expect("读取 ICO 失败");
    let entries = parse_ico(&ico);
    assert_eq!(
        entries.iter().map(|e| e.size).collect::<Vec<u8>>(),
        vec![16, 32, 48]
    );

    let missing = dir.join("no-such.png");
    assert!(
        write_png_ico(&missing, &ico_path, &[16]).is_err(),
        "缺文件应报 IO 错误"
    );

    std::fs::remove_dir_all(&dir).ok();
}
