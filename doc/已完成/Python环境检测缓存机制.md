# Python 环境检测缓存机制实现

## 功能概述

实现了 Python 环境检测结果的缓存机制，避免每次进入首页时重复检测环境，提升用户体验和性能。

## 实现方案

### 后端实现 (src-tauri/src/modules/yolo/commands/env.rs)

#### 1. 缓存存储
- 使用 `once_cell::sync::Lazy` 创建线程安全的静态变量 `PYTHON_ENV_CACHE`
- 使用 `std::sync::Mutex` 保护并发访问
- 缓存类型：`Option<PythonEnvInfo>`

#### 2. 命令接口

```rust
// 1. 带缓存的环境检测
#[tauri::command]
pub async fn check_python_env(force_refresh: bool) -> Result<EnvCheckResponse, String>

// 2. 仅获取缓存（不执行检测）
#[tauri::command]
pub fn get_cached_python_env() -> EnvCheckResponse

// 3. 清除缓存
#[tauri::command]
pub fn clear_python_env_cache() -> Result<EnvCheckResponse, String>
```

#### 3. 缓存逻辑

```rust
pub async fn check_python_env(force_refresh: bool) -> Result<EnvCheckResponse, String> {
    // 1. 如果不强制刷新，先检查缓存
    if !force_refresh {
        if let Some(cached) = *cache.lock().unwrap() {
            return Ok(EnvCheckResponse {
                success: true,
                data: Some(cached),
                from_cache: true,  // 标记数据来源
            });
        }
    }

    // 2. 执行实际的环境检测
    let env_info = perform_env_check();

    // 3. 更新缓存
    *cache.lock().unwrap() = Some(env_info.clone());

    Ok(EnvCheckResponse {
        success: true,
        data: Some(env_info),
        from_cache: false,  // 新检测的数据
    })
}
```

### 前端实现 (src/core/api/training.ts)

#### API 函数

```typescript
// 1. 检测环境（支持缓存）
export async function checkPythonEnv(forceRefresh: boolean = false): Promise<ApiResponse<PythonEnvInfo>>

// 2. 仅获取缓存
export async function getCachedPythonEnv(): Promise<ApiResponse<PythonEnvInfo>>

// 3. 清除缓存
export async function clearPythonEnvCache(): Promise<ApiResponse<void>>
```

### 前端组件 (src/modules/yolo/components/PythonEnvCheck.tsx)

#### 加载逻辑

```typescript
useEffect(() => {
  loadInstructions();
  loadEnvironmentInfo();  // 首次加载
}, []);

// 加载环境信息
const loadEnvironmentInfo = async () => {
  setLoading(true);

  // 1. 尝试获取缓存
  const cached = await getCachedPythonEnv();

  if (cached.success && cached.data) {
    // 有缓存，直接使用
    setEnvInfo(cached.data);
    setFromCache(true);
    setLoading(false);
    return;
  }

  // 2. 没有缓存，执行检测
  await checkEnvironment();
};
```

#### 用户交互流程

1. **首次进入页面**
   - 尝试获取缓存的环境信息
   - 如果有缓存 → 显示缓存数据（无需重新检测）
   - 如果没有缓存 → 执行检测

2. **点击"重新检测"按钮**
   - 调用 `checkEnvironment(true)` 强制刷新
   - 显示加载状态
   - 执行检测并更新缓存
   - 更新UI显示最新结果

3. **安装依赖后**
   - 自动调用 `checkEnvironment(true)` 强制刷新
   - 验证安装结果
   - 更新UI

## 优势

### 1. 性能优化
- **减少系统调用**：避免每次进入页面都执行 Python 子进程调用
- **提升响应速度**：缓存命中时几乎即时返回结果
- **降低资源消耗**：减少不必要的 Python 环境检测开销

### 2. 用户体验
- **即时加载**：有缓存时立即显示环境状态，无需等待
- **清晰的反馈**：区分缓存数据和新检测数据
- **灵活的控制**：用户可以随时手动刷新

