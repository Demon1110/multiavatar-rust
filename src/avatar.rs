//! 头像产物（不可变）：封装输入字符串对应的 SVG 文本，并提供图像渲染能力。
//!
//! SVG 生成后缓存；图像按请求尺寸惰性渲染：`to_image` 直接返回 tiny-skia Pixmap
//! 供继续加工，`png`/`jpg` 编码为对应格式字节。
//! 解析后的 [crate::svg::SvgDocument] 同样惰性缓存——同一 Avatar 的多次图像请求只解析一次 SVG。

use std::sync::{Arc, OnceLock};

use base64::Engine as _;
use tiny_skia::Pixmap;

use crate::render::{AvatarFormat, Renderers, png_writer::to_png};
use crate::svg::{SvgDocument, SvgParser, SvgRasterizer};

use super::avatar_options::AvatarOptions;

/// 头像产物（不可变）。
#[derive(Debug, Clone)]
pub struct Avatar {
    input: String,
    options: AvatarOptions,
    svg: String,
    /// 惰性解析缓存：同一 Avatar 的图像请求只 parse 一次 SVG（线程安全）。
    doc: OnceLock<Arc<SvgDocument>>,
}

impl Avatar {
    pub(crate) fn new(input: String, options: AvatarOptions, svg: String) -> Avatar {
        Avatar {
            input,
            options,
            svg,
            doc: OnceLock::new(),
        }
    }

    /// 输入字符串。
    pub fn input(&self) -> &str {
        &self.input
    }

    /// 生成时使用的选项。
    pub fn options(&self) -> &AvatarOptions {
        &self.options
    }

    /// SVG 文本（空输入时为空串）。
    pub fn svg(&self) -> &str {
        &self.svg
    }

    /// 解析后的 SVG 文档（惰性缓存，线程安全；空输入时为空文档）。
    pub fn document(&self) -> Arc<SvgDocument> {
        self.doc
            .get_or_init(|| Arc::new(SvgParser::parse(&self.svg)))
            .clone()
    }

    /// 是否有内容（输入为空时无内容）。
    pub fn is_empty(&self) -> bool {
        self.svg.is_empty()
    }

    /// 光栅化图像（默认尺寸 256，透明背景）；空输入返回 None。
    pub fn to_image(&self) -> Option<Pixmap> {
        self.to_image_sized(self.options.size())
    }

    /// 光栅化图像（指定边长，透明背景）；空输入返回 None。
    ///
    /// 直接返回 Pixmap 便于调用方继续加工（缩放/合成/加水印）或自行编码。
    pub fn to_image_sized(&self, size: u32) -> Option<Pixmap> {
        if self.is_empty() {
            return None;
        }
        let doc = self.document();
        SvgRasterizer::rasterize(&doc, size)
    }

    /// PNG 字节（默认尺寸 256）；空输入返回空数组。
    pub fn png(&self) -> Vec<u8> {
        self.png_sized(self.options.size())
    }

    /// PNG 字节（指定边长）；空输入返回空数组。
    pub fn png_sized(&self, size: u32) -> Vec<u8> {
        match self.to_image_sized(size) {
            Some(pixmap) => to_png(&pixmap),
            None => Vec::new(),
        }
    }

    /// JPEG 字节（默认尺寸 256，白底）；空输入返回空数组。
    pub fn jpg(&self) -> Vec<u8> {
        self.jpg_sized(self.options.size())
    }

    /// JPEG 字节（指定边长，白底）；空输入返回空数组。
    pub fn jpg_sized(&self, size: u32) -> Vec<u8> {
        if self.is_empty() {
            return Vec::new();
        }
        let renderer = Renderers::create(AvatarFormat::Jpg);
        renderer.render(&self.svg, size)
    }

    /// PNG data URI（默认尺寸 256）：`data:image/png;base64,...`；空输入返回空串。
    pub fn data_uri(&self) -> String {
        self.data_uri_sized(self.options.size())
    }

    /// PNG data URI（指定边长）；空输入返回空串。
    pub fn data_uri_sized(&self, size: u32) -> String {
        let png = self.png_sized(size);
        if png.is_empty() {
            return String::new();
        }
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        )
    }
}
