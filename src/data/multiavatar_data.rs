//! 头像静态数据（颜色主题 + SVG 部件模板）。
//!
//! 数据来源：multiavatar.js 提取结果（与 Java 版同一份 gzip 资源），
//! 编译期内嵌（include_bytes!），首次访问时解压一次并构建只读结构。
//!
//! 行格式（UTF-8）：
//! - `T|<partKey>|<partName>|<c1>,<c2>,...` —— 主题颜色
//! - `P|<partKey>|<partName>|<utf8字节数>:<svg>` —— 部件模板

use std::collections::HashMap;
use std::io::Read;

use flate2::read::GzDecoder;

use super::DATA_GZ;

/// 颜色主题: THEMES[partKey(角色2位+主题,如"00A")][partName] -> 颜色列表(可为"none")
pub struct Themes(pub HashMap<String, HashMap<String, Vec<String>>>);

/// SVG 部件模板: PARTS[partKey(角色2位,如"00")][partName] -> SVG 片段(含 #xxx; 颜色占位符)
pub struct Parts(pub HashMap<String, HashMap<String, String>>);

/// 一次性解析 gzip 资源：返回 (主题, 部件)。
pub fn parse() -> (Themes, Parts) {
    let mut themes: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
    let mut parts: HashMap<String, HashMap<String, String>> = HashMap::new();

    let mut decoder = GzDecoder::new(DATA_GZ);
    let mut content = String::new();
    decoder
        .read_to_string(&mut content)
        .expect("解压 multiavatar-data.gz 失败");
    if !content.is_ascii() && !content.is_empty() {
        // 资源为 UTF-8 文本（read_to_string 已按 UTF-8 校验）
    }

    for line in content.lines() {
        if line.is_empty() {
            continue;
        }
        let kind = line.as_bytes()[0];
        if kind == b'T' {
            // T|<partKey>|<partName>|<c1>,<c2>,...
            let mut it = line[2..].splitn(3, '|');
            let part_key = it.next().unwrap_or("").to_string();
            let part_name = it.next().unwrap_or("").to_string();
            let colors = it
                .next()
                .unwrap_or("")
                .split(',')
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            themes
                .entry(part_key)
                .or_default()
                .insert(part_name, colors);
        } else {
            // P|<partKey>|<partName>|<utf8字节数>:<svg>
            let mut it = line[2..].splitn(3, '|');
            let part_key = it.next().unwrap_or("").to_string();
            let part_name = it.next().unwrap_or("").to_string();
            let body = it.next().unwrap_or("").to_string();
            // body = <len>:<svg>（len 为 UTF-8 字节数，仅用于数据校验/截取）
            let svg = match body.split_once(':') {
                Some((_len, s)) => s.to_string(),
                None => body,
            };
            parts
                .entry(part_key)
                .or_default()
                .insert(part_name, svg);
        }
    }

    (Themes(themes), Parts(parts))
}
