## 架构描述（2026-04-05 详细更新）

### 项目概述

MyRustTools 是一个基于 Tauri + React + TypeScript + Rust 的桌面应用程序，采用模块化架构设计，支持 YOLO 目标检测、爬虫管理、RPA 自动化等功能扩展。

### 技术栈

- **前端**: React 18 + TypeScript + Vite + Tailwind CSS + Zustand
- **后端**: Rust + Tauri 2.x + Serde
- **UI组件**: Lucide React 图标库
- **状态管理**: Zustand (前端全局状态)
- **构建工具**: Vite (前端) + Cargo (后端)
- **包管理**: npm/pnpm (前端) + Cargo (后端)

### 前端架构 (src/)

#### 核心基础设施 (core/) - 【只读，不可随意修改】

```
core/
├── api/                           # 全局 API 调用层
│   ├── index.ts                  # API 统一导出
│   ├── types.ts                  # 共享类型定义
│   ├── common.ts                 # 通用功能 (文件对话框、版本检查等)
│   ├── project.ts                # 项目管理 API
│   ├── dataset.ts                # 数据集管理 API
│   ├── annotation.ts             # 标注功能 API
│   ├── training.ts               # 训练功能 API
│   ├── model.ts                  # 模型管理 API
│   ├── device.ts                 # 设备管理 API
│   ├── video.ts                  # 视频处理 API
│   └── settings.ts               # 设置管理 API
├── components/                    # 全局共享组件
│   └── layout/                   # 通用布局组件
│       ├── AppShell.tsx          # 主应用外壳（接收 sidebar 和 children 作为 props）
│       └── TitleBar.tsx          # 自定义标题栏
├── stores/                        # 全局状态管理
│   ├── routerStore.ts            # 路由状态 (模块切换、页面导航)
│   ├── workspaceStore.ts         # 工作区状态
│   ├── trainingStore.ts          # 训练状态
│   └── settingsStore.ts          # 设置状态
└── styles/                        # 全局样式
    ├── index.css                 # 全局样式 + 标题栏样式
    └── hub.css                   # Hub 页面专用样式
```

**核心层职责**:

- 提供统一的 API 调用接口
- 管理全局状态 (路由、设置、工作区)
- 定义共享的 TypeScript 类型
- 实现基础的 UI 布局组件
- 处理跨模块的通用功能

#### 模块系统 (modules/) - 【可扩展，支持热插拔】

```
modules/
├── types.ts                      # 模块类型定义
├── registry.ts                   # 模块注册中心 (单例模式)
├── hub/                          # Hub 首页模块
│   └── HubPage.tsx
└── yolo/                         # YOLO 检测模块
    ├── manifest.ts               # 模块清单 (元数据)
    ├── pages/                    # 模块页面
    │   ├── AnnotationPage.tsx    # 标注页面
    │   ├── TrainingPage.tsx      # 训练页面
    │   ├── ResultsPage.tsx       # 结果页面
    │   ├── VideoPage.tsx         # 视频页面
    │   ├── DevicePage.tsx        # 设备页面
    │   └── ToolsPage.tsx         # 工具页面
    └── components/                # 模块专用组件
        ├── layout/               # 模块专用布局
        │   ├── ActivityBar.tsx  # YOLO 活动栏
        │   ├── Sidebar.tsx      # YOLO 侧边栏
        │   └── StatusBar.tsx    # YOLO 状态栏
        ├── NewProjectModal.tsx  # 新建项目弹窗（YOLO 专用）
        ├── HelpModal.tsx        # 帮助弹窗（YOLO 专用）
        ├── TrainingPanel.tsx    # 训练面板
        └── ModelConvertModal.tsx # 模型转换弹窗
```

**模块系统设计**:

- **模块清单 (ModuleManifest)**: 定义模块的 id、名称、图标、描述、版本、排序、能力列表
- **模块注册中心 (ModuleRegistry)**: 单例模式，管理模块的注册、卸载、查询
- **模块能力 (CapabilityType)**: 'annotation' | 'training' | 'inference' | 'crawling' | 'automation' | 'taskflow'
- **当前状态**: 模块注册中心已实现，但模块热插拔机制尚未完全实现 (YOLO模块直接在App.tsx中注册)

#### 共享组件 (shared/) - 【跨模块复用】

```
shared/
├── components/                   # 共享 UI 组件
│   ├── ui/                       # 基础 UI 组件
│   │   ├── Button.tsx
│   │   └── Modal.tsx
│   └── pages/                    # 共享页面组件
│       └── HubPage.tsx           # Hub 首页
└── stores/                       # 共享状态 (如有需要)
```

### 后端架构 (src-tauri/src/)

#### 核心基础设施 (core/) - 【只读，不可随意修改】

