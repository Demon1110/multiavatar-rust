//! 头像输出格式。

/// 头像输出格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarFormat {
    /// SVG 文本。
    Svg,
    /// PNG 图片字节（透明背景）。
    Png,
    /// JPEG 图片字节（白色背景，不支持透明）。
    Jpg,
    /// GIF 动画字节：部件逐帧淡入，无限循环。
    Gif,
}
