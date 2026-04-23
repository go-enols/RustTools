# RustTools Python 版 — 实现规格

## 误差向量审计链

| Phase | Agent | Error Magnitude | Convergence | 控制器决策 |
|-------|-------|-----------------|------------|-----------|
| Phase 1 | architect | 0.25 | ~0.5 | 通过 |
| Phase 1 | devils-advocate | 0.45 | 0.45 | **阻尼触发**：oscillation=true |
| Phase 1 | product-manager | 0.30 | 0.6 | 通过 |

**阻尼决策**：devils-advocate 与 architect 误差差值 0.2，但 convergence_score < 0.5，产品经理论据收敛到 0.6 → **控制器接受 NiceGUI 过渡方案，长期原生方案待评估**。

---

## 项目概述

- **项目名称**: RustTools (Python Version)
- **项目类型**: 桌面应用 (NiceGUI + Python)
- **核心功能**: YOLO 系列模型的训练、推理、标注、视频处理一体化工具
- **目标用户**: 计算机视觉研究者、数据标注工程师、YOLO 模型开发者
- **技术栈**: Python 3.11+ / NiceGUI / uv / torch 2.5.1+cu124 / ultralytics

---

## 关键设计决策

### YOLO 调用：无进程模式
**决策**：YOLO 直接作为后端函数调用，不启动独立服务器进程。

```python
import asyncio
from ultralytics import YOLO

_model_cache: dict[str, YOLO] = {}

def get_model(model_path: str) -> YOLO:
    if model_path not in _model_cache:
        _model_cache[model_path] = YOLO(model_path)
    return _model_cache[model_path]

async def async_detect(image_path: str, model_path: str) -> dict[str, Any]:
    """所有 YOLO 调用必须用 asyncio.to_thread 包装，避免 GIL 阻塞 UI"""
    loop = asyncio.get_event_loop()
    model = get_model(model_path)
    return await loop.run_in_executor(None, lambda: model(image_path)[0].to_dict())
```

### GIL 阻塞缓解（Phase 1 结论）
- **所有 YOLO 推理/训练调用必须 `await asyncio.to_thread()` 包装**
- `torch.set_num_threads(1)` 避免多线程竞争
- UI 永远不在主线程执行 torch 操作

---

## 项目目录结构

```
~/github/RustTools/
├── pyproject.toml
├── .python-version              # 3.11
├── SPEC.md
├── README.md
│
├── core/                        # 【只读，禁止日常修改】
│   ├── __init__.py
│   ├── api/
│   │   ├── __init__.py
│   │   └── types.py             # TypedDict 类型定义
│   ├── models/
│   │   ├── __init__.py
│   │   ├── error.py             # AppError
│   │   └── response.py          # ok() / err()
│   ├── services/
│   │   ├── __init__.py
│   │   └── logger.py            # loguru init
│   └── stores/
│       ├── __init__.py
│       ├── router.py            # 路由状态
│       ├── workspace.py         # 工作区状态
│       ├── training.py          # 训练状态
│       └── settings.py          # 设置状态
│
├── modules/                     # 【可扩展】
│   ├── __init__.py
│   ├── types.py                 # ModuleManifest, CAPABILITY_*
│   ├── registry.py              # ModuleRegistry 单例
│   ├── hub/
│   │   ├── __init__.py
│   │   └── page.py
│   └── yolo/
│       ├── __init__.py
│       ├── manifest.py          # YOLO_MANIFEST
│       ├── api/                  # 前端 API 调用层
│       │   ├── __init__.py
│       │   ├── inference.py
│       │   ├── training.py
│       │   ├── annotation.py
│       │   └── video.py
│       ├── services/
│       │   ├── __init__.py
│       │   ├── detector.py      # YOLO 推理（asyncio.to_thread）
│       │   ├── trainer.py       # YOLO 训练（asyncio.to_thread）
│       │   ├── annotator.py     # 标注服务
│       │   ├── model_manager.py # 模型下载/管理
│       │   └── converter.py     # 格式转换
│       ├── pages/
│       │   ├── __init__.py
│       │   ├── inference.py
│       │   ├── training.py
│       │   ├── annotation.py
│       │   ├── video.py
│       │   └── results.py
│       └── components/
│           ├── __init__.py
│           ├── activity_bar.py
│           ├── sidebar.py
│           ├── status_bar.py
│           ├── training_panel.py
│           ├── model_selector.py
│           └── progress.py
│
└── shared/
    ├── __init__.py
    └── components/
        ├── __init__.py
        ├── button.py
        ├── modal.py
        ├── card.py
        ├── tabs.py
        ├── progress.py
        ├── badge.py
        └── toast.py
```

