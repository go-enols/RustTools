# 模型下载地址验证脚本
# 验证HuggingFace上YOLO ONNX模型的可访问性

Write-Host "=== YOLO模型下载地址验证 ===" -ForegroundColor Cyan
Write-Host ""

$models = @{
    "yolov8n" = "https://huggingface.co/onnxruntime/yolov8n/resolve/main/yolov8n.onnx"
    "yolov8s" = "https://huggingface.co/onnxruntime/yolov8s/resolve/main/yolov8s.onnx"
    "yolov8m" = "https://huggingface.co/onnxruntime/yolov8m/resolve/main/yolov8m.onnx"
    "yolov8l" = "https://huggingface.co/onnxruntime/yolov8l/resolve/main/yolov8l.onnx"
    "yolov8x" = "https://huggingface.co/onnxruntime/yolov8x/resolve/main/yolov8x.onnx"
}

$results = @()

foreach ($model in $models.GetEnumerator()) {
    Write-Host "检查 $($model.Key)..." -NoNewline
    
    try {
        $response = Invoke-WebRequest -Uri $model.Value -Method Head -TimeoutSec 10 -ErrorAction Stop
        $statusCode = $response.StatusCode
        $contentLength = $response.Headers["Content-Length"]
        
        if ($statusCode -eq 200 -and $contentLength) {
            $sizeMB = [math]::Round([int64]$contentLength / 1MB, 2)
            Write-Host " ✓ OK ($sizeMB MB)" -ForegroundColor Green
            $results += [PSCustomObject]@{
                Model = $model.Key
                Status = "OK"
                Size = "$sizeMB MB"
                URL = $model.Value
            }
        } else {
            Write-Host " ⚠ Status: $statusCode" -ForegroundColor Yellow
            $results += [PSCustomObject]@{
                Model = $model.Key
                Status = "Warning"
                Size = "Unknown"
                URL = $model.Value
            }
        }
    }
    catch {
        Write-Host " ✗ Failed: $($_.Exception.Message)" -ForegroundColor Red
        $results += [PSCustomObject]@{
            Model = $model.Key
            Status = "Failed"
            Size = "N/A"
            URL = $model.Value
        }
    }
}

Write-Host ""
Write-Host "=== 验证结果摘要 ===" -ForegroundColor Cyan
$results | Format-Table -AutoSize

# 检查可用的镜像源
Write-Host ""
Write-Host "=== 备用镜像源 ===" -ForegroundColor Cyan
Write-Host "如果主地址无法访问，可以使用以下镜像："
Write-Host "  - hf-mirror.com: https://hf-mirror.com/onnxruntime/yolov8n/resolve/main/yolov8n.onnx"
Write-Host "  - HuggingFace CLI: huggingface-cli download onnxruntime/yolov8n yolov8n.onnx"

# 建议的替代方案
Write-Host ""
Write-Host "=== 替代方案 ===" -ForegroundColor Cyan
Write-Host "1. 使用 modelscope 镜像："
Write-Host "   https://www.modelscope.cn/models/AI-ModelScope/YOLOv8n"
Write-Host ""
Write-Host "2. 使用国内镜像："
Write-Host "   https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8n.onnx"
Write-Host ""
Write-Host "3. 直接从Ultralytics官方下载（需要Python）："
Write-Host "   pip install ultralytics"
Write-Host "   python -c 'from ultralytics import YOLO; m=YOLO(\"yolov8n.pt\"); m.export(format=\"onnx\")'"
