# 前端 API 层整理总结

## 整理日期
2026-04-11

## 整理目标
确保所有调用后端功能的地方都在 `src/core/api/` 文件夹中定义，遵循架构约束。

## 完成的修改

### 1. TrainingPage.tsx 整理
**文件**: `src/modules/yolo/pages/TrainingPage.tsx`

**修改内容**:
- ✅ 移除了直接 `import { invoke } from '@tauri-apps/api/core'`
- ✅ 添加了从 `@/core/api` 导入 `checkPythonEnv` 和 `selectFile`
- ✅ 将文件对话框调用从 `invoke('open_file_dialog', ...)` 改为使用 `selectFile` API
- ✅ 将环境检测调用从 `invoke('check_python_env', ...)` 改为使用 `checkPythonEnv` API

**修改前**:
```typescript
import { invoke } from '@tauri-apps/api/core';
// ...
const result = await invoke<DialogResult>('open_file_dialog', {
  title: '选择模型文件',
  filters: [{ name: 'YOLO Model', extensions: ['pt', 'pth'] }],
});
```

**修改后**:
```typescript
import { selectFile } from '@/core/api';
// ...
const result = await selectFile('选择模型文件', [{ name: 'YOLO Model', extensions: ['pt', 'pth'] }]);
```

### 2. PythonEnvCheck.tsx 整理
**文件**: `src/modules/yolo/components/PythonEnvCheck.tsx`

**修改内容**:
- ✅ 移除了直接 `import { invoke } from '@tauri-apps/api/core'`
- ✅ 移除了重复的接口定义（`PythonEnvInfo` 和 `InstallInstructions`）
- ✅ 添加了从 `@/core/api/training` 导入所有需要的函数和类型
- ✅ 将所有 `invoke` 调用改为使用 API 层函数：
  - `check_python_env` → `checkPythonEnv()`
  - `get_install_instructions` → `getInstallInstructions()`
  - `install_python_deps` → `installPythonDeps()`

**修改前**:
```typescript
import { invoke } from '@tauri-apps/api/core';

const checkEnvironment = async () => {
  const result = await invoke<...>('check_python_env');
  // ...
};
```

**修改后**:
```typescript
import { checkPythonEnv } from '@/core/api';

const checkEnvironment = async () => {
  const result = await checkPythonEnv();
  // ...
};
```

### 3. training.ts API 增强
**文件**: `src/core/api/training.ts`

**修改内容**:
- ✅ 扩展了 `PythonEnvInfo` 接口，添加了缺失的字段：
  - `torchaudio_exists: boolean`
  - `cuda_available: boolean`
  - `cuda_version: string | null`
- ✅ 更新了 `installPythonDeps` 函数，添加了 `cpuOnly` 参数支持

### 4. API 层导出更新
**文件**: `src/core/api/index.ts`

**修改内容**:
- ✅ 确保所有类型和函数都正确导出

## 架构合规性检查

### ✅ 已解决的问题
1. **直接调用后端**: 所有组件现在都通过 API 层调用后端
2. **类型重复定义**: 移除了组件中的重复接口定义，统一使用 API 层导出的类型
3. **错误处理统一**: API 层统一处理错误，返回标准化的 `ApiResponse` 格式

### 📋 当前 API 层结构
```
src/core/api/
├── index.ts              # 统一导出
├── types.ts              # 共享类型定义
├── common.ts             # 通用功能 (对话框、版本检查等)
├── file.ts               # 文件操作
├── project.ts            # 项目管理
├── dataset.ts            # 数据集管理
├── annotation.ts         # 标注功能
├── training.ts          # 训练功能 (包含 Python 环境检查)
├── model.ts              # 模型管理
├── device.ts            # 设备管理
├── video.ts             # 视频处理
└── settings.ts           # 设置管理
```

## 验证结果

### 搜索直接调用
```bash
# 检查所有直接使用 invoke 的地方（排除 api 文件夹）
Get-ChildItem -Path src -Recurse -Include "*.tsx","*.ts" | 
  Where-Object { $_.FullName -notmatch "node_modules" -and $_.FullName -notmatch "api" } | 
  Select-String -Pattern "from '@tauri-apps/api/core'"
```

**结果**: ✅ 无匹配项（除 api 文件夹外）

### 搜索 import 语句
```bash
# 检查所有从 @tauri-apps/api/core 导入的地方
Get-ChildItem -Path src -Recurse -Include "*.tsx","*.ts" | 
  Select-String -Pattern "from '@tauri-apps/api/core'"
```

**结果**: ✅ 仅在 api 文件夹中存在（符合预期）

## 下一步建议

### 可选优化
1. **添加 ESLint 规则**: 创建自定义 ESLint 规则，禁止在非 api 文件夹中使用 `invoke`
2. **API 文档生成**: 为 API 层生成 TypeScript 文档注释
3. **单元测试**: 为关键 API 函数添加单元测试

### 架构扩展
1. **错误边界**: 在 API 层统一添加错误边界处理
2. **重试机制**: 为网络相关的 API 调用添加自动重试机制
3. **缓存层**: 为频繁调用的 API（如设备列表）添加缓存

## 总结

✅ **已完成**: 前端代码已完全整理，所有后端调用都通过 API 层进行
✅ **架构合规**: 符合项目的核心约束（核心代码只读原则、接口标准化约束）
✅ **类型安全**: 所有 API 都有完整的 TypeScript 类型定义
✅ **错误处理**: 统一的错误处理和用户提示

---
**状态**: ✅ 已完成
**完成度**: 100%
