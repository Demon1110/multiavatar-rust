//! 头像部件规格（不可变）：记录 6 个部件各自选中的初始角色与颜色主题。
//!
//! 由 [resolve] 从输入推导，也可通过强制指定 part/theme（对应 JS 的 ver 参数）
//! 覆盖任意部件的选择。

use super::avatar_id_hasher::AvatarIdHasher;
use super::part_key_resolver;
use super::part_number_mapper;

/// 输出拼装顺序（与 multiavatar.js 一致）：env→head→clo→top→eyes→mouth。
pub const PART_ORDER: [&str; 6] = ["env", "head", "clo", "top", "eyes", "mouth"];

/// 头像部件规格（不可变）。
#[derive(Debug, Clone)]
pub struct AvatarSpec {
    /// 部件名 -> 部件键（如 "env" -> "00A"）
    part_keys: Vec<(String, String)>,
}

impl AvatarSpec {
    /// 从输入推导规格。
    ///
    /// - `input`：非空输入字符串（调用方需保证非空）
    /// - `hasher`：哈希策略
    /// - `forced_part`：强制角色编号（对应 JS 的 ver.part），None 表示自动
    /// - `forced_theme`：强制主题（对应 JS 的 ver.theme），None 表示自动
    pub fn resolve(
        input: &str,
        hasher: &dyn AvatarIdHasher,
        forced_part: Option<&str>,
        forced_theme: Option<&str>,
    ) -> AvatarSpec {
        let numbers = part_number_mapper::map(&hasher.hash12(input));

        let mut keys: Vec<(String, String)> = numbers
            .iter()
            .map(|(part, nr)| (part.clone(), part_key_resolver::resolve(*nr)))
            .collect();

        // 强制指定：所有部件统一使用（JS: if (ver) partV=ver.part; theme=ver.theme）
        if forced_part.is_some() || forced_theme.is_some() {
            for (_, k) in keys.iter_mut() {
                let part_v = forced_part.unwrap_or(&k[0..2]);
                let theme = forced_theme.unwrap_or(&k[2..3]);
                *k = format!("{}{}", part_v, theme);
            }
        }

        AvatarSpec { part_keys: keys }
    }

    /// 部件名（env/clo/head/mouth/eyes/top）→ 部件键（如 "00A"）。
    pub fn part_key_of(&self, part_name: &str) -> Option<&str> {
        self.part_keys
            .iter()
            .find(|(p, _)| p == part_name)
            .map(|(_, k)| k.as_str())
    }

    /// 部件名 → 初始角色编号（2 位，如 "00"）。
    pub fn part_v_of(&self, part_name: &str) -> Option<&str> {
        self.part_key_of(part_name).map(|k| &k[0..2])
    }

    /// 部件名 → 颜色主题（"A"/"B"/"C"）。
    pub fn theme_of(&self, part_name: &str) -> Option<&str> {
        self.part_key_of(part_name).map(|k| &k[2..3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::avatar_id_hasher::Sha256AvatarIdHasher;

    #[test]
    fn resolve_auto_keys() {
        let spec = AvatarSpec::resolve("Binx Bond", &Sha256AvatarIdHasher, None, None);
        // 6 个部件都有键
        for part in PART_ORDER {
            assert!(spec.part_key_of(part).is_some());
        }
        // 键格式：2 位角色 + A/B/C
        for part in PART_ORDER {
            let k = spec.part_key_of(part).unwrap();
            assert_eq!(k.len(), 3);
            assert!(k.ends_with('A') || k.ends_with('B') || k.ends_with('C'));
        }
    }

    #[test]
    fn resolve_forced() {
        let spec = AvatarSpec::resolve(
            "Binx Bond",
            &Sha256AvatarIdHasher,
            Some("07"),
            Some("B"),
        );
        for part in PART_ORDER {
            assert_eq!(spec.part_key_of(part), Some("07B"));
        }
    }

    #[test]
    fn resolve_forced_part_only() {
        let spec = AvatarSpec::resolve("Binx Bond", &Sha256AvatarIdHasher, Some("00"), None);
        for part in PART_ORDER {
            let k = spec.part_key_of(part).unwrap();
            assert_eq!(&k[0..2], "00");
            assert!(k.ends_with('A') || k.ends_with('B') || k.ends_with('C'));
        }
    }
}
