# multiavatar (Rust)

[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![rust-version](https://img.shields.io/badge/rust-1.80%2B-orange)
[![GitHub](https://img.shields.io/badge/repo-GitHub-181717?logo=github)](https://github.com/Demon1110/multiavatar-rust)

Multiavatar 的**纯 Rust 全功能移植**（原始 JavaScript：[multiavatar.js](https://github.com/multiavatar/Multiavatar)，作者 Gie Katon）。
基于输入字符串的 SHA-256 哈希派生 12 位随机种子，映射 12 个环境/部件槽位，从内置 12×16×3 主题部件库拼装成确定性头像，
支持 **SVG / PNG / JPEG / GIF / favicon(zip) / favicon.ico / PNG data URI** 七种输出，内置 **LRU 缓存**与 **SVG 体积优化**。

> 行为基准：`multiavatar-java`（Java 版，官方 JS 的逐字符等价移植）。
> 本仓库 20 组奇偶校验样本（ASCII/中文/emoji/强制角色主题）与 Java 版输出**逐字符一致**（见 `tests/parity_snapshot.rs`）。

---

## 特性 Features

| 能力 | 说明 |
| --- | --- |
| 确定性 | 同输入永远同头像（SHA-256 → 12 位种子 → 部件映射） |
| 多语言 | ASCII / CJK / emoji 输入均可 |
| 多输出 | SVG、PNG（透明）、JPEG（白底）、GIF（部件逐帧淡入动画）、favicon zip、favicon.ico、data URI |
| 体积优化 | 坐标精度裁剪（近无损，SVG 文本可减 20-40%），可开关 |
| LRU 缓存 | 同输入重复生成免重复计算（容量 128，键不含尺寸/格式） |
| 零运行时文件 | 部件库 gzip 嵌入二进制，单库交付 |
| 安全 | roxmltree 只读解析（无 XXE）；tiny-skia 纯 Rust 确定性光栅化 |

---

## 快速上手 Quick Start

```bash
cargo add multiavatar
```

```rust
use multiavatar::*;

// SVG 文本
let svg = multiavatar("demon1110");

// PNG 字节（256×256，透明背景）
let png = to_png("demon1110");

// JPEG / GIF / data URI
let jpg = to_jpg("Binx Bond");
let gif = to_gif("Binx Bond");          // 部件逐帧淡入动画

// 大尺寸输出（无锯齿的最直接方式）
let png_hi = to_png_sized("Binx Bond", 1024);

// SSAA 超采样抗锯齿：256 尺寸下边缘更平滑（4x 渲染后双线性缩小）
let png_smooth = to_png_smooth("Binx Bond");        // 256，默认 4x
let png_smooth_512 = to_png_smooth_sized("Binx Bond", 512);
let uri = to_data_uri("Binx Bond");     // data:image/png;base64,...

// favicon 包
let zip = to_favicon_zip("Binx Bond");  // 16/32/48/64/128/256 → avatar-{size}.png
let ico = to_favicon_ico("Binx Bond");  // 单文件 .ico

// 写文件
write_png("Binx Bond", "avatar.png")?;
write_favicon_ico("Binx Bond", "favicon.ico")?;

// 高级选项（Builder）
let options = avatar_options::AvatarOptions::builder()
    .sans_env(true)          // 去掉环境圆
    .part("05")              // 强制初始角色
    .theme("B")              // 强制颜色主题
    .size(512)               // 渲染尺寸
    .optimize_svg(false)     // 关闭体积优化（原始坐标）
    .build();
let avatar = avatar_with("Binx Bond", &options);
let png512 = avatar.png_sized(512);
```

演示与全量产物：

```bash
cargo run --example demo     # 输出到 demo/（10 组 × svg/png/jpg/gif + favicon + preview_grid.png）
```

---

## API 一览 API Overview

| 函数 | 等价 Java API | 说明 |
| --- | --- | --- |
| `multiavatar(input)` | `Multiavatar.multiavatar(input)` | SVG 文本 |
| `multiavatar_with(input, sans_env)` | `multiavatar(input, sansEnv)` | 去掉环境部件 |
| `multiavatar_full(input, sans_env, part, theme)` | `multiavatar(input, sansEnv, part, theme)` | 强制角色/主题 |
| `avatar(input)` / `avatar_with(input, options)` | `Multiavatar.avatar(...)` | 不可变产物（SVG + 惰性解析缓存） |
| `to_png / to_jpg / to_gif`(+`_sized`) | `toPng / toJpg / toGif` | 字节输出 |
| `to_image`(+`_sized`) | `toImage` | tiny-skia `Pixmap`（可继续加工） |
| `to_image_smooth`(+`_with`) / `to_png_smooth`(+`_sized`) / `to_jpg_smooth`(+`_sized`) | — | **SSAA 超采样抗锯齿**：内部按 `size×4` 渲染后双线性缩小，曲线边缘更平滑（小尺寸放大场景消除锯齿） |
| `to_data_uri`(+`_sized`) | `toDataUri` | `data:image/png;base64,...` |
| `to_favicon_zip / to_favicon_ico`(+`_sized`) | `toFaviconZip / toFaviconIco` | 多尺寸打包 |
| `render(input, format, options)` | `Multiavatar.render(...)` | 统一策略入口 |
| `write_png / write_favicon_zip / write_favicon_ico` | `writePng / ...` | 写文件 |
| `cache_size / clear_cache` | `cacheSize / clearCache` | LRU 缓存 |

---

## 架构 Architecture

```
┌────────────────────────────── 门面 Facade (src/lib.rs) ──────────────────────────────┐
│   Multiavatar：multiavatar* / avatar* / to_* / render / write_* / cache_*            │
│   全局 LRU 缓存（lru 128）＋ 单例 SvgComposer（OnceLock）                              │
└───────────────┬───────────────────────────────────────┬──────────────────────────────┘
                │ 产物 Avatar（不可变：SVG 文本缓存 +       │ 渲染（策略模式 Strategy）
                │ 惰性解析文档缓存 OnceLock）              ▼
                │                              ┌──────────────────────────────────┐
                │                              │ AvatarRenderer trait              │
                │                              │  ├ SvgRenderer（原文）             │
                │                              │  ├ PngRenderer（PNG 透明）         │
                │                              │  ├ JpgRenderer（JPEG 白底）         │
                │                              │  └ GifRenderer（逐帧淡入动画）      │
                │                              │  Renderers::create（简单工厂）      │
                │                              └──────────────────┬───────────────┘
                ▼                                                 ▼
┌──────────────────────────── SVG 领域层 src/svg ──────────────────────────────┐
│  SvgParser(roxmltree) → SvgDocument(viewBox+shapes) → SvgRasterizer(Pixmap) │
│  SvgShape（枚举：Path/Polygon/Rect/Line；模板方法：先填充后描边）             │
│  PathDataParser（全指令 + F.6 圆弧→三次贝塞尔）                                │
└───────────────────────────────┬─────────────────────────────────────────────┘
                                ▼
┌──────────────────────────── 核心装配层 src/core ─────────────────────────────┐
│  Sha256AvatarIdHasher（策略） → PartNumberMapper → PartKeyResolver          │
│  AvatarSpec（不可变对象，强制 part/theme 统一覆盖）                           │
│  SvgComposer（模板方法：固定部件顺序 env→head→clo→top→eyes→mouth）          │
│  SvgOptimizer（坐标精度裁剪，与官方 JS 逐字符一致）                            │
└───────────────────────────────┬─────────────────────────────────────────────┘
                                ▼
┌──────────────────────────── 数据层 src/data ─────────────────────────────────┐
│  DataTables（单例 OnceLock）：multiavatar-data.gz（gzip 内嵌，24939B）        │
│  解析 T|<partKey>|<name>|<colors> / P|<partKey>|<name>|<svg>                 │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 设计模式 Design Patterns

| 模式 | 位置 | 说明 |
| --- | --- | --- |
| 门面 Facade | `src/lib.rs` | 统一入口，屏蔽内部流水线 |
| 不可变对象 Immutable | `AvatarSpec` / `Avatar` / `AvatarOptions` | 线程安全共享，惰性缓存 |
| 模板方法 Template Method | `SvgComposer`（组装顺序）/ `SvgShape::paint` | 固定骨架 + 可换步骤 |
| 策略 Strategy | `AvatarIdHasher` / `AvatarRenderer` | 可插拔哈希与渲染算法 |
| 简单工厂 Simple Factory | `Renderers::create` | 按格式创建渲染器 |
| 单例 Singleton | `DataTables` / 门面 `SvgComposer` | OnceLock 惰性初始化 |
| Builder | `AvatarOptions::builder()` | 选项构建 |
| 组合 Composite | `SvgShape` 枚举（统一绘制协议） | 形状统一处理 |

---

## 与官方 JS / Java 的一致性 Fidelity

- **SVG 逐字符一致**：哈希、部件编号、拼装顺序、着色占位符替换、体积优化（含
  `Math.round` half-up 语义、负零 `-0`、前导小数点 token 不匹配、双空格属性格式）
  全部对齐；`tests/parity_snapshot.rs` 固化 4 组 Java 基准锚点，`cargo test` 即校验。
- **光栅化**：tiny-skia（f32）与 Java 2D / Chrome（f64）存在亚像素级差异（阈值 <0.1%），
  属渲染器精度差异，几何结构完全一致。
- **数据**：部件库 `multiavatar-data.gz` 与 Java 版同源（单一数据源，本仓库复制自 multiavatar-java）。

---

## 测试 Tests

```bash
cargo test          # 单测 32 + 集成 14 + doc 1 ≈ 47 项
cargo clippy        # 静态检查
```

覆盖：哈希/映射/舍入/优化器单测；SVG 奇偶锚点；PNG/JPEG/GIF/favicon/ICO/data URI 格式魔数与结构；
空输入；强制角色主题；LRU 缓存键语义（不含 size/format）；GIF 首帧可预览（完整头像）。

---

## 构建与产物 Build

```bash
cargo build --release   # LTO + codegen-units=1 + strip（体积优化）
```

依赖（允许常用 crate，均纯跨平台）：`flate2 roxmltree tiny-skia png jpeg-encoder gif zip base64 lru`。SHA-256 与 SVG 体积优化器均为手写实现（零正则/零哈希依赖）。

---

## 许可 License

MIT。原始 Multiavatar 版权归 Gie Katon；本移植与 multiavatar.js 同源（多文化头像数据来自原项目）。
