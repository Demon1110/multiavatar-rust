#!/usr/bin/env bash
# ============================================================
# multiavatar-studio — Linux 打包脚本
# 用法：./scripts/build-linux.sh [--appimage]
#   默认：cargo build --release 并打包 tar.gz（二进制 + sha256）
#   --appimage：额外用 linuxdeploy 生成 .AppImage（可选）
# 依赖：cargo/rustc（native target 即可，无需交叉编译）
#   --appimage 需要：linuxdeploy + linuxdeploy-plugin-gtk
#   （https://github.com/linuxdeploy/linuxdeploy/releases）
# 产物：target/dist/multiavatar-studio-<version>-linux-x86_64.tar.gz
# ============================================================
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(grep -m1 '^version' egui/Cargo.toml | sed -n 's/.*"\(.*\)".*/\1/p')
DIST="target/dist"
mkdir -p "$DIST"

echo "==> 构建 release 二进制"
cargo build --release -p multiavatar-studio

BIN="target/release/multiavatar-studio"
if [ ! -x "$BIN" ]; then
  echo "!! 未找到二进制：$BIN" >&2
  exit 1
fi

PKG="multiavatar-studio-$VERSION-linux-x86_64"
echo "==> 打包 $PKG.tar.gz"
mkdir -p "$DIST/$PKG"
cp "$BIN" "$DIST/$PKG/multiavatar-studio"
# 可选：附带使用说明
if [ -f README.md ]; then
  cp README.md "$DIST/$PKG/"
fi
tar -czf "$DIST/$PKG.tar.gz" -C "$DIST" "$PKG"
sha256sum "$DIST/$PKG.tar.gz" > "$DIST/$PKG.tar.gz.sha256"
rm -rf "$DIST/$PKG"
echo "==> 完成：$DIST/$PKG.tar.gz (+.sha256)"

# ---------- 可选 AppImage ----------
if [ "${1:-}" = "--appimage" ]; then
  echo "==> 生成 AppImage（需要 linuxdeploy）"
  LINUXDEPLOY="${LINUXDEPLOY:-$HOME/.local/bin/linuxdeploy-x86_64.AppImage}"
  GTK_PLUGIN="${GTK_PLUGIN:-$HOME/.local/bin/linuxdeploy-plugin-gtk.sh}"
  if [ ! -x "$LINUXDEPLOY" ] || [ ! -x "$GTK_PLUGIN" ]; then
    echo "!! 缺少 linuxdeploy 或 gtk 插件，跳过 AppImage" >&2
    echo "   下载：https://github.com/linuxdeploy/linuxdeploy/releases 与" >&2
    echo "         https://github.com/linuxdeploy/linuxdeploy-plugin-gtk/releases" >&2
    exit 0
  fi

  APPDIR="target/appdir"
  rm -rf "$APPDIR"
  mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/256x256/apps"

  # 图标：优先把 assets/app.ico 转成 PNG（需要 ImageMagick/rsvg-convert），没有则跳过
  ICON_PNG="$APPDIR/usr/share/icons/hicolor/256x256/apps/multiavatar-studio.png"
  if command -v convert >/dev/null 2>&1; then
    convert assets/app.ico[0] "$ICON_PNG"
  elif command -v rsvg-convert >/dev/null 2>&1; then
    rsvg-convert -w 256 -h 256 assets/app.ico -o "$ICON_PNG"
  fi

  cat > "$APPDIR/usr/share/applications/multiavatar-studio.desktop" <<EOF
[Desktop Entry]
Name=multiavatar Studio
Comment=跨平台头像生成工作台
Exec=multiavatar-studio
Icon=multiavatar-studio
Type=Application
Categories=Graphics;
EOF

  cp "$BIN" "$APPDIR/usr/bin/multiavatar-studio"
  "$LINUXDEPLOY" --appdir "$APPDIR" \
    --desktop-file "$APPDIR/usr/share/applications/multiavatar-studio.desktop" \
    --output appimage
  mv "$APPDIR/multiavatar-studio-x86_64.AppImage" "$DIST/multiavatar-studio-$VERSION-x86_64.AppImage" 2>/dev/null || true
  echo "==> AppImage：$DIST/multiavatar-studio-$VERSION-x86_64.AppImage"
fi
