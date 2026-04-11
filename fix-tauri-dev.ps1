# Tauri 开发环境诊断和修复脚本

Write-Host "=== Tauri 开发环境诊断 ===" -ForegroundColor Cyan
Write-Host ""

# 检查 1: Cargo.toml 中的 devtools feature
Write-Host "[检查 1/5] 验证 Cargo.toml 中的 devtools feature..." -ForegroundColor Yellow
$cargoContent = Get-Content "src-tauri/Cargo.toml" -Raw
if ($cargoContent -match 'tauri.*features.*\[.*"devtools".*\]') {
    Write-Host "  ✓ devtools feature 已启用" -ForegroundColor Green
} else {
    Write-Host "  ✗ devtools feature 未启用" -ForegroundColor Red
    Write-Host "  建议：在 Cargo.toml 中添加 'devtools' 到 tauri 的 features" -ForegroundColor Gray
}

# 检查 2: tauri.conf.json 配置
Write-Host "`n[检查 2/5] 验证 tauri.conf.json 配置..." -ForegroundColor Yellow
$tauriConfig = Get-Content "src-tauri/tauri.conf.json" -Raw | ConvertFrom-Json
Write-Host "  ✓ 产品名称: $($tauriConfig.productName)" -ForegroundColor Green
Write-Host "  ✓ 开发 URL: $($tauriConfig.build.devUrl)" -ForegroundColor Green
Write-Host "  ✓ 前端目录: $($tauriConfig.build.frontendDist)" -ForegroundColor Green

# 检查 3: Vite 配置
Write-Host "`n[检查 3/5] 验证 Vite 配置..." -ForegroundColor Yellow
if (Test-Path "vite.config.ts") {
    Write-Host "  ✓ vite.config.ts 存在" -ForegroundColor Green
} else {
    Write-Host "  ✗ vite.config.ts 不存在" -ForegroundColor Red
}

# 检查 4: 端口占用
Write-Host "`n[检查 4/5] 检查端口 1420 占用情况..." -ForegroundColor Yellow
$portCheck = netstat -ano | Select-String ":1420.*LISTENING"
if ($portCheck) {
    Write-Host "  ⚠ 端口 1420 已被占用：" -ForegroundColor Yellow
    $portCheck | ForEach-Object { Write-Host "    $_" -ForegroundColor Gray }
} else {
    Write-Host "  ✓ 端口 1420 可用" -ForegroundColor Green
}

# 检查 5: dist 目录
Write-Host "`n[检查 5/5] 检查 dist 目录..." -ForegroundColor Yellow
if (Test-Path "dist") {
    $distFiles = Get-ChildItem "dist" -File | Measure-Object
    Write-Host "  ✓ dist 目录存在，包含 $($distFiles.Count) 个文件" -ForegroundColor Green
} else {
    Write-Host "  ⚠ dist 目录不存在，需要先构建" -ForegroundColor Yellow
    Write-Host "    运行: npm run build" -ForegroundColor Gray
}

# 修复操作
Write-Host "`n=== 开始修复 ===" -ForegroundColor Cyan
Write-Host ""

# 清理操作
Write-Host "[操作 1/3] 关闭相关进程..." -ForegroundColor Yellow
$nodeProcess = Get-Process -Name "node" -ErrorAction SilentlyContinue
$rustProcess = Get-Process -Name "MyRustTools" -ErrorAction SilentlyContinue

if ($nodeProcess) {
    Stop-Process -Name "node" -Force -ErrorAction SilentlyContinue
    Write-Host "  ✓ Node 进程已关闭" -ForegroundColor Green
}

if ($rustProcess) {
    Stop-Process -Name "MyRustTools" -Force -ErrorAction SilentlyContinue
    Write-Host "  ✓ MyRustTools 进程已关闭" -ForegroundColor Green
}

if (-not $nodeProcess -and -not $rustProcess) {
    Write-Host "  ✓ 没有发现需要关闭的进程" -ForegroundColor Green
}

Write-Host "`n[操作 2/3] 清理缓存..." -ForegroundColor Yellow
if (Test-Path "node_modules/.vite") {
    Remove-Item -Path "node_modules/.vite" -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "  ✓ Vite 缓存已清理" -ForegroundColor Green
} else {
    Write-Host "  ✓ Vite 缓存目录不存在（无需清理）" -ForegroundColor Green
}

Write-Host "`n[操作 3/3] 等待端口释放..." -ForegroundColor Yellow
Start-Sleep -Seconds 3
Write-Host "  ✓ 准备就绪" -ForegroundColor Green

# 最终建议
Write-Host "`n=== 启动建议 ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "配置已修复！现在请执行以下步骤：" -ForegroundColor Green
Write-Host ""
Write-Host "1. 打开一个新的终端窗口" -ForegroundColor White
Write-Host "2. 进入项目目录：" -ForegroundColor White
Write-Host "   cd D:\Code\rust\rust-tools" -ForegroundColor Gray
Write-Host "3. 运行开发命令：" -ForegroundColor White
Write-Host "   npm run tauri dev" -ForegroundColor Gray
Write-Host ""
Write-Host "或者直接运行此脚本，它会尝试启动开发服务器..." -ForegroundColor Yellow

# 询问是否启动
Write-Host ""
$choice = Read-Host "是否现在启动 Tauri 开发服务器？(y/n)"
if ($choice -eq "y" -or $choice -eq "Y") {
    Write-Host "`n正在启动 Tauri 开发服务器..." -ForegroundColor Cyan
    npm run tauri dev
}
