# 纯 Rust YOLO 推理系统 - 依赖配置指南

## 技术栈

| 组件 | Crate | 用途 | GPU 支持 |
|------|-------|------|---------|
| 屏幕捕获 | scrap | 高性能零拷贝捕获 | N/A |
| 推理引擎 | burn | Rust 原生深度学习 | ✅ WebGPU |
| 模型转换 | tch-rs | PyTorch 绑定 | ✅ CUDA |

## 添加的依赖

```toml
[dependencies]
# 屏幕捕获 - scrap (零拷贝)
scrap = "0.2"

# 推理引擎 - burn (Rust 原生)
burn = "0.5"
burn-ndarray = "0.5"      # CPU 后端
burn-wgpu = "0.5"          # GPU 后端 (WebGPU)
burn-cudarc = "0.5"        # NVIDIA CUDA 支持

# 模型转换 - tch-rs (PyTorch)
tch = "0.5"

# 图像处理
image = "0.25"
ndarray = "0.16"

# 异步运行时
tokio = { version = "1", features = ["sync", "time", "rt-multi-thread"] }

# 其他
parking_lot = "0.12"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## 完整 Cargo.toml 配置

```toml
[package]
name = "MyRustTools"
version = "1.0.0"
description = "Rust编写的一站式工具集"
authors = ["MyRustTools"]
license = "MIT"
edition = "2021"
rust-version = "1.77.2"

[lib]
name = "app_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2.5.6", features = [] }

[dependencies]
# 基础依赖
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tauri = { version = "2.10.3", features = ["protocol-asset", "devtools"] }
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
tauri-plugin-shell = "2"

# 屏幕捕获 - scrap (零拷贝)
scrap = "0.2"
# xcap = "0.1"  # 备用

# 推理引擎 - burn (Rust 原生深度学习)
burn = "0.5"
burn-ndarray = "0.5"      # ndarray CPU 后端
# burn-wgpu = "0.5"      # WebGPU GPU 后端 (需要单独安装)
# burn-cudarc = "0.5"    # CUDA GPU 后端 (需要 CUDA)

# 模型转换 - tch-rs (PyTorch Rust 绑定)
tch = "0.5"

# 图像处理
image = "0.25"
ndarray = "0.16"

# ONNX Runtime (备用推理引擎)
# ort = "2.0.0-rc.1"

# Tract (备用 CPU 推理)
# tract-onnx = "0.22.0"

# 异步运行时
tokio = { version = "1", features = ["sync", "time", "process", "io-util", "macros", "rt-multi-thread"] }
tokio-stream = "0.1"

# 其他工具
rand = "0.9"
hex = "0.4"
chrono = "0.4"
reqwest = { version = "0.12", features = ["json", "stream"] }
once_cell = "1.19"
dirs = "5.0"
serde_yaml = "0.9"
base64 = "0.22"
notify = "8"
webp = "0.3"
parking_lot = "0.12"
rayon = "1.8"
lru-cache = "0.1"

# 性能监控
# perf-monitor = "0.1"
```

## 系统要求

### 对于 burn + WebGPU

```bash
# 安装 Vulkan SDK (Windows)
winget install KhronosGroup.VulkanSDK

# 或使用 chocolatey
choco install vulkan-sdk
```

### 对于 burn + CUDA

```bash
# 安装 CUDA Toolkit 11.8 或 12.1
# 下载地址: https://developer.nvidia.com/cuda-downloads

# 设置环境变量
set LIBTORCH=C:\path\to\libtorch
set PATH=%PATH%;C:\path\to\libtorch\bin
```

### 对于 tch-rs

```bash
# tch-rs 会自动下载 libtorch (~200MB)
# 或者手动下载并设置 LIBTORCH 环境变量
```

## 构建步骤

### 1. 更新依赖

```bash
cd src-tauri
cargo update
```

### 2. 构建（CPU 模式）

```bash
cargo build --release
```

### 3. 构建（GPU 模式）

```bash
# 启用 WebGPU feature
cargo build --release --features burn-wgpu

# 或启用 CUDA feature
cargo build --release --features burn-cudarc
```

### 4. 运行

```bash
npm run tauri dev
```

## 性能预期

| 配置 | 推理时间 | FPS | 设备 |
|------|---------|-----|------|
| burn-ndarray | 500-2000ms | 1-5 | CPU |
| burn-wgpu | 50-200ms | 15-30 | GPU (WebGPU) |
| burn-cudarc | 20-100ms | 30-60 | GPU (CUDA) |
| tch (CPU) | 200-1000ms | 2-10 | CPU |
| tch (CUDA) | 20-100ms | 30-60 | GPU (CUDA) |

## 模型转换

### 使用 tch-rs 转换 PyTorch → ONNX

```rust
use rust_native_yolo::ModelConverter;

let converter = ModelConverter::new()?;

// PyTorch → ONNX
converter.pytorch_to_onnx(
    Path::new("best.pt"),
    Path::new("best.onnx"),
    640,
)?;

// 优化 ONNX
converter.optimize_onnx(
    Path::new("best.onnx"),
    Path::new("best_optimized.onnx"),
)?;
```

### 使用 Python 脚本（推荐）

```bash
# 安装依赖
pip install torch onnx onnxruntime onnxsim onnxoptimizer

# 转换
python scripts/convert_yolo.py \
    --model best.pt \
    --output best.onnx \
    --img-size 640 \
    --batch-size 1
```

## 故障排除

### 问题 1：burn 编译失败

**错误：**
```
error: failed to run custom build command for `bindgen`
```

**解决：**
```bash
# 安装 LLVM/Clang
winget install LLVM.LLVM

# 或使用 chocolatey
choco install llvm
```

### 问题 2：scrap 找不到显示器

**错误：**
```
Failed to get displays
```

**解决：**
- 确保在 Windows 上运行
- 检查 DPI 设置
- 尝试以管理员身份运行

### 问题 3：tch 找不到 libtorch

**错误：**
```
failed to find libtorch
```

**解决：**
```bash
# 下载 libtorch
wget https://download.pytorch.org/libtorch/cpu/libtorch-cxx11-abi-shared-with-deps-2.1.0%2Bcpu.zip

# 设置环境变量
set LIBTORCH=D:\path\to\libtorch
```

### 问题 4：GPU 后端不可用

**检查：**
```bash
# WebGPU
vulkaninfo

# CUDA
nvidia-smi
```

**解决：**
- 启用对应的 feature
- 安装所需的 SDK
- 或回退到 CPU 模式

## 进一步优化

### 1. 模型量化

```rust
// INT8 量化（需要校准数据集）
converter.quantize_model(
    Path::new("best.onnx"),
    Path::new("best_int8.onnx"),
)?;
```

### 2. 动态输入尺寸

```rust
// 根据目标 FPS 调整
let input_size = match target_fps {
    60 => 320,
    30 => 480,
    15 => 640,
    _ => 640,
};
```

### 3. 异步 Pipeline

```rust
// 捕获和推理并行
tokio::join! {
    capture_loop(),
    inference_loop(),
};
```

## 总结

这个配置提供了：
- ✅ 完全纯 Rust 实现
- ✅ 零拷贝屏幕捕获（scrap）
- ✅ GPU 加速推理（burn/tch）
- ✅ 模型转换工具（tch）
- ✅ 30+ FPS 目标性能

下一步是更新 Cargo.toml 并测试！
