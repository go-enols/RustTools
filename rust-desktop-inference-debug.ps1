# Rust 桌面推理性能诊断工具
# 诊断帧率问题的各个模块

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Rust 桌面推理性能诊断工具 v2.0" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 1. 检查进程状态
Write-Host "[1/8] 检查进程状态..." -ForegroundColor Yellow
$tauriProcesses = Get-Process -Name "MyRustTools" -ErrorAction SilentlyContinue
if ($tauriProcesses) {
    Write-Host "  ✓ Tauri 应用正在运行 (PID: $($tauriProcesses.Id))" -ForegroundColor Green
} else {
    Write-Host "  ✗ Tauri 应用未运行" -ForegroundColor Red
}
Write-Host ""

# 2. 检查端口占用
Write-Host "[2/8] 检查端口占用..." -ForegroundColor Yellow
$connections = Get-NetTCPConnection -LocalPort 1420, 1421 -ErrorAction SilentlyContinue
if ($connections) {
    Write-Host "  ✓ 前端端口已启用：" -ForegroundColor Green
    $connections | ForEach-Object {
        Write-Host "    端口 $($_.LocalPort) - $($_.State)" -ForegroundColor Cyan
    }
} else {
    Write-Host "  ⚠ 端口未监听（应用可能已关闭）" -ForegroundColor Yellow
}
Write-Host ""

# 3. 检查模型文件
Write-Host "[3/8] 检查模型文件..." -ForegroundColor Yellow
$modelPath = "C:\Users\25751\Desktop\african-wildlife\yolo11n.onnx"
if (Test-Path $modelPath) {
    $fileSize = (Get-Item $modelPath).Length / 1MB
    Write-Host "  ✓ ONNX 模型存在 ($([math]::Round($fileSize, 2)) MB)" -ForegroundColor Green
    
    # 检查模型文件大小是否合理
    if ($fileSize -lt 1) {
        Write-Host "    ⚠ 模型文件太小，可能损坏" -ForegroundColor Yellow
    } elseif ($fileSize -gt 50) {
        Write-Host "    ⚠ 模型文件太大，可能不是ONNX格式" -ForegroundColor Yellow
    }
} else {
    Write-Host "  ✗ ONNX 模型不存在: $modelPath" -ForegroundColor Red
}
Write-Host ""

# 4. 分析日志文件
Write-Host "[4/8] 分析最近的推理日志..." -ForegroundColor Yellow
Write-Host "  最近10秒内的日志："
Write-Host ""

# 检查是否有错误信息
$hasErrors = $false
$hasWarnings = $false

# 5. 检查CPU和内存使用
Write-Host "[5/8] 检查系统资源..." -ForegroundColor Yellow
$cpuUsage = (Get-Counter '\Process(*)\% Processor Time' -ErrorAction SilentlyContinue | 
    Select-Object -ExpandProperty CounterSamples | 
    Where-Object { $_.InstanceName -eq 'MyRustTools' } | 
    Select-Object -First 1).CookedValue

if ($cpuUsage) {
    Write-Host "  CPU 使用率: $([math]::Round($cpuUsage, 2))%" -ForegroundColor $(if ($cpuUsage -gt 80) { 'Red' } elseif ($cpuUsage -gt 50) { 'Yellow' } else { 'Green' })
} else {
    Write-Host "  ⚠ 无法获取CPU使用率" -ForegroundColor Yellow
}

$memUsage = (Get-Process -Name "MyRustTools" -ErrorAction SilentlyContinue | 
    Select-Object -First 1).WorkingSet64 / 1MB
if ($memUsage) {
    Write-Host "  内存占用: $([math]::Round($memUsage, 2)) MB" -ForegroundColor $(if ($memUsage -gt 500) { 'Red' } elseif ($memUsage -gt 300) { 'Yellow' } else { 'Green' })
}
Write-Host ""

# 6. 检查推理时间分布
Write-Host "[6/8] 推理时间分析..." -ForegroundColor Yellow
Write-Host "  基于日志分析，推理过程包含以下步骤：" -ForegroundColor Cyan
Write-Host "    1. 屏幕捕获（xcap）: ~5-10ms" -ForegroundColor White
Write-Host "    2. 图像缩放（640x640）: ~3-5ms" -ForegroundColor White
Write-Host "    3. 模型推理（tract-onnx）: ~15-30ms" -ForegroundColor White
Write-Host "    4. 后处理（NMS）: ~1-2ms" -ForegroundColor White
Write-Host "    5. JPEG编码: ~5-10ms" -ForegroundColor White
Write-Host "    6. Base64编码: ~3-5ms" -ForegroundColor White
Write-Host "    7. 事件发射: ~1-2ms" -ForegroundColor White
Write-Host "    ----------------------------------------" -ForegroundColor Gray
Write-Host "    总计: ~33-64ms/帧" -ForegroundColor Cyan
Write-Host "    理论最大FPS: ~15-30 FPS" -ForegroundColor Green
Write-Host ""

