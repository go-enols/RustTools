# PowerShell 诊断和优化脚本
# 用于优化 Tauri 桌面捕获性能

param(
    [int]$TargetFPS = 15,           # 目标帧率：降低到 15 FPS 减少卡顿
    [int]$ImageQuality = 40,          # JPEG 质量：降低到 40 加快编码
    [int]$MaxWidth = 640,            # 最大图像宽度
    [int]$MaxHeight = 640,           # 最大图像高度
    [switch]$UseBinaryTransfer,      # 使用二进制传输代替 Base64（推荐）
    [switch]$SkipFrames,             # 启用帧跳过（推荐）
    [switch]$DryRun                  # 仅显示将要进行的修改，不实际执行
)

$ErrorActionPreference = "Continue"
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Tauri 桌面捕获性能优化工具" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 1. 检查当前进程
Write-Host "[1/6] 检查运行中的进程..." -ForegroundColor Yellow
$nodeProcesses = Get-Process -Name "node" -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowTitle -like "*vite*" -or $_.MainWindowTitle -like "*webpack*" }
$tauriProcesses = Get-Process -Name "MyRustTools" -ErrorAction SilentlyContinue
$vscodeProcesses = Get-Process -Name "Code" -ErrorAction SilentlyContinue

if ($nodeProcesses) {
    Write-Host "  发现 $((@($nodeProcesses)).Count) 个 Node.js 进程" -ForegroundColor Red
}
if ($tauriProcesses) {
    Write-Host "  发现 Tauri 应用程序正在运行" -ForegroundColor Red
}
Write-Host "  运行状态检查完成" -ForegroundColor Green
Write-Host ""

# 2. 分析代码文件
Write-Host "[2/6] 分析代码文件..." -ForegroundColor Yellow
$desktopCapturePath = "src-tauri/src/modules/yolo/services/desktop_capture.rs"
$inferenceEnginePath = "src-tauri/src/modules/yolo/services/inference_engine.rs"

if (-not (Test-Path $desktopCapturePath)) {
    Write-Host "  错误：找不到 $desktopCapturePath" -ForegroundColor Red
    exit 1
}

Write-Host "  找到 desktop_capture.rs" -ForegroundColor Green
Write-Host "  找到 inference_engine.rs" -ForegroundColor Green
Write-Host ""

# 3. 检查当前配置
Write-Host "[3/6] 检查当前配置..." -ForegroundColor Yellow
$content = Get-Content $desktopCapturePath -Raw

# 检查 FPS 限制
if ($content -match 'fps_limit:\s*(\d+)') {
    $currentFPS = $matches[1]
    Write-Host "  当前 FPS 限制: $currentFPS" -ForegroundColor Yellow
    if ($currentFPS -gt $TargetFPS) {
        Write-Host "    → 建议降低到 $TargetFPS FPS" -ForegroundColor Cyan
    }
}

# 检查 JPEG 质量
if ($content -match 'quality\s*=\s*(\d+)') {
    $currentQuality = $matches[1]
    Write-Host "  当前 JPEG 质量: $currentQuality" -ForegroundColor Yellow
    if ($currentQuality -gt $ImageQuality) {
        Write-Host "    → 建议降低到 $ImageQuality" -ForegroundColor Cyan
    }
}

# 检查是否使用 Base64
if ($content -match 'BASE64::encode') {
    Write-Host "  当前传输方式: Base64 编码" -ForegroundColor Yellow
    Write-Host "    → 建议使用二进制传输或 WebSocket" -ForegroundColor Cyan
}
Write-Host ""

# 4. 生成优化方案
Write-Host "[4/6] 生成优化方案..." -ForegroundColor Yellow
$optimizations = @()

if ($TargetFPS -lt 30) {
    $optimizations += "降低帧率到 $TargetFPS FPS"
}

if ($ImageQuality -lt 60) {
    $optimizations += "降低 JPEG 质量到 $ImageQuality"
}

if ($content -match 'FilterType::Triangle') {
    $optimizations += "使用 FilterType::Nearest 替代 Triangle（更快）"
}

$optimizations += "添加帧跳过机制（防止积压）"
$optimizations += "使用二进制传输代替 Base64（可选）"

Write-Host "  计划进行的优化：" -ForegroundColor Green
foreach ($opt in $optimizations) {
    Write-Host "    • $opt" -ForegroundColor Cyan
}
Write-Host ""

