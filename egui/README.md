# multiavatar Studio — 跨平台头像生成工作台

基于 **egui / eframe** 的桌面与移动端应用，把 [multiavatar](../) 库的全部能力包装成面向大众的图形化工具。

## 场景定位（应用场景）

| 用户 | 场景 |
| --- | --- |
| 个人用户 | 输入名字 / 昵称 / 任意字符串 → 立刻得到独一无二的多元文化风格头像，用于社交账号、游戏、论坛 |
| 开发者 | 一键导出 **SVG / PNG / JPEG / GIF / ICO / favicon.zip**，做网页图标、UI 占位图、文档插图 |
| 团队 / 运营 | 批量模式：每行一个名字 → 网格预览 → 目录批量导出统一风格头像 |
| 前端 / 设计师 | 复制 **data-URI** 直接嵌入代码或设计稿；历史记录一键回选 |

## 功能

- **实时预览**：输入即生成，PNG / JPEG / SVG（渲染预览）/ GIF 动画（多帧轮播）
- **尺寸自由**：64 ~ 1024 px 对数滑块
- **格式导出**：SVG · PNG · JPEG · GIF · ICO · favicon.zip（原生保存对话框）
- **批量生成**：多行名字 → 网格预览 → 选择目录批量导出 PNG
- **复制 data-URI**：一键复制可嵌入的 Base64 资源
- **随机生成**：一键随机字符串，产出惊喜头像
- **历史记录**：最近 30 条，点击回选
- **绚丽主题**：🌙 暗夜霓虹（默认）/ ☀️ 明亮 / ⚡ 赛博霓虹，渐变背景 + 发光控件 + 圆角卡片

## UI 原型设计

```
┌─────────────────────────────────────────────────────────────────────────┐
│  top  multiavatar Studio ✨ 跨平台头像生成工作台    [☀️][🌙][⚡]  🎲 随机 │
├───────────────┬─────────────────────────────────────────────────────────┤
│ left  ✏️ 输入  │  center  👤 Neo  · 输入字符 3 个                          │
│   大输入框     │     ┌──────────────────────────────────────┐            │
│  [⚡生成][🎲随机]│     │                                      │            │
│  🕘 历史       │     │        （头像大预览，圆角发光卡片）        │            │
│  📐 尺寸滑块    │     │                                      │            │
│  🖼️ 预览格式    │     └──────────────────────────────────────┘            │
│  💾 导出       │  256×256 · 12.3 KB · SVG 6 782 字节                       │
│  [SVG][PNG]…  │  ✔ 状态栏                                               │
│  📋 复制URI    │                                                         │
├───────────────┴─────────────────────────────────────────────────────────┤
│  bottom 批量生成（每行一个名字）  [🖼️ 生成网格] [📁 批量导出(PNG)]          │
│   [头像] [头像] [头像] [头像] [头像] ← 横向滚动网格                          │
└─────────────────────────────────────────────────────────────────────────┘
```

布局策略：`Panel::top` 顶栏 → `Panel::left` 控制面板 → `CentralPanel` 预览 → `Panel::bottom` 批量区（egui 0.37 面板统一为 `egui::Panel`）。左侧纵向滚动，中央自适应大图，底部批量区可拖拽调高。

## 快速开始

```bash
# 桌面运行（Windows / Linux / macOS）
cargo run -p multiavatar-studio --release

# 打包发布版
cargo build -p multiavatar-studio --release
# 产物：target/release/multiavatar-studio(.exe)
```

中文字体自动探测系统字体（Windows 微软雅黑 / macOS 苹方 / Linux Noto CJK / Android 系统字体），无需打包字体，保持二进制体积最小（当前 release 约 5.2 MB）。

## 跨平台构建

### Windows / Linux / macOS

eframe 原生支持三平台，一条 `cargo build --release` 即可。Linux 需系统依赖：

```bash
sudo apt install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev libgtk-3-dev
```

### Android（APK）

eframe 通过 `android-native-activity` 特性支持 Android（已在本 crate 的 `Cargo.toml` 配置）：

```bash
# 1. 安装 Android Rust 目标与工具
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk

# 2. 设置 NDK（已安装 Android SDK/NDK 时）
#    把 NDK 路径写入 local.properties 或环境变量，例如：
#    export ANDROID_NDK_HOME=$HOME/Android/Sdk/ndk/<version>

# 3. 交叉编译动态库
cargo ndk -t arm64-v8a -o egui/jniLibs build --release -p multiavatar-studio

# 4. 用 Android Studio 新建 Native C++ 空工程，把 egui/jniLibs 与
#    AndroidManifest（android:hasCode="false" + 指向 android_main）接入；
#    或使用 cargo-apk（cargo install cargo-apk && cargo apk build -p multiavatar-studio）
```

Android 端导出走外部存储（无文件对话框），其余功能与桌面一致。

## 技术栈

- **egui 0.37 / eframe 0.37**：即时模式 GUI，跨平台渲染（glow / OpenGL 后端，绕开 wgpu 依赖树，体积更小）
- **multiavatar**：核心生成库（path 依赖，SVG/PNG/JPEG/GIF/ICO/favicon/data-URI）
- **image 0.25**：预览解码（PNG/JPEG/GIF 帧动画）
- **rfd 0.15**：桌面原生保存对话框（Android 下按 cfg 剔除）

## 目录结构

```
egui/
├── Cargo.toml          # multiavatar-studio crate（publish=false）
├── README.md
└── src/
    ├── main.rs         # 入口：桌面 main + Android android_main
    ├── app.rs          # 应用状态 + 主 UI 编排（顶栏/左栏/中央/批量）
    ├── theme.rs        # 三套绚丽主题 + 渐变背景 + 发光控件
    ├── preview.rs      # 位图解码 → egui 纹理；GIF 帧动画轮播
    └── export.rs       # 导出（rfd 对话框 / Android 内部存储）
```
