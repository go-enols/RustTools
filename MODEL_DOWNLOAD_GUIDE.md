# YOLO模型下载地址说明

## 当前状态

经过验证，以下数据源可用性如下：

| 数据源 | 状态 | 原因 |
|--------|------|------|
| ModelScope | ✓ 可用 | 国内镜像，访问正常 |
| GitHub | ✗ 不可用 | 404错误（版本号可能不正确） |
| HuggingFace | ✗ 不可用 | 401认证错误（需要登录或代理） |

## 推荐的下载源

### 1. ModelScope (国内推荐)

ModelScope是国内可用的镜像站点，访问速度快。

**注意**：ModelScope页面URL不是直接下载链接，需要使用API或SDK下载。

#### 使用Python SDK下载：
```python
from modelscope.hub.api import HubApi
api = HubApi()
# 登录获取token: https://modelscope.cn/my/token
api.login("your_token")
api.model_download('AI-ModelScope/YOLOv8n', 'yolov8n.onnx')
```

#### 或使用pip安装后下载：
```bash
pip install modelscope
python -c "from modelscope.hub.api import HubApi; api = HubApi(); api.model_download('AI-ModelScope/YOLOv8n', 'yolov8n.onnx')"
```

### 2. HuggingFace (需要配置)

HuggingFace的YOLO模型页面：
- https://huggingface.co/ultralytics/yolov8n
- https://huggingface.co/onnxruntime/yolov8n

#### 使用huggingface-cli下载：
```bash
pip install huggingface_hub
huggingface-cli download ultralytics/yolov8n yolov8n.onnx
```

#### 或使用Python：
```python
from huggingface_hub import hf_hub_download
path = hf_hub_download(repo_id="ultralytics/yolov8n", filename="yolov8n.onnx")
```

## 模型下载地址

### ModelScope

ModelScope上的YOLOv8模型：
- YOLOv8n: https://www.modelscope.cn/models/AI-ModelScope/YOLOv8n
- YOLOv8s: https://www.modelscope.cn/models/AI-ModelScope/YOLOv8s
- YOLOv8m: https://www.modelscope.cn/models/AI-ModelScope/YOLOv8m
- YOLOv8l: https://www.modelscope.cn/models/AI-ModelScope/YOLOv8l
- YOLOv8x: https://www.modelscope.cn/models/AI-ModelScope/YOLOv8x

### HuggingFace

HuggingFace上的YOLOv8 ONNX模型：
- ultralytics官方: https://huggingface.co/ultralytics/yolov8n
- onnxruntime官方: https://huggingface.co/onnxruntime/yolov8n

## GitHub官方资源

虽然直接访问GitHub assets失败，但YOLOv8的源代码和预训练权重在：
- https://github.com/ultralytics/ultralytics
- https://github.com/ultralytics/assets

可以在浏览器中手动访问下载。

## 模型版本说明

| 模型 | 参数量 | ONNX大小 | 适用场景 |
|------|--------|----------|----------|
| yolov8n | 3.2M | ~6MB | 实时推理、资源受限 |
| yolov8s | 11.2M | ~22MB | 平衡性能和速度 |
| yolov8m | 25.9M | ~52MB | 高精度需求 |
| yolov8l | 53.7M | ~108MB | 极高精度 |
| yolov8x | 68.2M | ~136MB | 最高精度 |

## 建议

1. **国内用户**：使用ModelScope下载，速度快
2. **有代理的用户**：可以使用HuggingFace
3. **手动下载**：访问上述URL手动下载到本地

## 验证脚本

运行 `verify_model_urls.py` 可以测试各数据源的可访问性。