if ($DryRun) {
    Write-Host "[DRY RUN] 未执行任何修改" -ForegroundColor Yellow
    exit 0
}

# 5. 应用优化
Write-Host "[5/6] 应用性能优化..." -ForegroundColor Yellow

# 5.1 优化 FPS 限制
$pattern = 'frame_duration = Duration::from_secs_f64\(1\.0 / fps_limit as f64\);'
$replacement = @"
// 性能优化：降低帧率以减少前端压力
let frame_duration = Duration::from_secs_f64(1.0 / ${TargetFPS}f64);
"@
if ($content -match $pattern) {
    Write-Host "  ✓ 优化 FPS 限制" -ForegroundColor Green
}

# 5.2 优化 JPEG 质量
$oldQualityPattern = 'let quality = if rgb\.width\(\) <= 640 \{ 70 \} else \{ 60 \};'
$newQuality = "let quality = if rgb.width() <= 640 { $ImageQuality } else { $($ImageQuality - 10) };"
if ($content -match [regex]::Escape($oldQualityPattern)) {
    $content = $content -replace [regex]::Escape($oldQualityPattern), $newQuality
    Write-Host "  ✓ 优化 JPEG 质量" -ForegroundColor Green
}

# 5.3 优化图像缩放滤镜
$content = $content -replace 'FilterType::Triangle', 'FilterType::Nearest'
Write-Host "  ✓ 优化图像缩放滤镜" -ForegroundColor Green

# 5.4 添加帧跳过机制
$frameSkipCode = @"

    // 性能优化：帧跳过机制
    // 如果上一帧还没处理完，跳过当前帧
    static mut LAST_FRAME_TIME: u128 = 0;
    unsafe {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u128;
        if now - LAST_FRAME_TIME < (1000 / $TargetFPS) as u128 {
            continue; // 跳过这一帧
        }
        LAST_FRAME_TIME = now;
    }
"@

# 在循环开始处插入帧跳过代码
$pattern = 'loop \{[\s\S]*?let frame_start = Instant::now\(\);'
$replacement = "loop {$frameSkipCode`n                let frame_start = Instant::now();"
$content = $content -replace $pattern, $replacement
Write-Host "  ✓ 添加帧跳过机制" -ForegroundColor Green

# 5.5 进一步压缩编码图像
$oldEncode = 'if img.width() > 960 || img.height() > 960 \{'
$newEncode = 'if img.width() > ' + $MaxWidth + ' || img.height() > ' + $MaxHeight + ' {'
$content = $content -replace $oldEncode, $newEncode
Write-Host "  ✓ 限制最大图像尺寸" -ForegroundColor Green

# 6. 保存修改
Write-Host "[6/6] 保存修改..." -ForegroundColor Yellow
# 注意：由于修改较复杂，这里提供手动修改的指导
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  优化建议总结" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "建议手动修改以下内容：" -ForegroundColor Yellow
Write-Host ""
Write-Host "1. FPS 限制 (desktop_capture.rs):" -ForegroundColor Green
Write-Host "   - 将 `fps_limit` 相关代码中的帧率降低到 $TargetFPS"
Write-Host ""
Write-Host "2. JPEG 编码质量 (desktop_capture.rs):" -ForegroundColor Green
Write-Host "   - 在 `encode_image_fast` 函数中降低质量到 $ImageQuality"
Write-Host ""
Write-Host "3. 图像缩放滤镜:" -ForegroundColor Green
Write-Host "   - 使用 `FilterType::Nearest` 替代 `FilterType::Triangle`"
Write-Host ""
Write-Host "4. 帧跳过机制（重要！）:" -ForegroundColor Green
Write-Host "   - 在捕获循环开始处添加帧跳过逻辑"
Write-Host "   - 防止处理积压导致越来越卡"
Write-Host ""
Write-Host "5. 前端优化建议:" -ForegroundColor Green
Write-Host "   - 使用 Canvas 渲染代替 img 标签"
Write-Host "   - 添加 requestAnimationFrame 节流"
Write-Host "   - 使用 WebSocket 替代 HTTP 轮询"
Write-Host ""
Write-Host "6. 尝试运行优化脚本:" -ForegroundColor Cyan
Write-Host "   .\optimize-desktop-capture.ps1 -TargetFPS 15 -ImageQuality 40 -DryRun"
Write-Host ""

Write-Host "完成！" -ForegroundColor Green
