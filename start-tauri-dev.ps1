# Tauri 前端连接修复脚本

Write-Host "=== Tauri 前端连接修复 ===" -ForegroundColor Cyan
Write-Host ""

# 1. 关闭所有相关进程
Write-Host "[步骤 1/5] 关闭所有相关进程..." -ForegroundColor Yellow

# 关闭 node 进程
$nodeProcesses = Get-Process -Name "node" -ErrorAction SilentlyContinue
if ($nodeProcesses) {
    Write-Host "  正在关闭 $($nodeProcesses.Count) 个 Node 进程..." -ForegroundColor Gray
    Stop-Process -Name "node" -Force -ErrorAction SilentlyContinue
    Write-Host "  ✓ Node 进程已关闭" -ForegroundColor Green
}

# 关闭 MyRustTools 进程
$rustProcesses = Get-Process -Name "MyRustTools" -ErrorAction SilentlyContinue
if ($rustProcesses) {
    Write-Host "  正在关闭 $($rustProcesses.Count) 个 MyRustTools 进程..." -ForegroundColor Gray
    Stop-Process -Name "MyRustTools" -Force -ErrorAction SilentlyContinue
    Write-Host "  ✓ MyRustTools 进程已关闭" -ForegroundColor Green
}

if (-not $nodeProcesses -and -not $rustProcesses) {
    Write-Host "  ✓ 没有需要关闭的进程" -ForegroundColor Green
}

# 2. 清理缓存
Write-Host "`n[步骤 2/5] 清理缓存..." -ForegroundColor Yellow

# 清理 Vite 缓存
if (Test-Path "node_modules/.vite") {
    Remove-Item -Path "node_modules/.vite" -Recurse -Force
    Write-Host "  ✓ Vite 缓存已清理" -ForegroundColor Green
} else {
    Write-Host "  ✓ Vite 缓存目录不存在" -ForegroundColor Green
}

# 清理 dist 目录（可选，确保干净的构建）
if (Test-Path "dist") {
    Remove-Item -Path "dist" -Recurse -Force
    Write-Host "  ✓ dist 目录已清理" -ForegroundColor Green
}

# 3. 等待端口释放
Write-Host "`n[步骤 3/5] 等待端口释放..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

# 检查端口是否已释放
$portInUse = netstat -ano | Select-String ":1420.*LISTENING"
if ($portInUse) {
    Write-Host "  ⚠ 端口 1420 仍被占用，等待更长时间..." -ForegroundColor Yellow
    Start-Sleep -Seconds 5
    
    # 再次尝试关闭
    $remainingNodes = Get-Process -Name "node" -ErrorAction SilentlyContinue
    if ($remainingNodes) {
        Stop-Process -Name "node" -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "  ✓ 端口准备就绪" -ForegroundColor Green

# 4. 验证文件
Write-Host "`n[步骤 4/5] 验证关键文件..." -ForegroundColor Yellow

$files = @{
    "index.html" = "根目录 HTML 入口"
    "vite.config.ts" = "Vite 配置文件"
    "src/main.tsx" = "React 入口文件"
    "src/App.tsx" = "主应用组件"
}

$allFilesExist = $true
foreach ($file in $files.Keys) {
    if (Test-Path $file) {
        Write-Host "  ✓ $($files[$file]) - 存在" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $($files[$file]) - 缺失!" -ForegroundColor Red
        $allFilesExist = $false
    }
}

if (-not $allFilesExist) {
    Write-Host "`n⚠ 某些关键文件缺失，请先修复！" -ForegroundColor Red
    exit 1
}

# 5. 启动开发服务器
Write-Host "`n[步骤 5/5] 启动 Tauri 开发服务器..." -ForegroundColor Yellow
Write-Host ""
Write-Host "正在启动服务器，请稍候..." -ForegroundColor Cyan
Write-Host ""

# 使用 Tauri CLI 启动开发服务器
npm run tauri dev
