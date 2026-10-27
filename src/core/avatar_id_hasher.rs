//! 哈希策略：字符串 → 12 位数字（与 multiavatar.js 逐位一致）。

use crate::util::sha256_hex;

/// 哈希策略接口：把输入字符串映射为头像标识数字串。
pub trait AvatarIdHasher {
    /// 返回 12 位数字字符串（可能少于 12 位，此时按 JS 语义截断）。
    fn hash12(&self, input: &str) -> String;
}

/// 默认哈希策略：SHA-256 取前 12 位数字。
///
/// 与 multiavatar.js 行为逐位一致：
/// `sha256Hex = SHA256(utf8(input))` → 去除非数字字符 → 取前 12 位。
pub struct Sha256AvatarIdHasher;

impl AvatarIdHasher for Sha256AvatarIdHasher {
    fn hash12(&self, input: &str) -> String {
        let hex = sha256_hex(input.as_bytes());
        // JS: hex.replace(/\D/g, '') —— 去掉所有非数字字符（仅 ASCII 数字）
        // 再 substring(0, 12)
        hex.chars()
            .filter(|c| c.is_ascii_digit())
            .take(12)
            .collect()
    }
}
