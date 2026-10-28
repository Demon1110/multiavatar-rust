#!/usr/bin/env bash
# ============================================================
# multiavatar-studio — macOS 打包脚本
# 用法：./scripts/build-macos.sh
#   在 macOS 上运行。构建 aarch64 + x86_64 两个架构，
#   lipo 合并为 universal 二进制，生成 .app 并压缩为 .dmg。
# 依赖：
#   rustup target add aarch64-apple-darwin x86_64-apple-darwin
#   （Xcode Command Line Tools：lipo / iconutil / hdiutil 自带）
# 产物：target/dist/multiavatar-studio-<version>-macos-universal.dmg
# ============================================================
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(grep -m1 '^version' egui/Cargo.toml | sed -n 's/.*"\(.*\)".*/\1/p')
DIST="target/dist"
mkdir -p "$DIST"

echo "==> 添加目标（已安装则跳过）"
rustup target add aarch64-apple-darwin x86_64-apple-darwin 2>/dev/null || true

echo "==> 构建 aarch64-apple-darwin（Apple Silicon）"
cargo build --release -p multiavatar-studio --target aarch64-apple-darwin

echo "==> 构建 x86_64-apple-darwin（Intel）"
cargo build --release -p multiavatar-studio --target x86_64-apple-darwin

echo "==> lipo 合并 universal 二进制"
mkdir -p target/universal
lipo -create \
  -output target/universal/multiavatar-studio \
  target/aarch64-apple-darwin/release/multiavatar-studio \
  target/x86_64-apple-darwin/release/multiavatar-studio
lipo -info target/universal/multiavatar-studio

APP="$DIST/multiavatar Studio.app"
echo "==> 生成 .app bundle：$APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/universal/multiavatar-studio "$APP/Contents/MacOS/multiavatar-studio"

cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>multiavatar Studio</string>
  <key>CFBundleDisplayName</key><string>multiavatar Studio</string>
  <key>CFBundleExecutable</key><string>multiavatar-studio</string>
  <key>CFBundleIdentifier</key><string>com.cary.multiavatar-studio</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.graphics-design</string>
</dict>
</plist>
EOF

# 图标：assets/app.ico -> icns（macOS 需要 .iconset 再 iconutil 转换）
echo "==> 生成 .icns 图标"
if [ -f assets/app.ico ] && command -v iconutil >/dev/null 2>&1 && command -v sips >/dev/null 2>&1; then
  ICONSET="target/app.iconset"
  rm -rf "$ICONSET"; mkdir -p "$ICONSET"
  # 先把 ico 转成 1024 png（sips 支持部分 ico；失败则跳过图标）
  if sips -s format png assets/app.ico --out "$ICONSET/icon_1024x1024.png" >/dev/null 2>&1; then
    for s in 16 32 64 128 256 512; do
      sips -z $s $s "$ICONSET/icon_1024x1024.png" --out "$ICONSET/icon_${s}x${s}.png" >/dev/null 2>&1 || true
      sips -z $((s*2)) $((s*2)) "$ICONSET/icon_1024x1024.png" --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null 2>&1 || true
    done
    iconutil -c icns -o "$APP/Contents/Resources/app.icns" "$ICONSET" && {
      # 更新 Info.plist 指定图标
      /usr/libexec/PlistBuddy -c "Add :CFBundleIconFile string app.icns" "$APP/Contents/Info.plist" 2>/dev/null || true
    }
    rm -rf "$ICONSET"
  fi
else
  echo "!! 跳过图标（需要 iconutil/sips，或在 macOS 上执行）"
fi

echo "==> 生成 .dmg"
hdiutil create \
  -volname "multiavatar Studio $VERSION" \
  -srcfolder "$APP" \
  -ov -format UDZO \
  "$DIST/multiavatar-studio-$VERSION-macos-universal.dmg"

echo "==> 完成：$DIST/multiavatar-studio-$VERSION-macos-universal.dmg"
