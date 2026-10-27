//! multiavatar Studio —— 跨平台头像生成工作台（egui/eframe）。
//!
//! 平台支持：
//!   - Windows / Linux / macOS：原生窗口（eframe/winit）
//!   - Android：`android_main` 入口（cargo-ndk / cargo-apk 构建）
//! 发布版在 Windows 上不弹控制台窗口。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod export;
mod preview;
mod theme;

use app::StudioApp;
use eframe::egui;

#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("multiavatar Studio")
            .with_inner_size([1280.0, 840.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "multiavatar Studio",
        options,
        Box::new(|cc| Ok(Box::new(StudioApp::new(cc)))),
    )
}

/// Android 原生入口（cargo-ndk 构建时由应用框架调用）。
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(_app: egui::android::AndroidApp) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("multiavatar Studio"),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "multiavatar Studio",
        options,
        Box::new(|cc| Ok(Box::new(StudioApp::new(cc)))),
    );
}
