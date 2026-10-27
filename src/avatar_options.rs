//! 头像生成选项（不可变），通过 [AvatarOptions::builder] 构建。

use crate::render::AvatarFormat;

/// 默认 PNG 输出边长。
pub const DEFAULT_SIZE: u32 = 256;

/// 默认 SVG 根元素宽高（像素）。
pub const DEFAULT_SVG_SIZE: i32 = 256;

/// 默认是否启用 SVG 体积优化（坐标精度裁剪，近无损）。
pub const DEFAULT_OPTIMIZE_SVG: bool = true;

/// 头像生成选项（不可变）。
#[derive(Debug, Clone)]
pub struct AvatarOptions {
    sans_env: bool,
    part: Option<String>,
    theme: Option<String>,
    size: u32,
    svg_width: i32,
    svg_height: i32,
    format: AvatarFormat,
    optimize_svg: bool,
}

impl AvatarOptions {
    pub fn builder() -> AvatarOptionsBuilder {
        AvatarOptionsBuilder::default()
    }

    /// 便捷构造：默认选项。
    pub fn defaults() -> AvatarOptions {
        AvatarOptionsBuilder::default().build()
    }

    pub fn sans_env(&self) -> bool {
        self.sans_env
    }

    /// 强制角色编号（"00"-"15"），None 表示自动。
    pub fn part(&self) -> Option<&str> {
        self.part.as_deref()
    }

    /// 强制颜色主题（"A"/"B"/"C"），None 表示自动。
    pub fn theme(&self) -> Option<&str> {
        self.theme.as_deref()
    }

    /// PNG 输出边长（像素）。
    pub fn size(&self) -> u32 {
        self.size
    }

    /// SVG 根元素宽度（像素）。
    pub fn svg_width(&self) -> i32 {
        self.svg_width
    }

    /// SVG 根元素高度（像素）。
    pub fn svg_height(&self) -> i32 {
        self.svg_height
    }

    pub fn format(&self) -> AvatarFormat {
        self.format
    }

    /// 是否启用 SVG 体积优化（坐标精度裁剪，近无损）。
    pub fn optimize_svg(&self) -> bool {
        self.optimize_svg
    }
}

/// AvatarOptions 构建者（Builder 模式）。
#[derive(Debug, Clone)]
pub struct AvatarOptionsBuilder {
    sans_env: bool,
    part: Option<String>,
    theme: Option<String>,
    size: u32,
    svg_width: i32,
    svg_height: i32,
    format: AvatarFormat,
    optimize_svg: bool,
}

impl Default for AvatarOptionsBuilder {
    fn default() -> Self {
        AvatarOptionsBuilder {
            sans_env: false,
            part: None,
            theme: None,
            size: DEFAULT_SIZE,
            svg_width: DEFAULT_SVG_SIZE,
            svg_height: DEFAULT_SVG_SIZE,
            format: AvatarFormat::Svg,
            optimize_svg: DEFAULT_OPTIMIZE_SVG,
        }
    }
}

impl AvatarOptionsBuilder {
    /// 是否去掉环境部件（背景圆）。
    pub fn sans_env(mut self, sans_env: bool) -> Self {
        self.sans_env = sans_env;
        self
    }

    /// 强制初始角色编号（"00"-"15"），None 表示自动。
    pub fn part(mut self, part: impl Into<String>) -> Self {
        self.part = Some(part.into());
        self
    }

    /// 强制初始角色编号（owned，用于从不可变选项重建）。
    pub fn part_owned(mut self, part: Option<String>) -> Self {
        self.part = part;
        self
    }

    /// 强制颜色主题（"A"/"B"/"C"），None 表示自动。
    pub fn theme(mut self, theme: impl Into<String>) -> Self {
        self.theme = Some(theme.into());
        self
    }

    /// 强制颜色主题（owned，用于从不可变选项重建）。
    pub fn theme_owned(mut self, theme: Option<String>) -> Self {
        self.theme = theme;
        self
    }

    /// PNG 输出边长（像素）。
    pub fn size(mut self, size: u32) -> Self {
        self.size = size.max(1);
        self
    }

    /// SVG 根元素宽高（像素）。
    pub fn svg_size(mut self, width: i32, height: i32) -> Self {
        self.svg_width = width.max(1);
        self.svg_height = height.max(1);
        self
    }

    /// 输出格式。
    pub fn format(mut self, format: AvatarFormat) -> Self {
        self.format = format;
        self
    }

    /// 是否启用 SVG 体积优化（默认开）；设 false 可获得与官方 JS 逐字符一致的原文。
    pub fn optimize_svg(mut self, optimize_svg: bool) -> Self {
        self.optimize_svg = optimize_svg;
        self
    }

    pub fn build(self) -> AvatarOptions {
        AvatarOptions {
            sans_env: self.sans_env,
            part: self.part,
            theme: self.theme,
            size: self.size,
            svg_width: self.svg_width,
            svg_height: self.svg_height,
            format: self.format,
            optimize_svg: self.optimize_svg,
        }
    }
}
