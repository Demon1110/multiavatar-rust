#!/usr/bin/env bash
# ============================================================
# multiavatar-studio — Android 打包脚本
# 用法：./scripts/build-android.sh
#   编译 Android .so（arm64-v8a / armeabi-v7a / x86_64）到
#   android-app/app/src/main/jniLibs/<abi>/libmultiavatar_studio.so
#   之后用 gradle 模板打 APK（见 scripts/BUILD.md「Android」一节）。
# 依赖（一次性安装）：
#   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
#   cargo install cargo-ndk
#   Android NDK（如 r25+），并设置环境变量 ANDROID_NDK_HOME
# 产物：android-app/app/src/main/jniLibs/**/*.so
# ============================================================
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "!! 缺少 cargo-ndk，先执行：cargo install cargo-ndk" >&2
  exit 1
fi
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  echo "!! 未设置 ANDROID_NDK_HOME（指向 Android NDK 目录）" >&2
  exit 1
fi

# 库名：multiavatar-studio -> libmultiavatar_studio.so（crate 名转下划线）
JNI="android-app/app/src/main/jniLibs"
mkdir -p "$JNI"

echo "==> 添加 Android targets（已装则跳过）"
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android 2>/dev/null || true

echo "==> cargo ndk 编译三个 ABI"
cargo ndk \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86_64 \
  -o "$JNI" \
  build --release -p multiavatar-studio

echo "==> jniLibs 产物："
find "$JNI" -name '*.so' -exec ls -lh {} \;

cat <<'MSG'
==> 下一步：打 APK
  1) 参考 egui 官方 Android 模板（https://github.com/emilk/egui/tree/master/examples/eframe_template/android）
     在 android-app/ 下建立最小 gradle 工程：
       android-app/
         settings.gradle / build.gradle / gradlew
         app/src/main/
           AndroidManifest.xml
           java/com/cary/multiavatarstudio/MainActivity.kt
           res/values/strings.xml
         （jniLibs 已由本脚本生成）
     MainActivity 核心（3 行）：
       class MainActivity : android.app.Activity() {
         override fun onCreate(savedInstanceState: android.os.Bundle?) {
           super.onCreate(savedInstanceState)
           android.widget.FrameLayout(this).addView(org.egui.NativeView(this))
         }
       }
  2) cd android-app && ./gradlew assembleRelease
  3) APK 输出：android-app/app/build/outputs/apk/release/app-release.apk
MSG
