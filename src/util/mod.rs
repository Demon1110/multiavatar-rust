//! 工具函数。

pub mod strings;

/// 计算 SHA-256，输出小写十六进制（与 JS CryptoJS SHA256 的 toString() 一致）。
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(data);
    let mut sb = String::with_capacity(digest.len() * 2);
    for b in digest {
        sb.push_str(&format!("{:02x}", b));
    }
    sb
}

/// 两位补零格式化（与 JS 中 nr.length==1 时补 '0' 的语义一致）。
pub fn two(x: i32) -> String {
    if x < 10 {
        format!("0{}", x)
    } else {
        x.to_string()
    }
}

/// 是否为空字符串（null 视为空）。
pub fn is_empty(s: Option<&str>) -> bool {
    match s {
        None => true,
        Some(v) => v.is_empty(),
    }
}
