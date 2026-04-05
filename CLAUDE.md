# MyRustTools 开发规范

## 开发语言

**所有开发对话必须使用中文**

---

## 源代码目录结构

### 目录规范

#### 前端目录 (src/)

```
src/
├── main.tsx                          # 应用入口
├── App.tsx                           # 根组件
│
├── core/                             # 【核心基础设施】- 模块不可修改
│   ├── api/
│   │   └── api.ts                   # 全局 API 调用
│   ├── components/
│   │   └── layout/                  # 布局组件
│   │       ├── TitleBar.tsx
│   │       ├── ActivityBar.tsx
│   │       ├── Sidebar.tsx
│   │       ├── StatusBar.tsx
│   │       └── AppShell.tsx
│   ├── stores/                       # 全局状态
│   │   ├── routerStore.ts
│   │   ├── workspaceStore.ts
│   │   └── settingsStore.ts
│   └── styles/
│       ├── index.css                # 全局样式
│       └── hub.css                  # Hub 页面样式
│
├── modules/                          # 【模块系统】
│   ├── types.ts                      # 模块类型定义
│   ├── registry.ts                   # 模块注册中心
│   │
│   ├── hub/                          # Hub 首页模块
│   │   └── HubPage.tsx
│   │
│   ├── yolo/                         # YOLO 检测模块
│   │   ├── index.ts                  # 模块入口（注册）
│   │   ├── manifest.ts               # 模块清单
│   │   ├── pages/
│   │   │   ├── AnnotationPage.tsx
│   │   │   ├── TrainingPage.tsx
│   │   │   ├── ResultsPage.tsx
│   │   │   ├── VideoPage.tsx
│   │   │   └── DevicePage.tsx
│   │   └── components/
│   │       ├── TrainingPanel.tsx
│   │       ├── ModelConvertModal.tsx
│   │       └── ...
│   │
│   ├── crawler/                      # 【未来】爬虫管理模块
│   └── rpa/                          # 【未来】RPA 模块
│
└── shared/                           # 【共享组件】
    └── components/
        ├── ui/
        │   ├── Button.tsx
        │   ├── Modal.tsx
        │   └── HelpModal.tsx
        └── workspace/
            ├── HomePage.tsx
            └── NewProjectModal.tsx
```

#### 后端目录 (src-tauri/src/)

```
src-tauri/src/
├── main.rs                           # 应用入口
├── lib.rs                            # 库入口，模块导出
│
├── core/                             # 【核心基础设施】
│   ├── commands/                     # Tauri 命令
│   │   ├── mod.rs
│   │   ├── file_commands.rs         # 文件操作命令
│   │   └── system_commands.rs       # 系统命令
│   ├── models/                       # 数据模型
│   │   ├── mod.rs
│   │   ├── error.rs                 # 错误类型
│   │   └── response.rs              # API 响应格式
│   └── services/                     # 公共服务
│       ├── mod.rs
│       └── logger.rs                 # 日志服务
│
├── modules/                          # 【模块系统】
│   │
│   ├── yolo/                         # YOLO 检测模块
│   │   ├── mod.rs                    # 模块入口
│   │   ├── commands/                 # 模块命令
│   │   │   ├── mod.rs
│   │   │   ├── train.rs             # 训练命令
│   │   │   ├── detect.rs            # 推理命令
│   │   │   └── export.rs             # 导出命令
│   │   ├── services/                 # 模块服务
│   │   │   ├── mod.rs
│   │   │   ├── trainer.rs            # 训练服务
│   │   │   └── detector.rs           # 检测服务
│   │   └── models/                    # 模块模型
│   │       ├── mod.rs
│   │       └── config.rs             # YOLO 配置
│   │
│   ├── crawler/                      # 【未来】爬虫管理模块
│   └── rpa/                          # 【未来】RPA 模块
│
└── shared/                           # 【共享模块】
    └── utils/
        ├── mod.rs
        └── path.rs                    # 路径工具
```

### 目录原则

| 目录       | 原则                       |
| ---------- | -------------------------- |
| `core/`    | 核心代码，模块只读不可修改 |
| `modules/` | 每个模块独立文件夹，自包含 |
| `shared/`  | 多模块共用的 UI 组件       |

#### 后端目录原则

| 目录       | 原则                                          |
| ---------- | --------------------------------------------- |
| `core/`    | 基础设施，命令、模型、服务                    |
| `modules/` | 每个模块独立，自包含 commands/services/models |
| `shared/`  | 跨模块共享的工具函数                          |

### 文件命名规范

- 组件文件：`PascalCase.tsx`（如 `TitleBar.tsx`）
- 工具/工具函数：`camelCase.ts`（如 `api.ts`）
- 模块清单：`manifest.ts`
- 样式文件：`kebab-case.css`

#### Rust 文件命名

