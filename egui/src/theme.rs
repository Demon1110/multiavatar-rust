//! 绚丽主题：三套配色（暗夜霓虹 / 明亮 / 赛博霓虹）+ 渐变背景 + 发光控件。

use eframe::egui::{self, Color32, CornerRadius, FontId, Margin, Stroke, TextStyle};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeKind {
    /// 暗夜霓虹（默认）：深蓝紫底 + 青色光
    Dark,
    /// 明亮清爽：浅底 + 蓝紫点缀
    Light,
    /// 赛博霓虹：更深底色 + 双霓虹色（青 + 粉）
    Neon,
}

impl ThemeKind {
    pub const ALL: [ThemeKind; 3] = [ThemeKind::Dark, ThemeKind::Light, ThemeKind::Neon];
    pub fn label(self) -> &'static str {
        match self {
            ThemeKind::Dark => "🌙 暗夜霓虹",
            ThemeKind::Light => "☀️ 明亮",
            ThemeKind::Neon => "⚡ 赛博霓虹",
        }
    }
}

/// 主题色板
pub struct Palette {
    pub bg_top: Color32,
    pub bg_bottom: Color32,
    pub accent: Color32,
    pub accent2: Color32,
    pub panel: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub stroke: Color32,
}

impl Palette {
    pub fn for_kind(k: ThemeKind) -> Self {
        match k {
            ThemeKind::Dark => Palette {
                bg_top: Color32::from_rgb(10, 12, 34),
                bg_bottom: Color32::from_rgb(28, 18, 58),
                accent: Color32::from_rgb(0, 229, 255),   // 青色
                accent2: Color32::from_rgb(124, 77, 255), // 紫
                panel: Color32::from_rgba_unmultiplied(24, 26, 52, 208),
                text: Color32::from_rgb(230, 235, 250),
                text_dim: Color32::from_rgb(140, 150, 190),
                stroke: Color32::from_rgba_unmultiplied(0, 229, 255, 90),
            },
            ThemeKind::Light => Palette {
                bg_top: Color32::from_rgb(240, 244, 255),
                bg_bottom: Color32::from_rgb(220, 228, 255),
                accent: Color32::from_rgb(56, 96, 255),
                accent2: Color32::from_rgb(156, 39, 176),
                panel: Color32::from_rgba_unmultiplied(255, 255, 255, 235),
                text: Color32::from_rgb(30, 36, 60),
                text_dim: Color32::from_rgb(110, 120, 150),
                stroke: Color32::from_rgba_unmultiplied(56, 96, 255, 120),
            },
            ThemeKind::Neon => Palette {
                bg_top: Color32::from_rgb(5, 6, 18),
                bg_bottom: Color32::from_rgb(38, 6, 40),
                accent: Color32::from_rgb(0, 255, 214),    // 霓虹青绿
                accent2: Color32::from_rgb(255, 64, 129), // 霓虹粉
                panel: Color32::from_rgba_unmultiplied(16, 14, 34, 220),
                text: Color32::from_rgb(235, 240, 250),
                text_dim: Color32::from_rgb(150, 160, 200),
                stroke: Color32::from_rgba_unmultiplied(255, 64, 129, 110),
            },
        }
    }
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgba_unmultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

/// 将主题应用到 egui 全局样式。
pub fn apply(ctx: &egui::Context, kind: ThemeKind) {
    let pal = Palette::for_kind(kind);

    let egui_theme = match kind {
        ThemeKind::Light => egui::Theme::Light,
        _ => egui::Theme::Dark,
    };
    // 关键：先切换当前主题。0.37 下 set_visuals/set_style_of 按主题存储，
    // 渲染用的 style/visuals 取决于 ctx.theme()；不切换则与系统默认主题
    // （如浅色）错配，导致刚打开时主题"乱的"、点主题按钮后才正常。
    ctx.set_theme(egui_theme);

    let mut visuals = match kind {
        ThemeKind::Light => egui::Visuals::light(),
        _ => egui::Visuals::dark(),
    };
    // 覆盖关键样式：让控件融入主题
    visuals.panel_fill = pal.bg_bottom.gamma_multiply(0.55);
    visuals.window_fill = pal.panel;
    visuals.extreme_bg_color = pal.bg_top;
    visuals.override_text_color = Some(pal.text);
    visuals.selection.bg_fill = pal.accent.gamma_multiply(0.55);
    visuals.hyperlink_color = pal.accent;
    visuals.widgets.noninteractive.bg_fill = pal.panel;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, pal.stroke);
    visuals.widgets.inactive.bg_fill = pal.panel;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, pal.text);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, pal.stroke);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(10);
    visuals.widgets.hovered.bg_fill = pal.accent.gamma_multiply(0.18);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.2_f32, pal.accent);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(10);
    visuals.widgets.active.bg_fill = pal.accent.gamma_multiply(0.30);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, pal.accent);
    visuals.widgets.active.corner_radius = CornerRadius::same(10);
    // set_visuals_of 显式绑定主题，不依赖 ctx.theme()（与上面 set_theme 双保险）
    ctx.set_visuals_of(egui_theme, visuals);

    let mut style = (*ctx.style_of(egui_theme)).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(16.0, 9.0);
    style.spacing.window_margin = Margin::same(10);
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::proportional(24.0),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::proportional(15.0),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::proportional(15.0),
    );
    ctx.set_style_of(egui_theme, style);
}

