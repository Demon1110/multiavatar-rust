//! multiavatar Studio —— 跨平台头像生成工作台。
//!
//! 场景定位（面向大众用户）：
//!   1. 个人头像：输入名字/单词/任意字符串 → 立刻得到一张独一无二的多文化风格头像；
//!   2. 开发者工具：一键导出 SVG / PNG / JPEG / GIF / ICO / favicon.zip，适配网页图标与 UI 占位；
//!   3. 批量生成：为团队、账号、资源批量生成统一风格头像（网格预览 + 目录批量导出）；
//!   4. 素材复用：复制 data-URI 直接嵌入代码/文档；历史记录一键回选。

use crate::{export, theme};
use crate::preview::{bitmap_preview, color_image_texture, BitmapPreview, GifPlayer};
use crate::theme::{Palette, ThemeKind};
use eframe::egui::{self, Color32, RichText};

/// 预览/导出格式。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PreviewFormat {
    Svg,
    Png,
    Jpg,
    Gif,
}

impl PreviewFormat {
    const ALL: [PreviewFormat; 4] = [PreviewFormat::Svg, PreviewFormat::Png, PreviewFormat::Jpg, PreviewFormat::Gif];
    fn label(self) -> &'static str {
        match self {
            PreviewFormat::Svg => "SVG 矢量",
            PreviewFormat::Png => "PNG 位图",
            PreviewFormat::Jpg => "JPEG 位图",
            PreviewFormat::Gif => "GIF 动画",
        }
    }
}

/// 导出格式清单（供按钮组使用）。
enum ExportKind {
    Svg,
    Png,
    Jpg,
    Gif,
    Ico,
    Favzip,
}

pub struct StudioApp {
    theme: ThemeKind,
    palette: Palette,

    input: String,
    size: u32,
    format: PreviewFormat,

    // 预览缓存（避免每帧重算）
    cache_key: Option<(String, u32, PreviewFormat)>,
    png_preview: Option<BitmapPreview>,
    gif: Option<GifPlayer>,
    gif_texture: Option<egui::TextureHandle>,
    gif_frame_idx: usize,
    svg_len: usize,

    // 历史
    history: Vec<String>,

    // 批量
    batch_text: String,
    grid: Vec<(String, egui::TextureHandle)>,
    grid_size: usize,

    // 状态栏
    status: String,
    status_ok: bool,

    rng_state: u64,
}

