//! 生成应用图标 assets/app.ico（多尺寸，Windows exe 资源 + 桌面快捷方式用）。
//!
//! 用法：cargo run --example gen_icon
//! 产物：egui/assets/app.ico（16/32/48/64/128/256 六个尺寸，约几 KB）

fn main() {
    let sizes = [16u32, 32, 48, 64, 128, 256];
    let ico = multiavatar::to_favicon_ico_sized("multiavatar", &sizes);
    std::fs::create_dir_all("assets").expect("create assets dir");
    std::fs::write("assets/app.ico", &ico).expect("write app.ico");
    println!("assets/app.ico written ({} bytes)", ico.len());
}
