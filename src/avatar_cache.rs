//! 头像 LRU 缓存：同参数重复生成免重复计算（容量 128）。
//!
//! 缓存键 = 输入字符串 + 选项（**不含输出 size 与 format**）——同一 Avatar 的
//! 多尺寸/多格式渲染复用同一份 SVG（图像按需惰性渲染，见 [crate::Avatar]）。

use std::num::NonZeroUsize;
use std::sync::{Arc, LazyLock, Mutex};

use lru::LruCache;

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

/// 全局 LRU 缓存（线程安全）。
pub struct AvatarCache;

static CACHE: LazyLock<Mutex<LruCache<CacheKey, Arc<Avatar>>>> = LazyLock::new(|| {
    Mutex::new(LruCache::new(
        NonZeroUsize::new(CACHE_CAPACITY).expect("cache capacity > 0"),
    ))
});

impl AvatarCache {
    /// 读取缓存。
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
