# multiavatar Studio 跨平台打包指南

工作台（egui 分支）基于 eframe 0.37（glow/OpenGL 后端），支持
**Windows / Linux / macOS / Android**。各平台打包脚本统一放在 `scripts/`。

| 平台 | 脚本 | 产物 |
| --- | --- | --- |
| Windows | `scripts/upx-pack.ps1`（先 `cargo build --release -p multiavatar-studio`） | `multiavatar-studio.exe`（UPX 加壳后 ~1.7MB） |
| Linux | `scripts/build-linux.sh`（可选 `--appimage`） | `target/dist/multiavatar-studio-<ver>-linux-x86_64.tar.gz` |
| macOS | `scripts/build-macos.sh` | `target/dist/multiavatar-studio-<ver>-macos-universal.dmg`（aarch64+x86_64 universal） |
| Android | `scripts/build-android.sh` + gradle 模板 | `android-app/app/build/outputs/apk/release/app-release.apk` |

> 库本体（`multiavatar`）无平台差异，`cargo build --release` 即可用于服务端/CLI；
> 本指南只针对桌面/移动工作台 `multiavatar-studio`。

---

## 通用前置

```bash
cargo build --release -p multiavatar-studio   # 各平台先做这一步验证本机构建
```

版本号取自 `egui/Cargo.toml` 的 `version`，打包脚本自动读取。

---

## Windows

```powershell
cargo build --release -p multiavatar-studio
.\scripts\upx-pack.ps1            # 自动探测 upx；--best --lzma 压到 ~1.7MB
```

- exe 已嵌入 ICO 图标（build.rs + winresource）
- **每次 cargo build 后原 exe 被覆盖，需重新执行 upx-pack.ps1**
- 首次运行可能被杀软实时扫描延迟约 6s

---

## Linux

```bash
./scripts/build-linux.sh                    # tar.gz
./scripts/build-linux.sh --appimage         # 额外生成 AppImage
```

- 默认用 native target 构建（无需交叉编译）
- `--appimage` 需要 linuxdeploy + linuxdeploy-plugin-gtk（脚本会给出下载地址）
- 产物：`target/dist/multiavatar-studio-<ver>-linux-x86_64.tar.gz`（含二进制 + sha256）

## macOS

```bash
./scripts/build-macos.sh
```

- 依赖：Xcode CLT（自带 lipo / iconutil / hdiutil）
- 先 `rustup target add aarch64-apple-darwin x86_64-apple-darwin`
- 双架构 lipo 合并 → universal 二进制 → `.app` bundle（Info.plist + icns 图标）→ `.dmg`
- 未签名：本机运行需 `右键 -> 打开` 绕过 Gatekeeper；分发需 Apple Developer 签名/公证

## Android

```bash
# 一次性安装
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk
# 设置 NDK 路径（以 r25 为例）
export ANDROID_NDK_HOME=$HOME/Android/Sdk/ndk/25.2.9519653

./scripts/build-android.sh
# 产物：android-app/app/src/main/jniLibs/{arm64-v8a,armeabi-v7a,x86_64}/libmultiavatar_studio.so
```

APK 打包需要最小 gradle 工程（脚本末尾打印了 MainActivity 核心代码）：

```text
android-app/
├── settings.gradle / build.gradle / gradle-wrapper.properties / gradlew
└── app/src/main/
    ├── AndroidManifest.xml          # package=com.cary.multiavatarstudio
    ├── java/com/cary/multiavatarstudio/MainActivity.kt
    └── res/values/strings.xml
```

```bash
cd android-app && ./gradlew assembleRelease
# APK：app/build/outputs/apk/release/app-release.apk
```

> Android 上 `rfd`（原生保存对话框）自动排除，导出走 `EXTERNAL_STORAGE`/Downloads。

---

## WASM（npm 发布，main 分支链路）

```powershell
.\scripts\build-wasm.ps1          # wasm-bindgen 封装到 wasm/ 目录
cd wasm && npm publish            # 或 npm stage publish（2FA 场景）
```

版本同步：`scripts/sync-version.mjs`（prepublishOnly 钩子，自动对齐 Cargo.toml 与 package.json）。
