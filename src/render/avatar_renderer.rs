//! 头像渲染策略接口：把 SVG 文本渲染为指定输出。

/// 头像渲染策略：把 SVG 文本渲染为指定输出。
pub trait AvatarRenderer {
    /// 渲染。
    ///
    /// - `svg`：完整 SVG 文本（非空）
    /// - `size`：目标边长（像素，SVG 渲染器可忽略）
    fn render(&self, svg: &str, size: u32) -> Vec<u8>;
}
