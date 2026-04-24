# RustTools

AI 视觉开发工具箱 —— YOLO 目标检测模型的训练、推理、标注一体化桌面应用。

## 技术栈

- **前端**: React 18 + TypeScript + Vite + TailwindCSS
- **桌面框架**: Tauri v2 (Rust)
- **推理引擎**: ONNX Runtime (`ort`) + 自定义 YOLO 后处理
- **桌面捕获**: `scrap` (直接帧缓冲访问)
- **构建工具**: Cargo + npm

## 功能概览

| 模块 | 功能 |
|------|------|
| **Hub** | 项目管理、数据集统计、快捷入口 |
| **Project** | 创建/打开 YOLO 项目、类别管理 |
| **Annotation** | 图片标注（支持 train/val 切换）、YOLO 格式导出 |
| **Image Inference** | 单图推理、画布叠加检测框、置信度筛选 |
| **Video Inference** | 视频推理、逐帧/自动检测、播放控制 |
| **Desktop Capture** | 实时屏幕捕获 + YOLO 推理、双线程架构 |
| **Training** | YOLO 模型训练（通过 Python Ultralytics） |
| **Device** | GPU/CPU 信息、CUDA 状态检测 |
| **Settings** | 主题切换、全局配置 |

## 项目结构

```
RustTools/
├── frontend/               # React 前端
│   ├── src/
│   │   ├── pages/          # 页面组件
│   │   ├── components/     # 共享组件
│   │   ├── contexts/       # React Context
│   │   └── __tests__/      # 测试文件
│   ├── package.json
│   └── vite.config.ts
├── src-tauri/              # Tauri v2 Rust 后端
│   ├── src/lib.rs          # Tauri 命令入口
│   ├── capabilities/
│   └── tauri.conf.json
├── crates/app/             # 共享 Rust crate
│   └── src/
│       ├── models/         # 数据模型
│       ├── services/       # 业务服务
│       └── ui/             # egui 原生界面（可选编译）
└── docs/
    └── APPLE_UI_DESIGN_SPEC.md
```

## 快速开始

### 开发环境

```bash
# 1. 安装依赖
cd frontend && npm install

# 2. 启动开发服务器（前端 + Tauri）
npm run tauri dev          # 在 frontend/ 目录下
# 或
cargo tauri dev            # 在 src-tauri/ 目录下
```

### 构建 Release

```bash
# 前端构建
cd frontend && npm run build

# 完整应用构建
cargo tauri build
```

### 测试

```bash
# 前端测试
cd frontend && npx vitest run

# 后端测试
cargo test
```

## 模型下载

首次使用推理功能前，需要下载 YOLO 模型。应用支持自动下载，也可手动放置：

- 自动下载：在推理页面选择模型，应用会自动从 Ultralytics releases 下载
- 手动放置：将 `.pt` 或 `.onnx` 模型放到 `~/.cache/ultralytics/` 目录

详见 [MODEL_DOWNLOAD_GUIDE.md](./MODEL_DOWNLOAD_GUIDE.md)

## 标注数据格式

采用标准 YOLO 格式：

```
project/
├── images/
│   ├── train/
│   └── val/
├── labels/
│   ├── train/
│   └── val/
└── data.yaml
```

每张图片对应一个同名的 `.txt` 标注文件，每行格式：
```
<class_id> <x_center> <y_center> <width> <height>
```

所有坐标均为相对于图片尺寸的归一化值 (0-1)。

## 许可证

MIT License
