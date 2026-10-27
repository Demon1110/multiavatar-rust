//! SVG 组装器：固定「推导规格 → 逐部件着色 → 按序拼装」的组装流程（模板方法）。

use crate::core::avatar_id_hasher::Sha256AvatarIdHasher;
use crate::core::avatar_spec::{AvatarSpec, PART_ORDER};
use crate::core::svg_fragment_painter;
use crate::data::DataTables;

/// 默认 SVG 宽高（像素）。
pub const DEFAULT_SVG_SIZE: i32 = 256;

const SVG_END: &str = "</svg>";

/// SVG 根开标签（width/height 与 viewBox 取自构建配置）。
const SVG_START: &str =
    "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"{vb}\">";

/// SVG 组装器：SHA-256 + 默认数据源 + 指定 viewBox。
pub struct SvgComposer {
    view_box: String,
    svg_width: i32,
    svg_height: i32,
}

impl SvgComposer {
    /// 便捷构造：SHA-256 + 默认数据源 + 指定 viewBox，宽高取默认 256×256。
    pub fn with_view_box(view_box: &str) -> SvgComposer {
        SvgComposer {
            view_box: view_box.to_string(),
            svg_width: DEFAULT_SVG_SIZE,
            svg_height: DEFAULT_SVG_SIZE,
        }
    }

    /// 便捷构造：SHA-256 + 默认数据源 + 指定 viewBox 与默认宽高。
    pub fn with_view_box_size(view_box: &str, width: i32, height: i32) -> SvgComposer {
        SvgComposer {
            view_box: view_box.to_string(),
            svg_width: width,
            svg_height: height,
        }
    }

    /// 组装完整 SVG（使用本实例默认宽高）。
    ///
    /// 输入为空串时返回空串（与 JS 一致）。
    pub fn compose(
        &self,
        input: &str,
        sans_env: bool,
        forced_part: Option<&str>,
        forced_theme: Option<&str>,
    ) -> String {
        self.compose_sized(input, sans_env, forced_part, forced_theme, self.svg_width, self.svg_height)
    }

    /// 组装完整 SVG（自定义根元素宽高）。
    pub fn compose_sized(
        &self,
        input: &str,
        sans_env: bool,
        forced_part: Option<&str>,
        forced_theme: Option<&str>,
        width: i32,
        height: i32,
    ) -> String {
        if input.is_empty() {
            return String::new();
        }

        let spec = AvatarSpec::resolve(
            input,
            &Sha256AvatarIdHasher,
            forced_part,
            forced_theme,
        );

        let mut sb = String::with_capacity(4096);
        sb.push_str(&SVG_START.replace("{w}", &width.to_string())
            .replace("{h}", &height.to_string())
            .replace("{vb}", &self.view_box));
        let tables = DataTables::instance();
        for part_name in PART_ORDER {
            if sans_env && part_name == "env" {
                continue; // JS: final['env'] = ''
            }
            if let (Some(part_v), Some(theme)) = (spec.part_v_of(part_name), spec.theme_of(part_name))
            {
                sb.push_str(&svg_fragment_painter::paint(tables, part_name, part_v, theme));
            }
        }
        sb.push_str(SVG_END);
        sb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_empty_svg() {
        let composer = SvgComposer::with_view_box("0 0 231 231");
        assert_eq!(composer.compose("", false, None, None), "");
    }

    #[test]
    fn compose_basic() {
        let composer = SvgComposer::with_view_box("0 0 231 231");
        let svg = composer.compose("Binx Bond", false, None, None);
        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"256\" height=\"256\" viewBox=\"0 0 231 231\">"));
        assert!(svg.ends_with("</svg>"));
        // 6 个部件都有输出（含 env 背景圆等），着色存在
        assert!(svg.len() > 500, "SVG 过短: {}", svg.len());
        assert!(svg.contains("style="), "缺少着色样式");
        assert!(svg.contains("<path"), "缺少 path 元素");
    }

    #[test]
    fn sans_env_omits_background() {
        let composer = SvgComposer::with_view_box("0 0 231 231");
        let full = composer.compose("Binx Bond", false, None, None);
        let sans = composer.compose("Binx Bond", true, None, None);
        assert!(sans.len() < full.len());
    }
}
