# 纯 Rust YOLO 推理系统 - 实施指南

## 完成的工作

我已经创建了一个完全基于 Rust 的高性能 YOLO 推理系统：

### 1. 核心模块 - `rust_native_yolo.rs`

**技术栈：**
- **scrap**: 高性能零拷贝屏幕捕获
- **burn**: Rust 原生深度学习框架
- **tch-rs**: PyTorch 模型转换

**特性：**
- 零拷贝屏幕捕获（scrap）
- GPU 加速支持（burn WebGPU/CUDA）
- 模型转换工具（tch）
- 详细的性能监控

### 2. 依赖配置 - `Cargo.toml`

**已添加：**
```toml
scrap = "0.2"              # 零拷贝屏幕捕获
burn = "0.5"               # 推理引擎
burn-ndarray = "0.5"       # CPU 后端
# burn-wgpu = "0.5"       # GPU 后端（需要 Vulkan）
# burn-cudarc = "0.5"     # CUDA 后端（需要 CUDA）
tch = "0.5"               # 模型转换
```

**已移除：**
- tract-onnx (暂时)
- xcap (改为 scrap)

### 3. 配置文档 - `RUST_NATIVE_SETUP.md`

包含详细的：
- 依赖说明
- 系统要求
- 构建步骤
- 性能预期
- 故障排除

## 下一步操作

### 步骤 1：安装系统依赖

#### 对于 WebGPU (burn)
```powershell
# 安装 Vulkan SDK
winget install KhronosGroup.VulkanSDK
```

#### 对于 CUDA (burn + tch)
```powershell
# 安装 CUDA Toolkit 11.8 或 12.1
# 下载地址: https://developer.nvidia.com/cuda-downloads
```

#### 对于 tch (模型转换)
```powershell
# tch 会自动下载 libtorch (~200MB)
# 或者手动下载：
wget https://download.pytorch.org/libtorch/cpu/libtorch-cxx11-abi-shared-with-deps-2.1.0%2Bcpu.zip
```

### 步骤 2：更新依赖

```bash
cd src-tauri
cargo update
```

### 步骤 3：构建（CPU 模式 - 最简单）

```bash
cargo build --release
```

如果成功，尝试运行：
```bash
npm run tauri dev
```

### 步骤 4：构建（GPU 模式 - 性能最佳）

#### 启用 WebGPU
```bash
# 在 Cargo.toml 中取消注释
# burn-wgpu = "0.5"

cargo build --release
```

#### 启用 CUDA
```bash
# 在 Cargo.toml 中取消注释
# burn-cudarc = "0.5"

# 设置环境变量
$env:LIBTORCH = "C:\path\to\libtorch"

cargo build --release
```

## 预期的性能

| 配置 | 推理时间 | FPS | 说明 |
|------|---------|-----|------|
| burn-ndarray (CPU) | 500-2000ms | 1-5 | 最简单，立即可用 |
| burn-wgpu (GPU) | 50-200ms | 15-30 | 需要 Vulkan |
| burn-cudarc (GPU) | 20-100ms | 30-60 | 需要 CUDA |
| tch (CPU) | 200-1000ms | 2-10 | 模型转换用 |
| tch (CUDA) | 20-100ms | 30-60 | 模型转换+推理 |

## 代码使用示例

### 初始化捕获服务

```rust
use rust_native_yolo::{ScrapCaptureService, CaptureConfig};

let config = CaptureConfig {
    target_fps: 30,
    input_size: 640,
    inference_interval: 1,
    confidence_threshold: 0.65,
    nms_threshold: 0.45,
};

let mut service = ScrapCaptureService::new(config);

// 初始化推理引擎
service.init_engine(
    Path::new("best.onnx"),
    4,  // 4 classes
    vec![
        "elephant".to_string(),
        "zebra".to_string(),
        "buffalo".to_string(),
        "rhino".to_string(),
    ],
)?;

// 启动捕获
service.start(0).await?;
```