- 模块文件：`mod.rs`（模块入口）或 `模块名.rs`
- 命令文件：`xxx_commands.rs`
- 服务文件：`xxx_service.rs`
- 模型文件：`xxx.rs`（如 `config.rs`）

### 新增文件规则

1. **新模块开发**
   - 在 `modules/` 下创建模块文件夹
   - 模块内页面放入 `pages/`
   - 模块内共享组件放入 `components/`
   - 在模块 `index.ts` 中注册

2. **共享组件开发**
   - 放入 `shared/components/` 对应分类
   - 如组件仅被一个模块使用，应放入该模块的 `components/` 下

3. **核心修改**
   - `core/` 目录代码不可随意修改
   - 如需修改核心功能，先讨论

#### 后端新增规则

1. **新模块开发**
   - 在 `modules/` 下创建模块文件夹
   - 模块内命令放入 `commands/`
   - 模块内服务放入 `services/`
   - 模块内模型放入 `models/`
   - 在模块 `mod.rs` 中导出所有子模块

2. **新增命令**
   - 放入对应模块的 `commands/` 目录
   - 使用 `#[tauri::command]` 标记
   - 返回 `Result<T, String>` 或使用自定义错误类型

3. **公共服务**
   - 放入 `core/services/`
   - 如被多个模块使用，标记为 `pub(crate)`

---

## 文档管理

### 目录结构

```
doc/
├── 0-index.md              # 模块清单索引（追踪所有模块状态）
├── 进行中/                  # 正在开发的功能
│   └── {序号}-{功能名}-{日期}.md
├── 已完成/
│   ├── 已完成项目/          # 已完成但不再活跃维护
│   └── 正在维护/            # 已完成且正在维护
└── 归档/                   # 成熟稳定，不再需要维护
```

### 文档命名规范

```bash
{序号}-{功能名}-{日期}.md
# 示例: 01-模块化架构设计-20260405.md
```

### 索引文件 (0-index.md)

```markdown
# 模块清单

| 模块         | 状态     | 负责人 | 最后更新   |
| ------------ | -------- | ------ | ---------- |
| 自定义标题栏 | 正在维护 | Claude | 2026-04-05 |
| 模块化架构   | 进行中   | Claude | 2026-04-05 |
```

### 文档更新规则

| 时机             | 操作                           |
| ---------------- | ------------------------------ |
| 开始开发前       | 阅读 `doc/进行中/` 相关文档    |
| 完成一个子任务   | 更新进度百分比                 |
| 功能正式完成     | 将文档移动到 `已完成/正在维护` |
| 功能稳定不再维护 | 将文档移动到 `归档`            |

---

## 开发流程

1. **开始新功能前**
   - 阅读 `doc/进行中/` 下相关文档
   - 确认当前进度和待办事项

2. **开发过程中**
   - 每完成一个功能点，更新文档进度
   - 使用 `[x]` 标记已完成项

3. **功能完成时**
   - 更新文档状态为"已完成"
   - 移动文档到 `已完成/正在维护`
   - **必须由用户检查确认后才能视为完成**
   - 未经过用户检查的功能不得标记为完成

---

## 示例对话

```
用户: 我们来实现模块化架构吧

助手: 好的，我先查看一下当前进行中的文档。
      发现相关文档: 01-模块化架构设计-20260405.md
      当前进度: 30%
      待完成:
      - [ ] 创建模块类型定义
      - [x] 设计架构方案

      开始实现模块类型定义...
```

---

## 文档模板

### 进行中文档模板

```markdown
# {功能名称}

## 基本信息

- 开始日期: {YYYY-MM-DD}
- 预计完成: {YYYY-MM-DD}
- 状态: 进行中
- 进度: {X%}

## 功能描述

{简要说明}

## 任务分解

### Phase 1: {阶段名}

- [ ] 子任务1
- [ ] 子任务2

### Phase 2: {阶段名}

- [ ] 子任务3

## 当前进度

{具体进度说明}

## 备注

{待解决的问题等}
```

### 已完成文档模板

```markdown
# {功能名称}

## 基本信息

- 完成日期: {YYYY-MM-DD}
- 状态: 正在维护 | 已归档
- 版本: {v1.0.0}

## 功能描述

{已完成的功能说明}

## 核心实现

{技术要点简述}

## 相关文件

- `src/xxx.ts`
- `src/yyy.tsx`

## 更新日志

| 日期       | 版本   | 更新内容 |
| ---------- | ------ | -------- |
| YYYY-MM-DD | v1.0.0 | 初始完成 |
```

---

## 规则一致性

- 所有开发对话使用中文
- 每次开发前先阅读进行中文档
- 每完成一个功能点立即更新文档
- 功能完成后及时移动文档位置
- 保持索引文件 (0-index.md) 最新状态
