//! 渲染策略层：格式枚举 + 渲染器接口 + 各格式编码器。

pub mod avatar_format;
pub mod avatar_renderer;
pub mod renderers;
pub mod svg_avatar_renderer;
pub mod png_avatar_renderer;
pub mod jpg_avatar_renderer;
pub mod gif_avatar_renderer;
pub mod png_writer;
pub mod jpg_writer;
pub mod gif_writer;
pub mod favicons;
pub mod downscale;

pub use avatar_format::AvatarFormat;
pub use avatar_renderer::AvatarRenderer;
pub use renderers::Renderers;
