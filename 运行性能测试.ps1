# 桌面推理性能测试脚本
# 用于诊断推理帧率为0的问题

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "桌面推理性能测试" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 检查模型文件是否存在
$modelPath = "C:\Users\25751\Desktop\african-wildlife\yolo11n.onnx"
if (-not (Test-Path $modelPath)) {
    Write-Host "❌ 模型文件不存在: $modelPath" -ForegroundColor Red
    exit 1
}

Write-Host "✅ 模型文件存在: $modelPath" -ForegroundColor Green
Write-Host ""

# 运行性能测试
Write-Host "开始运行性能测试..." -ForegroundColor Yellow
Write-Host ""

cargo test --package rust-tools --lib modules::yolo::services::desktop_performance_test::tests::test_performance_analysis -- --nocapture 2>&1 | Tee-Object -Variable testOutput

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "测试完成" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 分析输出
if ($testOutput -match "Class Scores.*Max:\s*([\d.]+)") {
    $maxScore = $matches[1]
    Write-Host "检测到的最大类别分数: $maxScore" -ForegroundColor Yellow
    
    if ([float]$maxScore -gt 1.0) {
        Write-Host "⚠️  警告: 类别分数 > 1.0,模型可能输出了原始logits而不是sigmoid概率" -ForegroundColor Red
        Write-Host "   可能需要在后处理中添加sigmoid激活函数" -ForegroundColor Red
    }
}

if ($testOutput -match "BBox Coordinates.*Max:\s*([\d.]+)") {
    $maxBBox = $matches[1]
    Write-Host "检测到的最大边界框值: $maxBBox" -ForegroundColor Yellow
    
    if ([float]$maxBBox -gt 640) {
        Write-Host "⚠️  警告: 边界框值 > 640,坐标可能是归一化的(0-1范围)" -ForegroundColor Red
        Write-Host "   可能需要乘以图像尺寸来转换为绝对像素坐标" -ForegroundColor Red
    } else {
        Write-Host "✅ 边界框值在合理范围内" -ForegroundColor Green
    }
}

if ($testOutput -match "Detected\s+(\d+)\s+classes") {
    $numClasses = $matches[1]
    Write-Host "模型检测到的类别数: $numClasses" -ForegroundColor Yellow
    
    if ([int]$numClasses -eq 4) {
        Write-Host "✅ 这是野生动物模型(4个类别: elephant, buffalo, rhino, zebra)" -ForegroundColor Green
    } elseif ([int]$numClasses -eq 80) {
        Write-Host "⚠️  这是COCO模型(80个类别)" -ForegroundColor Yellow
    }
}

if ($testOutput -match "Average FPS:\s*([\d.]+)") {
    $avgFPS = $matches[1]
    Write-Host "平均FPS: $avgFPS" -ForegroundColor Cyan
    
    if ([float]$avgFPS -lt 1.0) {
        Write-Host "❌ FPS过低(<1),存在严重的性能问题" -ForegroundColor Red
    } elseif ([float]$avgFPS -lt 5.0) {
        Write-Host "⚠️  FPS较低(<5),可能存在性能瓶颈" -ForegroundColor Yellow
    } else {
        Write-Host "✅ FPS正常" -ForegroundColor Green
    }
}

Write-Host ""
Write-Host "详细测试输出已保存,可以从上面的日志中查看完整信息" -ForegroundColor Cyan
