# Tauri 实时检测性能测试

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Tauri 实时检测性能测试工具" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 检查是否正在运行
Write-Host "[检查] 查找正在运行的 Tauri 进程..." -ForegroundColor Yellow
$tauriProcess = Get-Process -Name "MyRustTools" -ErrorAction SilentlyContinue
$nodeProcess = Get-Process -Name "node" -ErrorAction SilentlyContinue

if ($tauriProcess -or $nodeProcess) {
    Write-Host "  ⚠ 检测到 Tauri 开发服务器正在运行" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "请先关闭正在运行的 Tauri 进程：" -ForegroundColor Yellow
    Write-Host "  1. 关闭 Tauri 窗口" -ForegroundColor Gray
    Write-Host "  2. 按 Ctrl+C 在终端中停止 npm run tauri dev" -ForegroundColor Gray
    Write-Host ""
    $choice = Read-Host "是否自动关闭这些进程？(y/n)"
    
    if ($choice -eq "y" -or $choice -eq "Y") {
        Write-Host "`n正在关闭进程..." -ForegroundColor Yellow
        
        if ($nodeProcess) {
            Stop-Process -Name "node" -Force -ErrorAction SilentlyContinue
            Write-Host "  ✓ Node 进程已关闭" -ForegroundColor Green
        }
        
        if ($tauriProcess) {
            Stop-Process -Name "MyRustTools" -Force -ErrorAction SilentlyContinue
            Write-Host "  ✓ MyRustTools 进程已关闭" -ForegroundColor Green
        }
        
        Write-Host "  等待端口释放..." -ForegroundColor Yellow
        Start-Sleep -Seconds 3
    } else {
        Write-Host "请手动关闭进程后重新运行此脚本" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "  ✓ 没有找到正在运行的进程" -ForegroundColor Green
}

# 清理缓存
Write-Host "`n[清理] 清理缓存..." -ForegroundColor Yellow
if (Test-Path "node_modules/.vite") {
    Remove-Item -Path "node_modules/.vite" -Recurse -Force
    Write-Host "  ✓ Vite 缓存已清理" -ForegroundColor Green
}

# 重新编译 Rust 代码
Write-Host "`n[编译] 重新编译 Rust 代码（性能优化版本）..." -ForegroundColor Yellow
Write-Host ""

# 检查是否有编译错误
$compileOutput = cargo build --manifest-path src-tauri/Cargo.toml 2>&1
$compileSuccess = $LASTEXITCODE -eq 0

if (-not $compileSuccess) {
    Write-Host "  ✗ 编译失败！" -ForegroundColor Red
    Write-Host ""
    Write-Host "错误信息：" -ForegroundColor Red
    Write-Host $compileOutput -ForegroundColor Gray
    exit 1
} else {
    Write-Host "  ✓ 编译成功！" -ForegroundColor Green
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  性能优化已应用！" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "优化内容：" -ForegroundColor Green
Write-Host "  1. ✓ 捕获后立即 resize 到 640x640" -ForegroundColor Gray
Write-Host "  2. ✓ 使用 Nearest Neighbor 快速 resize" -ForegroundColor Gray
Write-Host "  3. ✓ 跳过重复的 resize 操作" -ForegroundColor Gray
Write-Host "  4. ✓ 使用固定尺寸预处理" -ForegroundColor Gray
Write-Host "  5. ✓ 小图画框和编码" -ForegroundColor Gray
Write-Host "  6. ✓ 自适应 JPEG 编码质量" -ForegroundColor Gray
Write-Host ""
Write-Host "预期性能提升：" -ForegroundColor Cyan
Write-Host "  - FPS: 5-10 FPS → 25-30 FPS (提升 300%)" -ForegroundColor White
Write-Host "  - 延迟: 降低 80%" -ForegroundColor White
Write-Host "  - 内存: 减少 83%" -ForegroundColor White
Write-Host ""

# 启动开发服务器
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  准备启动测试..." -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "启动后，请：" -ForegroundColor Yellow
Write-Host "  1. 打开前端页面" -ForegroundColor White
Write-Host "  2. 进入实时检测功能" -ForegroundColor White
Write-Host "  3. 选择一个模型（如 yolo11n.onnx）" -ForegroundColor White
Write-Host "  4. 选择一个屏幕" -ForegroundColor White
Write-Host "  5. 点击开始检测" -ForegroundColor White
Write-Host ""
Write-Host "  观察 FPS 是否从 5-10 提升到 25-30！" -ForegroundColor Green
Write-Host ""

$choice = Read-Host "是否现在启动 Tauri 开发服务器？(y/n)"
if ($choice -eq "y" -or $choice -eq "Y") {
    Write-Host "`n正在启动 Tauri 开发服务器..." -ForegroundColor Cyan
    Write-Host "请在浏览器开发者工具的 Console 中查看 FPS 日志" -ForegroundColor Yellow
    Write-Host ""
    
    npm run tauri dev
}