/// 全屏渐变背景 + 装饰光斑（画在 background layer，面板半透明透出）。
pub fn paint_background(ctx: &egui::Context, pal: &Palette, _t: f64) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let rect = ctx.content_rect();

    // 垂直渐变（分条插值，避免依赖已被移除的 mesh gradient API）
    const BANDS: usize = 28;
    for i in 0..BANDS {
        let t0 = i as f32 / BANDS as f32;
        let t1 = (i + 1) as f32 / BANDS as f32;
        let y0 = rect.top() + rect.height() * t0;
        let y1 = rect.top() + rect.height() * t1;
        let c = lerp(pal.bg_top, pal.bg_bottom, (t0 + t1) * 0.5);
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(rect.left(), y0), egui::pos2(rect.right(), y1)),
            0.0,
            c,
        );
    }

    // 装饰光斑：右上青色晕、左下紫色晕（低透明度，营造氛围）
    painter.circle_filled(
        egui::pos2(rect.right() - 100.0, rect.top() + 80.0),
        240.0,
        pal.accent.gamma_multiply(0.05),
    );
    painter.circle_filled(
        egui::pos2(rect.left() + 120.0, rect.bottom() - 140.0),
        300.0,
        pal.accent2.gamma_multiply(0.05),
    );
}

/// 圆角卡片 Frame。
pub fn card(pal: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(pal.panel)
        .stroke(Stroke::new(1.0_f32, pal.stroke))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::same(14))
}

/// 发光主按钮。
pub fn glow_button(ui: &mut egui::Ui, text: &str, pal: &Palette) -> egui::Response {
    let btn = egui::Button::new(
        egui::RichText::new(text).size(15.0).color(pal.text),
    )
    .fill(pal.accent.gamma_multiply(0.16))
    .stroke(Stroke::new(1.5_f32, pal.accent))
    .corner_radius(CornerRadius::same(12));
    ui.add(btn)
}

/// 渐变文字标题：逐字符在两端色之间 gamma 插值（LayoutJob 单段渲染），
/// 避免旧实现按字节硬切导致的单词/字母割裂感。
pub fn gradient_heading(ui: &mut egui::Ui, text: &str, pal: &Palette) {
    use egui::text::{LayoutJob, TextFormat};

    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    if n == 0 {
        return;
    }
    let font_id = egui::FontId::proportional(24.0);
    let mut job = LayoutJob::default();
    for (i, &ch) in chars.iter().enumerate() {
        let t = if n > 1 { i as f32 / (n - 1) as f32 } else { 0.0 };
        let color = pal.accent.lerp_to_gamma(pal.accent2, t);
        job.append(
            &ch.to_string(),
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color,
                ..Default::default()
            },
        );
    }
    ui.label(job);
}
