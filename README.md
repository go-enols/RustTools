# YOLO-Flow

> 模块化 AI 工作流聚合平台 — 支持 YOLO 检测、爬虫管理、任务流、RPA 自动化

## 核心功能

### 已完成模块

| 模块 | 说明 |
|------|------|
| **Hub 首页** | 模块选择、项目快速访问、最近项目 |
| **YOLO 检测** | 图片标注、模型训练、推理展示、结果分析 |

### 规划中模块

| 模块 | 状态 |
|------|------|
| 爬虫管理 | 规划中 |
| 任务流 | 规划中 |
| RPA 自动化 | 规划中 |

### YOLO 检测模块功能

| 功能 | 说明 |
|------|------|
| 图片标注 | 矩形框标注、支持 YOLO/COCO 格式 |
| 视频取帧 | 固定间隔/关键帧/时间范围提取 |
| 模型训练 | YOLOv8/YOLO11 训练、实时进度监控 |
| 训练可视化 | Loss/mAP/Precision/Recall 曲线 |
| 运行展示 | 图片/视频/桌面帧实时检测 |
| 模型转换 | 支持多种模型格式导出 |

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri 2.x |
| 前端 | React 18 + TypeScript |
| 状态管理 | Zustand |
| 样式 | Tailwind CSS + CSS Variables |
| 图标 | Lucide React |
| 后端 | Rust |
| YOLO 推理 | Candle (Rust ML) |
| 视频处理 | FFmpeg |

## 快速开始

### 环境要求

- Node.js >= 18
- Rust >= 1.75
- Windows 10/11

### 安装依赖

```bash
npm install
```

### 开发

```bash
npm run tauri dev    # Tauri 开发模式（前后端同时启动）
```

### 构建

```bash
npm run tauri build  # 构建生产版本
```

## 项目结构

```
yolo-flow/
├── src/                          # 前端源码
│   ├── main.tsx                  # 应用入口
│   ├── App.tsx                   # 根组件
│   ├── core/                     # 核心基础设施
│   │   ├── api/                  # API 调用
│   │   ├── components/layout/   # 布局组件
│   │   ├── stores/               # Zustand 状态
│   │   └── styles/               # 全局样式
│   ├── modules/                  # 模块系统
│   │   ├── types.ts              # 模块类型定义
│   │   ├── registry.ts           # 模块注册中心
│   │   ├── hub/                  # Hub 首页
│   │   └── yolo/                 # YOLO 检测模块
│   └── shared/                   # 共享组件
│
├── src-tauri/                    # Rust 后端
│   └── src/
│       ├── main.rs               # 应用入口
│       ├── lib.rs                # 库入口
│       ├── core/                  # 核心基础设施
│       ├── modules/              # 模块系统
│       └── shared/               # 共享工具
│
├── doc/                          # 项目文档
│   ├── 进行中/                   # 正在开发的功能
│   ├── 已完成/                   # 已完成的功能
│   └── 归档/                    # 已归档的文档
│
├── CLAUDE.md                     # 开发规范
├── SPEC.md                       # 详细功能规格
└── README.md                      # 项目说明
```

### 目录原则

| 目录 | 说明 |
|------|------|
| `core/` | 核心代码，仅限查阅，不可随意修改 |
| `modules/` | 每个模块独立，包含自己的 pages/components |
| `shared/` | 多模块共用的 UI 组件 |

## 模块开发

### 模块结构

每个模块包含以下部分：

```
modules/{module-name}/
├── index.ts           # 模块入口（注册）
├── manifest.ts        # 模块清单
├── pages/             # 页面组件
└── components/        # 模块内共享组件
```

### 模块清单定义

```typescript
interface ModuleManifest {
  id: string;           // 唯一标识
  name: string;         // 显示名称
  icon: string;         // Lucide 图标名
  description: string;  // 模块描述
  version: string;       // 模块版本
  order: number;         // 排序顺序
  capabilities: string[]; // 支持的能力列表
}
```

## 文档

- [CLAUDE.md](./CLAUDE.md) - 开发规范与目录结构
- [SPEC.md](./SPEC.md) - 详细功能规格与 UI 规格

## 许可证

MIT License