impl StudioApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 中文字体：探测系统字体（不打包字体，保持二进制小）
        install_cjk_fonts(&cc.egui_ctx);
        let theme = ThemeKind::Dark;
        theme::apply(&cc.egui_ctx, theme);
        Self {
            palette: Palette::for_kind(theme),
            theme,
            input: "Neo".into(),
            size: 256,
            format: PreviewFormat::Png,
            cache_key: None,
            png_preview: None,
            gif: None,
            gif_texture: None,
            gif_frame_idx: 0,
            svg_len: 0,
            history: Vec::new(),
            batch_text: "Alice\nBob\nCarol\nEve\nMallory".into(),
            grid: Vec::new(),
            grid_size: 96,
            status: "就绪".into(),
            status_ok: true,
            rng_state: 0x9E3779B97F4A7C15,
        }
    }

    fn set_status(&mut self, ok: bool, msg: impl Into<String>) {
        self.status = msg.into();
        self.status_ok = ok;
    }

    // ---- 随机名字（无 rand 依赖，LCG） ----
    fn random_name(&mut self) -> String {
        const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut s = String::new();
        for _ in 0..10 {
            self.rng_state = self
                .rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            s.push(CHARS[(self.rng_state >> 33) as usize % CHARS.len()] as char);
        }
        s
    }

    fn push_history(&mut self) {
        let v = self.input.trim().to_string();
        if v.is_empty() {
            return;
        }
        if let Some(i) = self.history.iter().position(|h| h == &v) {
            self.history.remove(i);
        }
        self.history.insert(0, v);
        self.history.truncate(30);
    }

    // ---- 预览管线：仅在输入/尺寸/格式变化时重算 ----
    fn ensure_preview(&mut self, ctx: &egui::Context) {
        let key = (self.input.clone(), self.size, self.format);
        let cache_hit = self.cache_key.as_ref() == Some(&key)
            && (self.png_preview.is_some() || self.gif.is_some() || self.input.trim().is_empty());
        if cache_hit {
            return;
        }
        self.cache_key = Some(key);
        self.png_preview = None;
        self.gif = None;
        self.gif_texture = None;

        let input = self.input.trim();
        if input.is_empty() {
            self.svg_len = 0;
            return;
        }

        match self.format {
            PreviewFormat::Png => {
                let bytes = multiavatar::to_png_sized(input, self.size);
                self.png_preview = bitmap_preview(ctx, &bytes, "png");
                self.svg_len = multiavatar::multiavatar(input).len();
            }
            PreviewFormat::Jpg => {
                let bytes = multiavatar::to_jpg_sized(input, self.size);
                self.png_preview = bitmap_preview(ctx, &bytes, "jpg");
                self.svg_len = multiavatar::multiavatar(input).len();
            }
            PreviewFormat::Gif => {
                let bytes = multiavatar::to_gif_sized(input, self.size);
                self.gif = GifPlayer::new(&bytes);
                self.gif_frame_idx = 0;
                self.svg_len = multiavatar::multiavatar(input).len();
            }
            PreviewFormat::Svg => {
                let svg = multiavatar::multiavatar(input);
                self.svg_len = svg.len();
                // SVG 无位图解码器：用 PNG 渲染版预览（结果一致）
                let bytes = multiavatar::to_png_sized(input, self.size);
                self.png_preview = bitmap_preview(ctx, &bytes, "svg");
            }
        }
    }

    // ---- 顶部栏 ----
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            theme::gradient_heading(ui, "multiavatar Studio", &self.palette);
            ui.add_space(16.0);
            ui.label(
                RichText::new("跨平台头像生成工作台")
                    .size(13.0)
                    .color(self.palette.text_dim),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new("🎲 随机").color(self.palette.accent))
                    .clicked()
                {
                    self.input = self.random_name();
                    self.push_history();
                    self.set_status(true, format!("随机生成：{}", self.input));
                }
                ui.add_space(6.0);
                for k in ThemeKind::ALL {
                    let selected = self.theme == k;
                    if ui
                        .selectable_label(selected, RichText::new(k.label()).size(13.0))
                        .clicked()
                    {
                        self.theme = k;
                        theme::apply(&ctx, k);
                        self.palette = Palette::for_kind(k);
                    }
                }
            });
        });
        ui.add_space(8.0);
    }

    // ---- 左栏：输入 / 尺寸 / 格式 / 导出 / 历史 ----
    fn left_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("left")
            .default_size(310.0)
            .resizable(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    // 输入
                    theme::card(&self.palette).show(ui, |ui| {
                        ui.label(RichText::new("✏️ 输入文字").strong().color(self.palette.accent));
                        ui.add(
                            egui::TextEdit::multiline(&mut self.input)
                                .desired_rows(3)
                                .desired_width(f32::INFINITY)
                                .hint_text("任意字符串：名字 / 单词 / 昵称 …"),
                        );
                        ui.horizontal(|ui| {
                            if ui.button("⚡ 生成").clicked() {
                                self.push_history();
                                self.set_status(true, "已生成");
                            }
                            if ui.button("🎲 随机").clicked() {
                                self.input = self.random_name();
                                self.push_history();
                            }
                        });
                    });

                    // 历史
                    if !self.history.is_empty() {
                        egui::CollapsingHeader::new(format!("🕘 历史（{}）", self.history.len()))
                            .default_open(false)
                            .show(ui, |ui| {
                                for h in self.history.iter().take(10) {
                                    if ui
                                        .selectable_label(false, RichText::new(h.as_str()).size(13.0))
                                        .clicked()
                                    {
                                        self.input = h.clone();
                                        self.cache_key = None;
                                    }
                                }
                            });
                    }

                    ui.add_space(6.0);
                    // 尺寸与选项
                    theme::card(&self.palette).show(ui, |ui| {
                        ui.label(RichText::new("📐 尺寸").strong().color(self.palette.accent));
                        ui.horizontal(|ui| {
                            ui.label("尺寸");
                            ui.add(egui::Slider::new(&mut self.size, 64..=1024).logarithmic(true).suffix(" px"));
                        });
                        ui.label(
                            RichText::new(format!("输出约 {size}×{size} px", size = self.size))
                                .size(12.0)
                                .color(self.palette.text_dim),
                        );
                    });

                    ui.add_space(6.0);
                    // 格式
                    theme::card(&self.palette).show(ui, |ui| {
                        ui.label(RichText::new("🖼️ 预览格式").strong().color(self.palette.accent));
                        egui::ComboBox::from_id_salt("fmt")
                            .selected_text(self.format.label())
                            .show_ui(ui, |ui| {
                                for f in PreviewFormat::ALL {
                                    ui.selectable_value(&mut self.format, f, f.label());
                                }
                            });
                        if ui.button("❓ 格式说明").clicked() {
                            self.set_status(
                                false,
                                "SVG 矢量可无限缩放；PNG 无损；JPEG 体积小（白底）；GIF 多帧动画；ICO/favicon.zip 用于网站图标",
                            );
                        }
                    });

                    ui.add_space(6.0);
                    // 导出
                    theme::card(&self.palette).show(ui, |ui| {
                        ui.label(RichText::new("💾 导出").strong().color(self.palette.accent2));
                        if self.input.trim().is_empty() {
                            ui.label(RichText::new("先输入文字").color(self.palette.text_dim));
                        } else {
                            ui.horizontal_wrapped(|ui| {
                                if theme::glow_button(ui, "SVG", &self.palette).clicked() {
                                    self.export_current(ExportKind::Svg);
                                }
                                if theme::glow_button(ui, "PNG", &self.palette).clicked() {
                                    self.export_current(ExportKind::Png);
                                }
                                if theme::glow_button(ui, "JPEG", &self.palette).clicked() {
                                    self.export_current(ExportKind::Jpg);
                                }
                                if theme::glow_button(ui, "GIF", &self.palette).clicked() {
                                    self.export_current(ExportKind::Gif);
                                }
                                if theme::glow_button(ui, "ICO", &self.palette).clicked() {
                                    self.export_current(ExportKind::Ico);
                                }
                                if theme::glow_button(ui, "favicon.zip", &self.palette).clicked() {
                                    self.export_current(ExportKind::Favzip);
                                }
                            });
                            if ui.button("📋 复制 data-URI").clicked() {
                                let uri = multiavatar::to_data_uri_sized(&self.input, self.size);
                                ui.ctx().copy_text(uri);
                                self.set_status(true, "data-URI 已复制到剪贴板");
                            }
                        }
                    });
                    ui.add_space(10.0);
                });
            });
    }

    fn export_current(&mut self, kind: ExportKind) {
        let name = self.input.trim().to_string();
        if name.is_empty() {
            return;
        }
        let base = sanitize(&name);
        let result = match kind {
            ExportKind::Svg => {
                let bytes = multiavatar::multiavatar(&name).into_bytes();
                export::save_bytes(&bytes, &format!("{base}.svg"), &[("SVG", &["svg"])])
            }
            ExportKind::Png => {
                let bytes = multiavatar::to_png_sized(&name, self.size);
                export::save_bytes(&bytes, &format!("{base}.png"), &[("PNG", &["png"])])
            }
            ExportKind::Jpg => {
                let bytes = multiavatar::to_jpg_sized(&name, self.size);
                export::save_bytes(&bytes, &format!("{base}.jpg"), &[("JPEG", &["jpg", "jpeg"])])
            }
            ExportKind::Gif => {
                let bytes = multiavatar::to_gif_sized(&name, self.size);
                export::save_bytes(&bytes, &format!("{base}.gif"), &[("GIF", &["gif"])])
            }
            ExportKind::Ico => {
                let bytes = multiavatar::to_favicon_ico(&name);
                export::save_bytes(&bytes, &format!("{base}.ico"), &[("ICO", &["ico"])])
            }
            ExportKind::Favzip => {
                let bytes = multiavatar::to_favicon_zip(&name);
                export::save_bytes(
                    &bytes,
                    &format!("{base}-favicons.zip"),
                    &[("ZIP", &["zip"])],
                )
            }
        };
        match result {
            Some(path) => self.set_status(true, format!("已导出：{}", path.display())),
            None => self.set_status(false, "已取消或保存失败"),
        }
    }

    // ---- 中央：大预览 ----
    fn center(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            let ctx = ui.ctx().clone();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("👤 {}", self.input.trim()))
                        .size(18.0)
                        .strong()
                        .color(self.palette.text),
                );
                ui.label(
                    RichText::new(format!("输入字符 {} 个", self.input.trim().chars().count()))
                        .size(12.0)
                        .color(self.palette.text_dim),
                );
            });
            ui.add_space(10.0);

            let avail = ui.available_size();
            let max_w = (avail.x - 40.0).min(560.0).max(120.0);
            let img_size = max_w.min(avail.y - 140.0).max(64.0);

            ui.vertical_centered(|ui| {
                let card = theme::card(&self.palette)
                    .corner_radius(egui::CornerRadius::same(20))
                    .stroke(egui::Stroke::new(1.5_f32, self.palette.accent.gamma_multiply(0.5)));
                card.show(ui, |ui| {
                    ui.add_space(8.0);
                    if self.input.trim().is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(img_size * 0.35);
                            ui.label(
                                RichText::new("输入任意文字开始生成 ✨")
                                    .size(20.0)
                                    .color(self.palette.text_dim),
                            );
                            ui.add_space(img_size * 0.35);
                        });
                    } else {
                        match self.format {
                            PreviewFormat::Gif => {
                                if let Some(gif) = &self.gif {
                                    let now = ui.input(|i| i.time);
                                    let idx = gif.frame_at(now);
                                    if idx != self.gif_frame_idx || self.gif_texture.is_none() {
                                        self.gif_frame_idx = idx;
                                        self.gif_texture = Some(color_image_texture(
                                            &ctx,
                                            gif.image(idx),
                                            &format!("gif-frame-{idx}"),
                                        ));
                                    }
                                    if let Some(tex) = &self.gif_texture {
                                        ui.add(
                                            egui::Image::new(tex)
                                                .max_size(egui::vec2(img_size, img_size))
                                                .corner_radius(egui::CornerRadius::same(16)),
                                        );
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "GIF 动画 · {} 帧 · {}",
                                            gif.frame_count(),
                                            self.fmt_bytes(0)
                                        ))
                                        .size(12.0)
                                        .color(self.palette.text_dim),
                                    );
                                } else {
                                    ui.label("GIF 解码失败");
                                }
                            }
                            _ => {
                                if let Some(pv) = &self.png_preview {
                                    ui.add(
                                        egui::Image::new(&pv.texture)
                                            .max_size(egui::vec2(img_size, img_size))
                                            .corner_radius(egui::CornerRadius::same(16)),
                                    );
                                    let extra = if self.format == PreviewFormat::Svg {
                                        format!(
                                            " · SVG {} 字节",
                                            self.svg_len
                                        )
                                    } else {
                                        String::new()
                                    };
                                    ui.label(
                                        RichText::new(format!(
                                            "{}×{} · {} 字节{}",
                                            pv.width,
                                            pv.height,
                                            self.fmt_bytes(pv.encoded_bytes),
                                            extra
                                        ))
                                        .size(12.0)
                                        .color(self.palette.text_dim),
                                    );
                                } else {
                                    ui.label("生成失败");
                                }
                            }
                        }
                    }
                    ui.add_space(8.0);
                });
            });

            // 状态栏
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let c = if self.status_ok {
                    self.palette.accent
                } else {
                    Color32::from_rgb(255, 120, 120)
                };
                ui.label(RichText::new(&self.status).size(13.0).color(c));
            });
        });
    }

    fn fmt_bytes(&self, n: usize) -> String {
        if n >= 1_048_576 {
            format!("{:.2} MB", n as f64 / 1_048_576.0)
        } else if n >= 1024 {
            format!("{:.1} KB", n as f64 / 1024.0)
        } else {
            format!("{n} B")
        }
    }

    // ---- 底部：批量生成 ----
    fn bottom_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("bottom")
            .resizable(true)
            .default_size(220.0)
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    theme::gradient_heading(ui, "批量生成", &self.palette);
                    ui.label(
                        RichText::new("每行一个名字，统一风格头像")
                            .size(12.0)
                            .color(self.palette.text_dim),
                    );
                });
                ui.add_space(6.0);
                egui::Frame::new()
                    .fill(self.palette.panel)
                    .stroke(egui::Stroke::new(1.0_f32, self.palette.stroke))
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.batch_text)
                                        .desired_rows(4)
                                        .desired_width(260.0)
                                        .hint_text("每行一个名字"),
                                );
                                ui.horizontal(|ui| {
                                    if ui.button("🖼️ 生成网格").clicked() {
                                        self.generate_grid(ui.ctx());
                                    }
                                    if !self.grid.is_empty() && ui.button("📁 批量导出 (PNG)").clicked() {
                                        self.export_grid();
                                    }
                                });
                            });
                            ui.add_space(12.0);
                            // 网格
                            if self.grid.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(30.0);
                                    ui.label(
                                        RichText::new("点击「生成网格」预览全部头像")
                                            .color(self.palette.text_dim),
                                    );
                                });
                            } else {
                                egui::ScrollArea::horizontal().show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        for (name, tex) in &self.grid {
                                            ui.vertical(|ui| {
                                                ui.add(
                                                    egui::Image::new(tex)
                                                        .max_size(egui::vec2(
                                                            self.grid_size as f32,
                                                            self.grid_size as f32,
                                                        ))
                                                        .corner_radius(egui::CornerRadius::same(10)),
                                                );
                                                ui.label(
                                                    RichText::new(name.as_str())
                                                        .size(11.0)
                                                        .color(self.palette.text_dim),
                                                );
                                            });
                                        }
                                    });
                                });
                            }
                        });
                    });
                ui.add_space(6.0);
            });
    }

    fn generate_grid(&mut self, ctx: &egui::Context) {
        self.grid.clear();
        for line in self.batch_text.lines() {
            let name = line.trim();
            if name.is_empty() {
                continue;
            }
            let bytes = multiavatar::to_png_sized(name, 128);
            if let Some(img) = image::load_from_memory(&bytes).ok() {
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                let ci = egui::ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw());
                let tex = ctx.load_texture(
                    format!("grid-{name}-{}", self.grid.len()),
                    ci,
                    egui::TextureOptions::LINEAR,
                );
                self.grid.push((name.to_string(), tex));
            }
        }
        self.set_status(true, format!("网格已生成：{} 个头像", self.grid.len()));
    }

    fn export_grid(&mut self) {
        if self.grid.is_empty() {
            return;
        }
        let Some(dir) = export::pick_dir() else {
            self.set_status(false, "已取消选择目录");
            return;
        };
        let mut ok = 0;
        for (name, _) in &self.grid {
            let bytes = multiavatar::to_png_sized(name, self.size);
            let path = dir.join(format!("{}.png", sanitize(name)));
            if std::fs::write(&path, bytes).is_ok() {
                ok += 1;
            }
        }
        self.set_status(true, format!("批量导出完成：{ok}/{} 个 → {}", self.grid.len(), dir.display()));
    }
}

