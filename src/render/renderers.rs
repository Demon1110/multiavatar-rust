//! 渲染器简单工厂：按 [AvatarFormat] 创建对应策略实例。

use super::avatar_format::AvatarFormat;
use super::avatar_renderer::AvatarRenderer;
use super::gif_avatar_renderer::GifAvatarRenderer;
use super::jpg_avatar_renderer::JpgAvatarRenderer;
use super::png_avatar_renderer::PngAvatarRenderer;
use super::svg_avatar_renderer::SvgAvatarRenderer;

/// 渲染器简单工厂。
pub struct Renderers;

impl Renderers {
    /// 按格式创建渲染器。
    pub fn create(format: AvatarFormat) -> Box<dyn AvatarRenderer> {
        match format {
            AvatarFormat::Svg => Box::new(SvgAvatarRenderer),
            AvatarFormat::Png => Box::new(PngAvatarRenderer),
            AvatarFormat::Jpg => Box::new(JpgAvatarRenderer),
            AvatarFormat::Gif => Box::new(GifAvatarRenderer),
        }
    }
}
