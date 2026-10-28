//! 构建脚本：Windows 下把 assets/app.ico 嵌入可执行文件资源
//! （资源管理器/快捷方式显示 exe 图标；跨平台构建时自动跳过）。

fn main() {
    // build.rs 编译于 host，判断的是"目标平台"而不是 host 平台：
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let icon = std::path::Path::new("assets").join("app.ico");
        if icon.exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon(icon.to_str().expect("icon path"));
            if let Err(e) = res.compile() {
                panic!("failed to embed app.ico: {e}");
            }
        } else {
            eprintln!("warn: assets/app.ico 不存在，跳过 exe 图标嵌入（先运行 cargo run --example gen_icon）");
        }
    }
}
