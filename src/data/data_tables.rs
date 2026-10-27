//! 静态数据的只读访问入口（单例语义：进程内仅解析一次）。

use std::collections::HashMap;
use std::sync::OnceLock;

use super::multiavatar_data::{parse, Parts, Themes};

/// 颜色主题: [partKey("00A")][partName] -> 颜色列表
pub struct DataTables {
    themes: HashMap<String, HashMap<String, Vec<String>>>,
    parts: HashMap<String, HashMap<String, String>>,
}

impl DataTables {
    /// 获取全局唯一实例（首次调用时解析 gzip 数据，之后复用；线程安全）。
    pub fn instance() -> &'static DataTables {
        static INSTANCE: OnceLock<DataTables> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            let (Themes(themes), Parts(parts)) = parse();
            DataTables { themes, parts }
        })
    }

    /// 取指定「角色+主题」（如 "00A"）下某部件的颜色列表；未命中返回 None。
    pub fn colors_of(&self, part_key: &str, part_name: &str) -> Option<&Vec<String>> {
        self.themes.get(part_key)?.get(part_name)
    }

    /// 取指定角色（如 "00"）下某部件的 SVG 模板；未命中返回 None。
    pub fn template_of(&self, part_v: &str, part_name: &str) -> Option<&str> {
        self.parts.get(part_v)?.get(part_name).map(|s| s.as_str())
    }
}