### 模型转换

```rust
use rust_native_yolo::ModelConverter;

let converter = ModelConverter::new()?;

// PyTorch → ONNX
converter.pytorch_to_onnx(
    Path::new("best.pt"),
    Path::new("best.onnx"),
    640,
)?;
```

### 性能监控

运行后观察日志：
```
[BurnEngine] Backend: wgpu, Device: webgpu
[ScrapCapture] Scrap capture: 2560x1600, Target FPS: 30
[PERF-Scrap] FPS: 28.5, Capture: 12.3ms, Inference: 45.2ms, Total: 57.5ms, Detections: 3
```

## 故障排除

### 问题 1：编译失败 - burn 依赖

**错误：**
```
error: failed to run custom build command for `bindgen`
```

**解决：**
```bash
# 安装 LLVM/Clang
winget install LLVM.LLVM
```

### 问题 2：找不到 scrap 库

**错误：**
```
cannot find crate `scrap`
```

**解决：**
```bash
# 确保在 src-tauri 目录
cd src-tauri
cargo update
cargo build
```

### 问题 3：GPU 不可用

**检查：**
```bash
# WebGPU
vulkaninfo | findstr Vulkan

# CUDA
nvidia-smi
```

**解决：**
- 先用 CPU 模式测试
- 安装对应的 SDK
- 或继续使用当前 tract-onnx 方案

### 问题 4：tch 找不到 libtorch

**错误：**
```
failed to find libtorch at ...
```

**解决：**
```bash
# 下载 libtorch
wget https://download.pytorch.org/libtorch/cpu/libtorch-cxx11-abi-shared-with-deps-2.1.0%2Bcpu.zip

# 解压
Expand-Archive libtorch.zip -DestinationPath C:\libtorch

# 设置环境变量
$env:LIBTORCH = "C:\libtorch\libtorch"
```

## 当前状态

✅ **已完成：**
- 创建 `rust_native_yolo.rs` - 完整的推理系统
- 更新 `Cargo.toml` - 添加 scrap, burn, tch 依赖
- 创建 `RUST_NATIVE_SETUP.md` - 配置文档

⏳ **待测试：**
- 编译是否成功
- 性能是否达标
- 是否有错误

## 建议的实施顺序

### 方案 A：立即测试（CPU 模式）

1. 运行 `cargo build --release`
2. 如果成功，运行 `npm run tauri dev`
3. 观察性能日志
4. 如果失败，看错误信息解决

### 方案 B：等待 GPU 支持

1. 安装 Vulkan SDK (WebGPU) 或 CUDA Toolkit
2. 在 Cargo.toml 中启用对应的 feature
3. 重新构建
4. 测试 GPU 性能

### 方案 C：混合方案

1. 保留当前的 tract-onnx 作为 fallback
2. 添加新的 rust_native_yolo 作为主方案
3. 根据硬件自动选择

## 性能对比

| 方案 | FPS | 难度 | 推荐度 |
|------|-----|------|--------|
| 当前 (tract CPU) | 1 | 低 | ⚠️ 太慢 |
| + scrap | 2-3 | 低 | ✅ 可用 |
| burn (CPU) | 1-5 | 中 | ⚠️ 差不多 |
| burn (WebGPU) | 15-30 | 中 | ✅✅ 推荐 |
| burn (CUDA) | 30-60 | 高 | ✅✅ 最佳 |
| ONNX Runtime | 30+ | 低 | ✅✅ 最稳定 |

## 需要你做的

1. **尝试编译：**
   ```bash
   cd src-tauri
   cargo build --release
   ```

2. **如果失败：** 告诉我错误信息

3. **如果成功：** 测试性能并告诉我结果

4. **决定是否安装 GPU 支持：**
   - Vulkan SDK (WebGPU)
   - CUDA Toolkit (CUDA)

你想先尝试哪个？编译应该很快就能知道结果！