---

## 依赖 (pyproject.toml)

```toml
[project]
name = "rusttools"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = [
    "nicegui>=1.6.0",
    "ultralytics>=8.3.0",
    "torch>=2.5.0",
    "torchvision>=0.20.0",
    "opencv-python>=4.10.0",
    "pillow>=10.0.0",
    "numpy>=1.26.0",
    "pydantic>=2.9.0",
    "loguru>=0.7.0",
    "requests>=2.32.0",
    "tqdm>=4.66.0",
]

[project.optional-dependencies]
dev = ["ruff>=0.8.0", "mypy>=1.13.0", "pytest>=8.3.0"]

[tool.ruff]
line-length = 100
target-version = "py311"

[tool.mypy]
python_version = "3.11"
strict = true
warn_return_any = true
warn_unused_ignores = true
```

---

## 验收标准

| # | Criterion | Priority | Error Tolerance | How to Test |
|---|-----------|----------|-----------------|-------------|
| 1 | `uv run python main.py` 启动 NiceGUI | P0 | magnitude = 0 | 浏览器访问 localhost:5173 |
| 2 | Hub 页面显示模块入口 | P0 | magnitude ≤ 0.1 | 截图 |
| 3 | YOLO 推理（图片）返回检测结果 | P0 | magnitude = 0 | 加载 yolo26n.pt |
| 4 | 推理时 UI 不卡顿（asyncio.to_thread） | P0 | magnitude = 0 | 推理期间点击按钮响应 |
| 5 | YOLO 训练启动并显示进度 | P0 | magnitude ≤ 0.1 | 观察进度条更新 |
| 6 | 标注页面可绘制 bounding box | P0 | magnitude = 0 | 手动交互 |
| 7 | 所有 Python 代码有类型标记 | P0 | magnitude = 0 | `mypy --strict` 零错误 |
| 8 | GPU 检测正确显示 CUDA/cpu | P1 | magnitude ≤ 0.1 | 设备页面 |

---

## 风险记录（Phase 1 结论）

| 风险 | magnitude | 缓解措施 |
|------|----------|---------|
| GIL 阻塞 UI | 0.35 | **强制** `asyncio.to_thread` 包装所有 YOLO 调用 |
| NiceGUI 桌面局限性 | 0.15 | 接受（Nginx/Chrome PWA 模式可弥补） |
| Rust 架构映射合理性 | 0.25 | Python 用模块内扁平结构（api/services/pages）替代 Rust 的 commands/services/models 分层 |

---

## TODO

- [ ] P0: 搭建 uv + NiceGUI 项目骨架
- [ ] P0: core/ 模块
- [ ] P0: modules/registry + hub
- [ ] P0: modules/yolo/services/detector.py（**asyncio.to_thread**）
- [ ] P0: modules/yolo/services/trainer.py（**asyncio.to_thread**）
- [ ] P0: modules/yolo/pages/（所有页面）
- [ ] P1: modules/yolo/services/annotator.py
- [ ] P1: shared/components
- [ ] Gate 1: `ruff check .` + `mypy --strict`
- [ ] Gate 2: NiceGUI Runtime Verification