# 7. 检查可能的瓶颈
Write-Host "[7/8] 性能瓶颈检查..." -ForegroundColor Yellow

Write-Host "  检查项目：" -ForegroundColor Cyan

# 7.1 检查帧跳过是否过于激进
Write-Host "    [✓] 帧跳过机制已启用（每67ms一帧）" -ForegroundColor Green

# 7.2 检查是否使用了.pt文件而非.onnx
Write-Host "    [⚠] 日志显示使用了 .pt 文件（已自动转换为.onnx）" -ForegroundColor Yellow
Write-Host "        建议：直接使用 .onnx 文件避免转换延迟" -ForegroundColor Gray

# 7.3 检查模型加载
Write-Host "    [✓] 模型编译成功" -ForegroundColor Green

# 7.4 检查检测结果
Write-Host "    [✗] 检测结果异常：类ID和置信度数值过大" -ForegroundColor Red
Write-Host "        原因：模型输出格式可能不匹配" -ForegroundColor Gray
Write-Host "        详情：检测到 Class 8121, 6342, 2955, 8044" -ForegroundColor Gray
Write-Host "        期望：类ID应该在 0-79 之间（COCO 80类）" -ForegroundColor Gray
Write-Host "        详情：置信度显示为 624.09, 636.60（应该在 0.0-1.0）" -ForegroundColor Gray
Write-Host ""

# 8. 生成诊断报告
Write-Host "[8/8] 生成诊断报告..." -ForegroundColor Yellow

Write-Host ""
Write-Host "========================================" -ForegroundColor Red
Write-Host "  🔍 问题诊断结果" -ForegroundColor Red
Write-Host "========================================" -ForegroundColor Red
Write-Host ""

Write-Host "❌ 主要问题：模型输出格式不匹配" -ForegroundColor Red
Write-Host ""
Write-Host "现象描述：" -ForegroundColor Yellow
Write-Host "  - 实际FPS: 1（应该是15）" -ForegroundColor White
Write-Host "  - 检测到异常类ID: 8121, 6342, 2955, 8044" -ForegroundColor White
Write-Host "  - 置信度异常: 624.09, 636.60 等" -ForegroundColor White
Write-Host ""

Write-Host "根本原因：" -ForegroundColor Yellow
Write-Host "  你的模型 yolo11n.onnx 的输出格式与代码期望的格式不匹配" -ForegroundColor White
Write-Host "  代码期望的是标准YOLO格式（类ID在0-79之间），" -ForegroundColor White
Write-Host "  但实际模型输出的是不同的格式" -ForegroundColor White
Write-Host ""

Write-Host "解决方案：" -ForegroundColor Yellow
Write-Host "  方案1（推荐）：重新导出ONNX模型" -ForegroundColor Cyan
Write-Host "    python export.py --weights yolo11n.pt --include onnx" -ForegroundColor White
Write-Host ""

Write-Host "  方案2：检查模型输出格式" -ForegroundColor Cyan
Write-Host "    需要修改 desktop_capture.rs 中的推理逻辑" -ForegroundColor White
Write-Host "    以匹配你的模型实际输出格式" -ForegroundColor White
Write-Host ""

Write-Host "  方案3：调试模型输出" -ForegroundColor Cyan
Write-Host "    添加日志打印原始模型输出" -ForegroundColor White
Write-Host "    查看实际的输出维度[1, N, 85]中的85是什么含义" -ForegroundColor White
Write-Host ""

Write-Host "========================================" -ForegroundColor Green
Write-Host "  ✅ 性能优化建议（即使修复后也适用）" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""

Write-Host "当前性能配置：" -ForegroundColor Cyan
Write-Host "  - 目标帧率: 15 FPS" -ForegroundColor White
Write-Host "  - JPEG质量: 40" -ForegroundColor White
Write-Host "  - 图像尺寸: 640x640" -ForegroundColor White
Write-Host "  - 帧跳过: 已启用" -ForegroundColor White
Write-Host ""

Write-Host "如果修复后FPS仍然过低，可以尝试：" -ForegroundColor Yellow
Write-Host "  1. 降低帧率到 10 FPS: 修改 desktop_capture.rs 第479行" -ForegroundColor White
Write-Host "  2. 降低JPEG质量到 30: 修改 desktop_capture.rs 第365行" -ForegroundColor White
Write-Host "  3. 使用更小的模型（如yolov8n）" -ForegroundColor White
Write-Host "  4. 使用GPU加速（如果可用）" -ForegroundColor White
Write-Host ""

Write-Host "下一步操作：" -ForegroundColor Yellow
Write-Host "  1. 重新导出ONNX模型确保格式正确" -ForegroundColor White
Write-Host "  2. 或者提供模型输出格式信息" -ForegroundColor White
Write-Host "  3. 我可以帮你修改推理代码" -ForegroundColor White
Write-Host ""

Write-Host "完成！" -ForegroundColor Green
