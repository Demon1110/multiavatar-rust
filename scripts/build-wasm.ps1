# 一键构建 wasm 浏览器包（根级 scripts/，输出 wasm/dist）
# 前置：rustup target add wasm32-unknown-unknown；wasm-bindgen-cli 与 Cargo.lock 锁定版本一致
# 用法：powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-wasm.ps1（或 cd wasm && npm run build）
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

Write-Host "[1/3] cargo build --target wasm32-unknown-unknown (release)"
Push-Location $root
try {
    cargo build --release --target wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
} finally {
    Pop-Location
}

Write-Host "[2/3] wasm-bindgen -> wasm/dist"
$wasm = Join-Path $root "target\wasm32-unknown-unknown\release\multiavatar.wasm"
$dist = Join-Path $root "wasm\dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
wasm-bindgen $wasm --target web --out-dir $dist
if ($LASTEXITCODE -ne 0) { throw "wasm-bindgen failed ($LASTEXITCODE)" }

Write-Host "[3/3] done"
Get-ChildItem $dist -File | ForEach-Object { "{0,-22} {1,8:N1} KB" -f $_.Name, ($_.Length / 1KB) }
