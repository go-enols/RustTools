# 高性能推理依赖配置指南

## 推荐的依赖组合

### 方案 1：使用 ONNX Runtime（最稳定）

```toml
# Cargo.toml

[dependencies]
# ONNX Runtime - GPU 加速推理
ort = "2.0.0-rc.1"

# 高性能屏幕捕获
scrap = "0.2"

# 异步运行时
tokio = { version = "1", features = ["sync", "time", "rt-multi-thread"] }
```

**优点：**
- ONNX Runtime 支持 CUDA、DirectML、CPU
- 成熟的推理优化
- 易于使用

**缺点：**
- 需要预编译的 ONNX Runtime 库

### 方案 2：使用 tract-onnx（纯 Rust，CPU）

```toml
[dependencies]
# ONNX Runtime - 纯 Rust 实现
tract-onnx = "0.22.0"

# 屏幕捕获（当前使用）
xcap = "0.1"

# 异步运行时
tokio = { version = "1", features = ["sync", "time", "rt-multi-thread"] }
```

**优点：**
- 纯 Rust 实现，无外部依赖
- 易于部署

**缺点：**
- 仅 CPU 推理，速度较慢

### 方案 3：使用 burn（Rust 原生，GPU）

```toml
[dependencies]
# Burn - Rust 原生深度学习框架
burn = "0.5"
burn-ndarray = "0.5"  # ndarray 后端
# burn-wgpu = "0.5"  # WebGPU 后端（需要 feature）

# 屏幕捕获
scrap = "0.2"

# 异步运行时
tokio = { version = "1", features = ["sync", "time", "rt-multi-thread"] }
```

**优点：**
- 纯 Rust 实现
- 支持 GPU 加速（WebGPU）
- 活跃开发中

**缺点：**
- burn 0.5 还很新，API 可能不稳定
- WebGPU 后端需要更多配置

### 方案 4：使用 tch-rs（PyTorch，GPU）

```toml
[dependencies]
# PyTorch Rust 绑定
tch = "0.5"

# 屏幕捕获
scrap = "0.2"

# 异步运行时
tokio = { version = "1", features = ["sync", "time", "rt-multi-thread"] }
```

**优点：**
- 完整的 PyTorch 功能
- GPU 加速

**缺点：**
- 需要 libtorch（约 200MB）
- API 变化频繁
- 在 Windows 上配置复杂

## 当前配置建议

基于您的需求（避免外部依赖，高性能），我建议：

1. **立即方案**：使用 `scrap` + 优化的 `tract-onnx`
   - scrap 提供零拷贝屏幕捕获
   - tract-onnx 进行图优化
   - 预期性能：5-10 FPS @ 640x640

2. **短期方案**：添加 ONNX Runtime
   - ort = "2.0.0-rc.1" 提供 GPU 支持
   - 预期性能：15-30 FPS @ 640x640

3. **长期方案**：等待 burn 成熟
   - burn 0.6+ 可能更稳定
   - 纯 Rust 实现

## 添加 scrap 支持

```toml
# 在 Cargo.toml 中取消注释
scrap = "0.2"  # 高性能屏幕捕获（异步）
```

并在代码中使用：

```rust
use scrap::{Capturer, Display};

let displays = Display::all().unwrap();
let mut capturer = Capturer::new(&displays[0]).unwrap();

loop {
    if let Ok(frame) = capturer.frame() {
        // frame 是零拷贝的 RGBA 数据
        let rgba = frame.into_owned();
        // 处理帧...
    }
}
```

## 性能优化提示

1. **模型优化**
   - 使用 `into_optimized()` 而非 `into_runnable()`
   - 启用所有优化 pass

2. **输入尺寸**
   - 640x640 是速度和精度的平衡点
   - 320x320 更快但精度降低

3. **帧跳过**
   - 捕获 30 FPS，推理 15 FPS
   - 使用 tokio 异步 pipeline

4. **零拷贝**
   - scrap 提供零拷贝帧捕获
   - 避免不必要的数据复制
