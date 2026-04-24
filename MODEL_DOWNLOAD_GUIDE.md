# YOLO模型下载地址

## 重要发现

经过验证，发现：
- **Ultralytics assets仓库最新版本是YOLO11**（不是YOLOv8）
- YOLOv8的ONNX模型已经不再在assets仓库中发布
- 最新的URL格式：`https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11n.onnx`

## 正确的下载地址

### YOLO11 系列（最新，推荐）

| 模型 | URL |
|------|-----|
| yolo11n | https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11n.onnx |
| yolo11s | https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11s.onnx |
| yolo11m | https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11m.onnx |
| yolo11l | https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11l.onnx |
| yolo11x | https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11x.onnx |

### YOLOv8 系列（需要从其他地方获取）

YOLOv8的ONNX模型不再在GitHub assets仓库中。建议：

1. **使用Python导出**：
   ```bash
   pip install ultralytics
   python -c "from ultralytics import YOLO; m = YOLO('yolov8n.pt'); m.export(format='onnx')"
   ```

2. **使用HuggingFace**（需要认证或代理）：
   ```
   https://huggingface.co/ultralytics/yolov8n/tree/main
   ```

3. **使用ModelScope**：
   ```
   https://www.modelscope.cn/models/AI-ModelScope/YOLOv8n
   ```

## 模型对比

| 版本 | 特点 | 适用场景 |
|------|------|----------|
| YOLO11 | 最新架构，性能更好 | 追求最佳性能 |
| YOLOv8 | 成熟稳定，资料多 | 兼容性好，生态完善 |

## 建议

1. **推理**：推荐使用YOLO11（最新）
2. **训练**：可以基于YOLOv8或YOLO11进行训练
3. **兼容性**：如果需要兼容老的训练流程，继续使用YOLOv8

## 手动下载

如果自动下载失败，可以手动从以下地址下载：

1. 访问 https://github.com/ultralytics/assets/releases
2. 查找 v8.4.0 或更高版本
3. 下载 yolo11n.onnx 等文件
4. 保存到 `~/.cache/ultralytics/` 目录