```
core/
├── commands/                     # Tauri 命令层
│   ├── mod.rs                    # 命令模块导出
│   ├── file_commands.rs          # 文件操作命令
│   └── system_commands.rs        # 系统命令
├── models/                       # 数据模型
│   ├── mod.rs                    # 模型导出
│   ├── error.rs                  # 统一错误类型
│   └── response.rs               # API 响应格式
└── services/                     # 公共服务
    ├── mod.rs                    # 服务导出
    └── logger.rs                 # 日志服务
```

**核心层职责**:

- 定义统一的错误处理和响应格式
- 提供基础的文件系统操作
- 实现日志记录服务
- 处理 Tauri 命令的基础设施

#### 模块系统 (modules/) - 【可扩展，支持热插拔】

```
modules/
└── yolo/                         # YOLO 检测模块
    ├── mod.rs                    # 模块入口
    ├── commands/                 # 模块命令
    │   ├── mod.rs                # 命令导出
    │   ├── project.rs            # 项目管理命令 (已实现)
    │   ├── train.rs              # 训练命令 (待实现)
    │   ├── detect.rs             # 推理命令 (待实现)
    │   └── export.rs             # 导出命令 (待实现)
    ├── services/                 # 模块服务
    │   ├── mod.rs                # 服务导出
    │   ├── trainer.rs            # 训练服务 (待实现)
    │   └── detector.rs           # 检测服务 (待实现)
    └── models/                   # 模块数据模型
        ├── mod.rs                # 模型导出
        └── config.rs             # YOLO 配置模型 (待实现)
```

**模块命令设计**:

- 每个命令使用 `#[tauri::command]` 宏标记
- 返回 `Result<T, String>` 统一错误处理
- 命令命名: `{module}_{action}` (如 `yolo_project_create`)
- **当前状态**: 仅实现了项目管理相关命令，其他功能命令待开发

#### 共享工具 (shared/) - 【跨模块复用】

```
shared/
└── utils/
    ├── mod.rs
    └── path.rs                    # 路径处理工具
```

### 状态管理架构

#### 前端状态分层

1. **路由状态 (routerStore)**: 管理模块切换、页面导航、路由参数
2. **工作区状态 (workspaceStore)**: 管理当前项目、文件状态
3. **训练状态 (trainingStore)**: 管理训练进程、日志、结果
4. **设置状态 (settingsStore)**: 管理应用配置

#### 状态更新模式

- 使用 Zustand 的 `create` 函数创建 store
- 支持订阅模式监听状态变化
- 模块间状态解耦，避免直接依赖

### API 架构

#### 前端 API 层

- **统一导出**: `core/api/index.ts` 导出所有 API 函数
- **类型安全**: 所有 API 调用都有完整的 TypeScript 类型
- **错误处理**: 统一的错误处理和用户提示
- **模块化**: 按功能划分 API 文件 (project.ts, training.ts 等)

#### 后端命令层

- **命令模式**: 每个功能对应一个 Tauri 命令
- **序列化**: 使用 Serde 进行 JSON 序列化/反序列化
- **异步处理**: 支持 `async fn` 的异步命令
- **错误传播**: 统一的 `Result<T, String>` 错误处理

### 文档管理架构

#### 文档目录结构

```
doc/
├── 0-index.md                    # 模块清单索引
├── 进行中/                        # 开发中功能文档
│   └── {序号}-{功能名}-{日期}.md
├── 已完成/                        # 已完成功能
│   ├── 已完成项目/                # 已完成但不维护
│   └── 正在维护/                  # 正在维护的功能
└── 归档/                         # 稳定功能
```

#### 文档规范

- **索引驱动**: 所有功能必须在 `0-index.md` 中登记
- **状态追踪**: 明确标注功能状态 (进行中/已完成/归档)
- **进度管理**: 进行中功能需标注完成百分比
- **更新日志**: 记录每次文档更新

---

## 架构约束与开发规范

### 核心约束

#### 1. 核心代码只读原则

**约束**: `core/` 目录下的代码禁止直接修改
**理由**: 核心基础设施应保持稳定，避免模块开发时意外破坏基础功能
**例外**: 只有在进行重大架构重构时，且经过充分讨论后才能修改
**替代方案**: 通过扩展机制 (如模块系统) 实现新功能

#### 2. 模块热插拔约束

**约束**: 新增/移除模块不得修改核心代码
**实现方式**:

- 模块通过 `ModuleRegistry.register()` 注册
- 模块清单 (manifest.ts) 定义模块元数据
- 路由系统自动发现已注册模块
  **当前状态**: 模块注册中心已实现，但热插拔机制尚未完全实现 (YOLO模块直接在App.tsx中注册)
  **目标**: 实现完全的模块热插拔，支持动态加载/卸载模块

#### 3. 接口标准化约束

**约束**: 前后端接口必须遵循统一标准
**前端 API**:

- 函数命名: `camelCase`
- 返回类型: `Promise<Result<T>>`
- 错误处理: 统一的错误提示
  **后端命令**:
- 命令命名: `{module}_{action}` (snake_case)
- 返回类型: `Result<T, String>`
- 序列化: 使用 Serde derive

#### 4. 状态管理约束

**约束**: 状态变更必须通过 store 方法
**禁止**: 直接修改 store 状态对象
**要求**: 使用 store 提供的 action 方法
**理由**: 确保状态变更可追踪和调试

#### 5. 文件组织约束

**前端文件命名**:

- 组件: `PascalCase.tsx`
- 工具函数: `camelCase.ts`
- 样式: `kebab-case.css`
- API 文件: `功能名.ts`

**后端文件命名**:

- 模块入口: `mod.rs`
- 命令文件: `{功能}_commands.rs`
- 服务文件: `{功能}_service.rs`
- 模型文件: `{类型}.rs`

### 开发流程约束

#### 1. 文档驱动开发

**要求**: 任何新功能开发前必须创建文档
**文档位置**: `doc/进行中/{序号}-{功能名}-{日期}.md`
**文档内容**:

- 功能描述
- 任务分解
- 进度跟踪
- 相关文件列表

#### 2. 模块开发流程

**步骤**:

1. 在 `doc/进行中/` 创建功能文档
2. 定义模块清单 (manifest.ts)
3. 实现模块页面和组件
4. 实现后端命令和服务
5. 在模块入口注册模块 (未来: 通过注册中心自动注册)
6. 更新路由配置
7. 测试模块功能
8. 更新文档状态为"已完成"

#### 3. 代码审查要求

**触发条件**: 所有代码修改后必须进行审查
**审查内容**:

- 遵循架构约束
- 代码质量检查
- 安全漏洞检查
- 性能优化建议
  **工具**: 使用 code-reviewer agent

#### 4. 测试要求

**单元测试**: 每个模块的核心功能必须有单元测试
**集成测试**: API 调用和状态管理需要集成测试
**E2E 测试**: 关键用户流程需要端到端测试
**覆盖率**: 最低 80% 测试覆盖率

### 扩展规划约束

#### 未来模块扩展

**已规划模块**:

- **crawler**: 爬虫管理模块
- **rpa**: RPA 自动化模块
- **taskflow**: 任务流模块

**扩展原则**:

- 每个新模块遵循相同目录结构
- 复用核心基础设施
- 通过注册中心自动集成
- 保持前后端接口一致性

#### 技术栈扩展约束

**允许扩展**:

- 前端: 新 UI 组件库、状态管理库
- 后端: 新 Rust crate、性能优化库
  **约束**:
- 必须经过技术评估
- 不能破坏现有架构
- 需要更新相关文档

### 质量保证约束

#### 1. 错误处理约束

**前端**: 所有 API 调用必须有错误处理
**后端**: 所有命令返回 `Result<T, String>`
**用户体验**: 错误信息要用户友好，不能暴露内部实现

#### 2. 性能约束

**启动时间**: 应用启动时间 < 3秒
**内存使用**: 正常使用 < 500MB
**响应时间**: UI 操作响应 < 100ms
**监控**: 定期检查性能指标

#### 3. 安全约束

**输入验证**: 所有用户输入必须验证
**文件操作**: 路径遍历防护
**敏感数据**: 不在日志中记录敏感信息
**依赖安全**: 定期更新依赖，检查安全漏洞

---

## 当前架构状态评估

### 已完成部分 (40%)

- ✅ 核心基础设施 (core/) 架构设计
- ✅ 模块注册中心 (ModuleRegistry) 实现
- ✅ 全局状态管理 (Zustand stores)
- ✅ 前端 API 层设计
- ✅ 后端项目管理命令实现
- ✅ 自定义标题栏功能
- ✅ 基础 UI 布局组件

### 进行中部分 (40%)

- 🔄 YOLO 模块页面实现 (AnnotationPage, TrainingPage 等)
- 🔄 后端训练/推理命令开发
- 🔄 模块热插拔机制完善
- 🔄 文档管理系统

### 待开发部分 (20%)

- ❌ 后端训练服务和推理服务
- ❌ 完整的模块热插拔支持
- ❌ 爬虫/RPA模块框架
- ❌ 单元测试和集成测试
- ❌ E2E 测试套件

### 架构演进路线

1. **Phase 1**: 完善 YOLO 模块功能 (当前重点)
2. **Phase 2**: 实现完全模块热插拔
3. **Phase 3**: 添加新模块 (crawler, rpa)
4. **Phase 4**: 测试和质量保证完善

---

## 文档

- [CLAUDE.md](./CLAUDE.md) - 开发规范与目录结构
- [SPEC.md](./SPEC.md) - 详细功能规格与 UI 规格

## 许可证

MIT License
