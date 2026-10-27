//! 部件编号映射器：把 12 位数字 hash 转换为 6 个部件各自的编号（0-47）。
//!
//! 对应 multiavatar.js：每 2 位数字经 `Math.round((47/100) * 两位)` 映射为 0-47。

/// 六个部件的固定定义顺序（与 JS 的 p 对象一致）。
pub const PARTS: [&str; 6] = ["env", "clo", "head", "mouth", "eyes", "top"];

const SCALE: f64 = 47.0 / 100.0;

/// 计算各部件编号：部件名 -> 0-47 编号（按 [PARTS] 顺序）。
pub fn map(hash12: &str) -> Vec<(String, i32)> {
    PARTS
        .iter()
        .enumerate()
        .map(|(i, part)| ((*part).to_string(), map_to47(two_digits(hash12, i * 2))))
        .collect()
}

/// 对应 JS：Math.round((47/100) * 两位数字)，返回 0-47。
fn map_to47(two_digits: i32) -> i32 {
    (SCALE * f64::from(two_digits)).round() as i32
}

/// 从 hash 中取从 idx 开始的两位数字；不足两位返回 0。
fn two_digits(hash: &str, idx: usize) -> i32 {
    let bytes = hash.as_bytes();
    if idx + 2 > bytes.len() {
        return 0;
    }
    (bytes[idx] - b'0') as i32 * 10 + (bytes[idx + 1] - b'0') as i32
}