/// 文件名安全化：去非法字符。
fn sanitize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("avatar");
    }
    out
}

/// 探测系统 CJK 字体并注入 egui（Windows/Linux/macOS/Android）。
fn install_cjk_fonts(ctx: &egui::Context) {
    const CANDIDATES: &[&str] = &[
        // Windows
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyh.ttf",
        "C:\\Windows\\Fonts\\simhei.ttf",
        // macOS
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        // Linux
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/wenquanyi/wqy-zenhei/wqy-zenhei.ttc",
        // Android
        "/system/fonts/NotoSansCJK-Regular.ttc",
        "/system/fonts/DroidSansFallback.ttf",
    ];
    for p in CANDIDATES {
        if let Ok(bytes) = std::fs::read(p) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "cjk".to_owned(),
                egui::FontData::from_owned(bytes).into(),
            );
            for family in [
                egui::FontFamily::Proportional,
                egui::FontFamily::Monospace,
            ] {
                fonts.families.entry(family).or_default().push("cjk".to_owned());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
    // 找不到中文字体时回退默认（英文可用，中文显示方块）
}

impl eframe::App for StudioApp {
    // eframe 0.37：App::ui 接收根 Ui（无 margin/背景），面板通过 ui 嵌套
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // 全屏渐变背景（最底层，面板半透明透出）
        theme::paint_background(&ctx, &self.palette, ctx.input(|i| i.time));

        self.ensure_preview(&ctx);
        egui::Panel::top("top").show(ui, |ui| self.top_bar(ui));
        egui::Panel::left("left").default_size(310.0).resizable(true).show(ui, |ui| {
            self.left_panel(ui);
        });
        egui::CentralPanel::default().show(ui, |ui| {
            self.center(ui);
        });
        egui::Panel::bottom("bottom").resizable(true).default_size(220.0).show(ui, |ui| {
            self.bottom_panel(ui);
        });
    }
}
