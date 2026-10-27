//! 部件键解析器：把 0-47 的编号换算为「初始角色(00-15) + 颜色主题(A/B/C)」的部件键。
//!
//! 对应 multiavatar.js：`nr>31 → nr-32+'C'；nr>15 → nr-16+'B'；否则 nr+'A'`，
//! 其中角色号补零到两位。

use crate::util::two;

/// 编号 → 部件键（如 16 → "00B"，35 → "03C"，7 → "07A"）。
pub fn resolve(nr: i32) -> String {
    let (base, theme) = if nr > 31 {
        (nr - 32, "C")
    } else if nr > 15 {
        (nr - 16, "B")
    } else {
        (nr, "A")
    };
    format!("{}{}", two(base), theme)
}
