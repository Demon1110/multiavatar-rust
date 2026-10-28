# UPX 加壳压缩 multiavatar-studio.exe（仅 Windows 桌面包）
#
# 用法：
#   powershell -ExecutionPolicy Bypass -File scripts/upx-pack.ps1          # 加壳
#   powershell -ExecutionPolicy Bypass -File scripts/upx-pack.ps1 -Test     # 加壳 + 启动冒烟测试
#
# 依赖：
#   - 已构建 release：cargo build --release -p multiavatar-studio
#   - UPX：winget install UPX.UPX（脚本会自动探测 PATH / winget 安装目录）
#
# 效果：约 5.3 MB -> 1.7 MB（--best --lzma，压缩到 ~32%）

param(
    [switch]$Test
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot   # 仓库根目录
$exe = Join-Path $root "target\release\multiavatar-studio.exe"

if (-not (Test-Path -LiteralPath $exe)) {
    Write-Host "未找到 $exe，先执行: cargo build --release -p multiavatar-studio" -ForegroundColor Red
    exit 1
}

# ---- 探测 upx ----
$upx = $null
$cmd = Get-Command upx -ErrorAction SilentlyContinue
if ($cmd) { $upx = $cmd.Source }
if (-not $upx) {
    # winget 安装目录（UPX.UPX 包）
    $pkgRoot = Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Packages"
    if (Test-Path -LiteralPath $pkgRoot) {
        $found = Get-ChildItem -LiteralPath $pkgRoot -Recurse -Filter upx.exe -ErrorAction SilentlyContinue |
            Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if ($found) { $upx = $found.FullName }
    }
}
if (-not $upx) {
    Write-Host "未找到 upx.exe。请先安装: winget install UPX.UPX" -ForegroundColor Red
    exit 1
}
Write-Host "使用 UPX: $upx" -ForegroundColor Cyan

# ---- 加壳 ----
$before = (Get-Item -LiteralPath $exe).Length
Write-Host "加壳前: $([math]::Round($before / 1KB)) KB"
& $upx --best --lzma $exe
if ($LASTEXITCODE -ne 0) { Write-Host "UPX 失败，退出码 $LASTEXITCODE" -ForegroundColor Red; exit $LASTEXITCODE }
$after = (Get-Item -LiteralPath $exe).Length
Write-Host "加壳后: $([math]::Round($after / 1KB)) KB  ($([math]::Round($after / $before * 100, 1))%)" -ForegroundColor Green

# ---- 可选冒烟测试：启动 -> 检查窗口标题 -> 杀掉 ----
if ($Test) {
    Write-Host "冒烟测试：启动 $exe ..."
    $p = Start-Process -FilePath $exe -PassThru
    Start-Sleep -Seconds 6
    $alive = Get-Process -Name "multiavatar-studio" -ErrorAction SilentlyContinue
    if ($alive) {
        Write-Host "OK：进程存活，窗口标题 '$($alive[0].MainWindowTitle)'" -ForegroundColor Green
        Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    } else {
        Write-Host "FAIL：进程未能存活（可能是首次运行被杀软实时扫描拦截，稍后再试）" -ForegroundColor Red
        exit 1
    }
}
Write-Host "完成。产物: $exe"
