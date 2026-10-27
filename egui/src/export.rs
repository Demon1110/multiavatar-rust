//! 导出：文件保存对话框（桌面用 rfd 原生对话框；Android 保存到外部存储目录）。

use std::path::PathBuf;

/// 弹出保存对话框并写入字节。返回保存路径。
#[cfg(not(target_os = "android"))]
pub fn save_bytes(
    bytes: &[u8],
    default_name: &str,
    filters: &[(&str, &[&str])],
) -> Option<PathBuf> {
    let mut dlg = rfd::FileDialog::new().set_file_name(default_name);
    for (name, exts) in filters {
        dlg = dlg.add_filter(*name, exts);
    }
    let path = dlg.save_file()?;
    std::fs::write(&path, bytes).ok()?;
    Some(path)
}

/// Android：无对话框，直接写入外部存储（Downloads）。
#[cfg(target_os = "android")]
pub fn save_bytes(
    bytes: &[u8],
    default_name: &str,
    _filters: &[(&str, &[&str])],
) -> Option<PathBuf> {
    let base = std::env::var("EXTERNAL_STORAGE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    let dir = std::path::Path::new(&base).join("Download");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(default_name);
    std::fs::write(&path, bytes).ok()?;
    Some(path)
}

/// 选择导出目录（批量导出用）。
#[cfg(not(target_os = "android"))]
pub fn pick_dir() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

#[cfg(target_os = "android")]
pub fn pick_dir() -> Option<PathBuf> {
    let base = std::env::var("EXTERNAL_STORAGE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    let dir = std::path::Path::new(&base).join("Download");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir)
}
