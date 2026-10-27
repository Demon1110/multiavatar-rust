//! 头像 LRU 缓存：同参数重复生成免重复计算（容量 128）。
//!
//! 缓存键 = 输入字符串 + 选项（**不含输出 size 与 format**）——同一 Avatar 的
//! 多尺寸/多格式渲染复用同一份 SVG（图像按需惰性渲染，见 [crate::Avatar]）。
//! LRU 本体手写（std HashMap + VecDeque），零 lru crate 依赖。

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::{Arc, LazyLock, Mutex};

use crate::Avatar;

/// 默认缓存容量。
pub const CACHE_CAPACITY: usize = 128;

/// 缓存键（输入 + 影响 SVG 文本的选项）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    input: String,
    sans_env: bool,
    part: Option<String>,
    theme: Option<String>,
    svg_width: i32,
    svg_height: i32,
    optimize_svg: bool,
}

impl CacheKey {
    pub fn new(
        input: &str,
        sans_env: bool,
        part: Option<&str>,
        theme: Option<&str>,
        svg_width: i32,
        svg_height: i32,
        optimize_svg: bool,
    ) -> CacheKey {
        CacheKey {
            input: input.to_string(),
            sans_env,
            part: part.map(str::to_string),
            theme: theme.map(str::to_string),
            svg_width,
            svg_height,
            optimize_svg,
        }
    }
}

/// 手写 LRU：`map` 存值、`order` 存最近使用顺序（队首最旧、队尾最新）。
/// 容量很小（128），`touch` 的 O(n) 重排成本可忽略。
struct LruCache<K, V> {
    map: HashMap<K, V>,
    order: VecDeque<K>,
    cap: usize,
}

impl<K: Clone + Eq + Hash, V> LruCache<K, V> {
    fn new(cap: usize) -> Self {
        LruCache {
            map: HashMap::with_capacity(cap),
            order: VecDeque::new(),
            cap,
        }
    }

    fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            self.touch(key);
            self.map.get(key)
        } else {
            None
        }
    }

    fn put(&mut self, key: K, value: V) {
        if self.map.contains_key(&key) {
            self.map.insert(key.clone(), value);
            self.touch(&key);
            return;
        }
        if self.map.len() == self.cap {
            // 淘汰最久未用（队首仍存在于 map 的 key）
            while let Some(old) = self.order.pop_front() {
                if self.map.remove(&old).is_some() {
                    break;
                }
            }
        }
        self.map.insert(key.clone(), value);
        self.order.push_back(key);
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    /// 把 key 移到队尾（标记为最近使用）。
    fn touch(&mut self, key: &K) {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            let k = self.order.remove(pos).expect("位置已存在");
            self.order.push_back(k);
        }
    }
}

/// 全局 LRU 缓存（线程安全）。
pub struct AvatarCache;

static CACHE: LazyLock<Mutex<LruCache<CacheKey, Arc<Avatar>>>> = LazyLock::new(|| {
    Mutex::new(LruCache::new(CACHE_CAPACITY))
});

impl AvatarCache {
    /// 读取缓存（命中即标记最近使用）。
    pub fn get(key: &CacheKey) -> Option<Arc<Avatar>> {
        CACHE
            .lock()
            .expect("avatar cache poisoned")
            .get(key)
            .cloned()
    }

    /// 写入缓存（超出容量自动淘汰最久未用）。
    pub fn put(key: CacheKey, avatar: Arc<Avatar>) {
        CACHE.lock().expect("avatar cache poisoned").put(key, avatar);
    }

    /// 当前缓存条目数。
    pub fn size() -> usize {
        CACHE.lock().expect("avatar cache poisoned").len()
    }

    /// 清空缓存。
    pub fn clear() {
        CACHE.lock().expect("avatar cache poisoned").clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lru_basic_hit_miss() {
        let mut c: LruCache<i32, &str> = LruCache::new(3);
        assert_eq!(c.get(&1), None);
        c.put(1, "a");
        c.put(2, "b");
        c.put(3, "c");
        assert_eq!(c.get(&2), Some(&"b"));
        assert_eq!(c.len(), 3);
    }

    #[test]
    fn lru_evicts_least_recently_used() {
        let mut c: LruCache<i32, &str> = LruCache::new(2);
        c.put(1, "a");
        c.put(2, "b");
        c.get(&1); // 1 变最近
        c.put(3, "c"); // 淘汰 2（最久未用）
        assert_eq!(c.get(&2), None);
        assert_eq!(c.get(&1), Some(&"a"));
        assert_eq!(c.get(&3), Some(&"c"));
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn lru_update_and_clear() {
        let mut c: LruCache<i32, &str> = LruCache::new(2);
        c.put(1, "a");
        c.put(1, "b"); // 覆盖不增条目
        assert_eq!(c.len(), 1);
        assert_eq!(c.get(&1), Some(&"b"));
        c.clear();
        assert_eq!(c.len(), 0);
        assert_eq!(c.get(&1), None);
    }
}
