# 桌面推理性能分析脚本

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  桌面推理性能分析" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 启动 Tauri 开发服务器
Write-Host "[1/3] 启动 Tauri 开发服务器..." -ForegroundColor Yellow
Write-Host "请在浏览器中打开应用" -ForegroundColor Gray
Write-Host "- 进入桌面推理页面" -ForegroundColor Gray
Write-Host "- 选择一个模型文件" -ForegroundColor Gray
Write-Host "- 点击开始捕获" -ForegroundColor Gray
Write-Host ""

$continue = Read-Host "准备就绪后按 Enter 继续（查看性能日志）..."
Write-Host ""
Write-Host "[2/3] 性能分析说明" -ForegroundColor Yellow
Write-Host ""
Write-Host "当开始捕获后，终端会输出详细的性能日志，格式如下：" -ForegroundColor Gray
Write-Host ""
Write-Host "[PERF-Frame] ===== 第 X 帧 =====" -ForegroundColor White
Write-Host "[PERF-Capture] 屏幕捕获: XXms" -ForegroundColor White
Write-Host "[PERF-Resize] Resize到640x640: XXms" -ForegroundColor White
Write-Host "[PERF-Inference] 推理总耗时: XXms" -ForegroundColor White
Write-Host "  [PERF-Inference] 预处理: XXms" -ForegroundColor Gray
Write-Host "  [PERF-Inference] 模型推理: XXms" -ForegroundColor Gray
Write-Host "  [PERF-Inference] 后处理: XXms" -ForegroundColor Gray
Write-Host "  [PERF-Inference] NMS: XXms" -ForegroundColor Gray
Write-Host "[PERF-Draw] 画 X 个框: XXms" -ForegroundColor White
Write-Host "[PERF-Encode]" -ForegroundColor White
Write-Host "  Resize: XXms | JPEG: XXms | Base64: XXms | 总计: XXms" -ForegroundColor Gray
Write-Host "[PERF-Emit] 发送帧: XXms" -ForegroundColor White
Write-Host "[PERF-Frame] 第 X 帧总计: XXms" -ForegroundColor Green
Write-Host ""

Write-Host "关键指标说明：" -ForegroundColor Cyan
Write-Host "- 屏幕捕获: 如果 >20ms,说明屏幕分辨率过高" -ForegroundColor Gray
Write-Host "- Resize: 如果 >10ms,说明resize算法不够快" -ForegroundColor Gray
Write-Host "- 模型推理: 如果 >100ms,说明ONNX模型或tract性能问题" -ForegroundColor Gray
Write-Host "- JPEG编码: 如果 >20ms,说明JPEG质量过高" -ForegroundColor Gray
Write-Host "- Base64编码: 如果 >50ms,说明图像太大" -ForegroundColor Gray
Write-Host ""

Write-Host "[3/3] 开始监控性能日志..." -ForegroundColor Yellow
Write-Host "请在另一个终端中观察输出" -ForegroundColor Gray
Write-Host "或者直接在这里查看 Tauri 的输出" -ForegroundColor Gray
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  性能分析提示" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "正常运行几秒后，观察以下指标：" -ForegroundColor White
Write-Host ""
Write-Host "1. FPS (实际帧率)" -ForegroundColor Yellow
Write-Host "   - 如果 FPS < 5: 严重性能问题" -ForegroundColor Gray
Write-Host "   - 如果 FPS < 10: 中等性能问题" -ForegroundColor Gray
Write-Host "   - 如果 FPS >= 10: 基本可接受" -ForegroundColor Gray
Write-Host ""
Write-Host "2. 各阶段耗时占比" -ForegroundColor Yellow
Write-Host "   - 理想情况: 推理 < 50%, 编码 < 30%, 其他 < 20%" -ForegroundColor Gray
Write-Host ""
Write-Host "3. Base64编码占比" -ForegroundColor Yellow
Write-Host "   - Base64 会增加 33% 数据大小" -ForegroundColor Gray
Write-Host "   - 如果 Base64 耗时 > 50ms: 考虑使用二进制传输" -ForegroundColor Gray
Write-Host ""

Write-Host "收集足够数据后，可以停止捕获" -ForegroundColor Cyan
Write-Host "我将根据日志分析性能瓶颈并提供优化建议" -ForegroundColor Cyan
Write-Host ""
