# 同步版本号：Cargo.toml（唯一来源）→ wasm/package.json
# 幂等；只替换顶层 "version" 字段，其余内容原样保留。
# 用法：powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sync-version.ps1
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

$cargo = Join-Path $root "Cargo.toml"
$pkg   = Join-Path $root "wasm\package.json"

$m = Select-String -LiteralPath $cargo -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
if (-not $m -or -not $m.Matches[0].Groups[1].Value) { throw "Cargo.toml 中未找到 version" }
$ver = $m.Matches[0].Groups[1].Value

$raw = Get-Content -LiteralPath $pkg -Raw
if ($raw -notmatch '"version"\s*:') { throw "wasm/package.json 中未找到 version 字段" }
$raw = $raw -replace '("version"\s*:\s*)"[^"]*"', "`$1`"$ver`""
[IO.File]::WriteAllText($pkg, $raw, (New-Object Text.UTF8Encoding $false))

Write-Host "synced: wasm/package.json version -> $ver"