### 3. 状态管理
- **缓存自动更新**：安装依赖后自动刷新缓存
- **数据一致性**：强制刷新确保获取最新状态
- **错误恢复**：检测失败时优雅降级

## 使用场景

### 场景1：应用启动
```
用户打开应用
  ↓
检测缓存 → 有缓存 → 直接显示环境状态 ✓
  ↓（无缓存）
执行检测 → 更新缓存 → 显示检测结果
```

### 场景2：手动刷新
```
用户点击"重新检测"
  ↓
显示加载状态
  ↓
调用 checkPythonEnv(forceRefresh=true)
  ↓
执行检测 → 更新缓存 → 显示结果
```

### 场景3：安装依赖
```
用户点击"立即安装"
  ↓
安装 PyTorch、Ultralytics 等
  ↓
自动触发 checkPythonEnv(forceRefresh=true)
  ↓
验证安装 → 更新UI → 提示成功/失败
```

## 缓存策略

### 缓存内容
```typescript
interface PythonEnvInfo {
  python_exists: boolean;        // Python 是否安装
  python_version: string | null; // Python 版本
  torch_exists: boolean;          // PyTorch 是否安装
  torch_version: string | null;   // PyTorch 版本
  torchaudio_exists: boolean;     // torchaudio 是否安装
  cuda_available: boolean;       // CUDA 是否可用
  cuda_version: string | null;   // CUDA 版本
  ultralytics_exists: boolean;   // Ultralytics 是否安装
  ultralytics_version: string | null; // Ultralytics 版本
  yolo_command_exists: boolean;  // YOLO CLI 是否可用
}
```

### 缓存生命周期
- **创建**：首次检测或缓存不存在时
- **更新**：手动刷新、安装依赖后
- **读取**：每次进入页面时
- **清除**：可通过 `clearPythonEnvCache()` 手动清除

## 注意事项

### 1. 缓存失效时机
- 用户手动点击"重新检测"
- 安装或卸载 Python 包后
- 应用重启（静态变量会被清空）

### 2. 线程安全
- 后端使用 `Mutex` 保护缓存变量
- 支持多线程并发访问
- Rust 静态变量生命周期管理

### 3. 错误处理
- 检测失败时返回默认值（所有项为 false）
- API 调用失败时有友好的错误日志
- UI 层优雅降级处理

## 测试建议

### 1. 首次加载测试
- 清空缓存
- 进入首页
- 验证：显示加载状态 → 执行检测 → 显示结果

### 2. 缓存命中测试
- 完成首次检测
- 刷新页面
- 验证：无加载状态 → 立即显示缓存数据 → 显示"已缓存"标记

### 3. 强制刷新测试
- 完成首次检测
- 点击"重新检测"
- 验证：显示加载状态 → 执行检测 → 更新UI

### 4. 安装后测试
- 环境未就绪
- 点击"立即安装"
- 验证：安装完成后 → 自动检测 → 更新状态显示

## 后续优化方向

### 1. 持久化缓存
- 将缓存写入本地文件
- 应用重启后恢复缓存
- 减少冷启动时的检测开销

### 2. 缓存过期机制
- 添加 TTL（Time-To-Live）
- 定期自动刷新
- 平衡实时性和性能

### 3. 增量检测
- 仅检测变化的项
- 缓存未变化的检测结果
- 进一步提升性能

### 4. 检测进度
- 对长时间检测显示进度条
- 后端流式返回检测状态
- 改善用户体验

## 相关文件

- `src-tauri/src/modules/yolo/commands/env.rs` - 后端命令实现
- `src-tauri/src/modules/yolo/commands/mod.rs` - 命令导出
- `src-tauri/Cargo.toml` - 依赖管理（tokio）
- `src/core/api/training.ts` - 前端 API 接口
- `src/core/api/index.ts` - API 导出
- `src/modules/yolo/components/PythonEnvCheck.tsx` - 环境检测组件
