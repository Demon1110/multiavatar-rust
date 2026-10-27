//! 字符串小工具。

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
