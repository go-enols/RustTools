# Tauri 开发环境重启脚本

Write-Host "=== Tauri 开发环境重启脚本 ===" -ForegroundColor Cyan

# 1. 查找并关闭所有相关进程
Write-Host "`n[1/4] 关闭相关进程..." -ForegroundColor Yellow

$nodeProcesses = Get-Process -Name "node" -ErrorAction SilentlyContinue
$rustProcesses = Get-Process -Name "MyRustTools" -ErrorAction SilentlyContinue

if ($nodeProcesses) {
    Write-Host "  关闭 Node 进程..." -ForegroundColor Gray
    Stop-Process -Name "node" -Force -ErrorAction SilentlyContinue
    Write-Host "  ✓ Node 进程已关闭" -ForegroundColor Green
}

if ($rustProcesses) {
    Write-Host "  关闭 MyRustTools 进程..." -ForegroundColor Gray
    Stop-Process -Name "MyRustTools" -Force -ErrorAction SilentlyContinue
    Write-Host "  ✓ MyRustTools 进程已关闭" -ForegroundColor Green
}

# 2. 等待端口释放
Write-Host "`n[2/4] 等待端口释放..." -ForegroundColor Yellow
Start-Sleep -Seconds 2

# 3. 清理 Vite 缓存
Write-Host "`n[3/4] 清理缓存..." -ForegroundColor Yellow
if (Test-Path "node_modules/.vite") {
    Remove-Item -Path "node_modules/.vite" -Recurse -Force
    Write-Host "  ✓ Vite 缓存已清理" -ForegroundColor Green
}

# 4. 启动开发服务器
Write-Host "`n[4/4] 启动 Tauri 开发服务器..." -ForegroundColor Yellow
Write-Host "`n请在新的终端窗口中运行:" -ForegroundColor Cyan
Write-Host "  npm run tauri dev" -ForegroundColor Green
Write-Host "`n或者直接启动（会覆盖此脚本）..." -ForegroundColor Gray
Write-Host ""

# 启动 Tauri
npm run tauri dev
