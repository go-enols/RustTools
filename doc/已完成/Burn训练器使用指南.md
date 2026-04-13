# Burn原生YOLO训练器使用指南

## 概述

本指南介绍如何使用纯Rust实现的Burn训练器来训练YOLO模型，完全替代Python依赖。

## 技术栈

- **训练框架**: Burn (Rust原生深度学习框架)
- **CPU后端**: burn-ndarray
- **GPU后端**: burn-cudarc (可选，需要CUDA环境)
- **数据加载**: 原生Rust实现

## 主要模块

### 1. burn_trainer - 训练器核心

```rust
use crate::modules::yolo::services::burn_trainer::{
    BurnTrainer,
    TrainingConfig,
    YOLOConfig,
    TrainingEvent,
};
```

**创建训练器:**

```rust
let trainer = BurnTrainer::new();
```

**训练配置:**

```rust
let config = TrainingConfig {
    project_name: "my_yolo_model".to_string(),
    epochs: 100,
    batch_size: 16,
    image_size: 640,
    num_classes: 4,  // 根据数据集调整
    optimizer: "SGD".to_string(),
    learning_rate: 0.01,
    weight_decay: 0.0005,
    momentum: 0.937,
    warmup_epochs: 3,
    device: "cpu".to_string(),  // 或 "cuda"
    workers: 8,
    save_period: 10,
};
```

### 2. yolo_dataset - 数据加载

```rust
use crate::modules::yolo::services::yolo_dataset::{
    YOLODataset,
    DatasetConfig,
    BoundingBox,
};
```

**创建数据集:**

```rust
let config = DatasetConfig {
    dataset_path: PathBuf::from("./dataset"),
    train_images: PathBuf::from("./dataset/train/images"),
    train_labels: PathBuf::from("./dataset/train/labels"),
    val_images: PathBuf::from("./dataset/val/images"),
    val_labels: PathBuf::from("./dataset/val/labels"),
    class_names: vec![
        "person".to_string(),
        "car".to_string(),
        "dog".to_string(),
        "cat".to_string(),
    ],
    num_classes: 4,
};

let dataset = YOLODataset::new(config, 640)?;
```

**数据增强:**

```rust
// 随机水平翻转
dataset.random_flip(&mut boxes);

// 随机亮度调整
let brightness = dataset.random_brightness();
```

### 3. yolo_loss - 损失函数

```rust
use crate::modules::yolo::services::yolo_loss::{
    YOLOLoss,
    YOLOLossConfig,
    YOLOTarget,
    BoxTarget,
    ciou_loss,
    calculate_iou,
};
```

**损失配置:**

```rust
let config = YOLOLossConfig {
    box_weight: 7.5,      // 边界框损失权重
    cls_weight: 0.5,      // 分类损失权重
    dfl_weight: 1.5,      // DFL损失权重
    cls_for_bg: 0.25,     // 背景类别的分类权重
};
```

## 完整训练示例

```rust
use tokio::sync::mpsc;

async fn train_yolo_model() -> Result<(), String> {
    // 1. 创建训练器
    let trainer = BurnTrainer::new();
    
    // 2. 创建训练配置
    let config = TrainingConfig {
        project_name: "wildlife_detector".to_string(),
        epochs: 300,
        batch_size: 16,
        image_size: 640,
        num_classes: 4,
        optimizer: "SGD".to_string(),
        learning_rate: 0.01,
        weight_decay: 0.0005,
        momentum: 0.937,
        warmup_epochs: 3,
        device: "cpu".to_string(),
        workers: 8,
        save_period: 10,
    };
    
    // 3. 创建事件通道
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    
    // 4. 启动训练
    let training_id = "train_001".to_string();
    let result = trainer.train_async(training_id, config, event_tx).await?;
    
    // 5. 处理训练事件
    while let Some(event) = event_rx.recv().await {
        match event {
            TrainingEvent::Started { total_epochs, .. } => {
                println!("训练开始，共 {} 个epoch", total_epochs);
            }
            TrainingEvent::BatchProgress(state) => {
                println!("Epoch {}/{}, Batch {}/{}, Loss: {:.4f}",
                    state.epoch, state.total_epochs,
                    state.batch, state.total_batches,
                    state.total_loss);
            }
            TrainingEvent::EpochComplete { epoch, map50, .. } => {
                if let Some(mAP) = map50 {
                    println!("Epoch {} 完成, mAP@50: {:.4f}", epoch, mAP);
                }
            }
            TrainingEvent::Complete { model_path } => {
                println!("训练完成! 模型保存到: {}", model_path);
                break;
            }
            TrainingEvent::Error { error } => {
                eprintln!("训练错误: {}", error);
                break;
            }
            TrainingEvent::Stopped => {
                println!("训练被停止");
                break;
            }
        }
    }
    
    Ok(())
}
```

## 从Python训练迁移

### Python方式 (已废弃)

```python
# 启动Python YOLO训练
python yolo_server.py 8080
# 发送训练命令...
```

### Rust方式 (推荐)

```rust
use crate::modules::yolo::services::burn_trainer::BurnTrainer;

// 直接在Rust中训练，无需外部进程
let trainer = BurnTrainer::new();
let result = trainer.train_async(id, config, event_tx).await?;
```

## 性能对比

| 指标 | Python (Ultralytics) | Rust (Burn) |
|------|---------------------|-------------|
| 训练速度 | 基准 | ~相同 |
| 依赖 | Python + PyTorch | 仅Rust |
| 可移植性 | 受限于Python环境 | 完全可移植 |
| GPU支持 | 原生CUDA | burn-cudarc |
| 部署难度 | 需要Python运行时 | 纯二进制 |

## GPU加速 (可选)

要启用GPU训练，需要：

1. 安装CUDA Toolkit (11.8或12.1)
2. 在`Cargo.toml`中启用burn-cudarc:

```toml
burn-cudarc = "0.5"  # 取消注释
```

3. 设置设备为"cuda":

```rust
let config = TrainingConfig {
    device: "cuda".to_string(),
    // ...
};
```

## 模型导出

训练完成后，模型将保存为ONNX格式：

```
train/{project_name}/weights/best.onnx
```

可以使用现有的推理引擎进行部署：

```rust
use crate::modules::yolo::services::inference_engine::InferenceEngine;

let engine = InferenceEngine::new("train/wildlife_detector/weights/best.onnx")?;
let detections = engine.detect(&image, 0.65)?;
```

## 限制和注意事项

### 当前限制

1. **模型简化**: 简化版的YOLOv8模型架构用于演示
2. **性能**: CPU训练速度较慢，建议使用GPU
3. **数据增强**: 当前实现较基础，可扩展更多增强方法

### 待完成功能

- [ ] 完整的YOLOv8架构实现
- [ ] 更多数据增强方法 (Mosaic, MixUp等)
- [ ] 学习率调度器 (Cosine Annealing, Warmup等)
- [ ] 模型导出为TorchScript格式
- [ ] 验证集评估和mAP计算

## 故障排除

### 问题1: 编译错误 "cannot find module"

**解决方案**: 确保在`src-tauri/src/modules/yolo/services/mod.rs`中导入了模块:

```rust
pub mod burn_trainer;
pub mod yolo_dataset;
pub mod yolo_loss;
```

### 问题2: 训练很慢

**解决方案**: 
1. 使用GPU加速 (设置`device: "cuda"`)
2. 减小图像尺寸 (从640降到320)
3. 减小batch_size以适应内存

### 问题3: OOM (内存不足)

**解决方案**:
1. 减小batch_size
2. 使用更小的图像尺寸
3. 启用梯度累积

## 进一步优化建议

1. **混合精度训练**: 实现FP16训练以减少内存使用
2. **多GPU训练**: 使用数据并行
3. **模型量化**: 训练后量化以减少模型大小
4. **更深的网络**: 实现完整的YOLOv8l或YOLOv8x

## 总结

Burn训练器提供了一个完全原生Rust的YOLO训练解决方案，消除了对Python的依赖。虽然当前版本是简化实现，但它为未来完整功能奠定了基础。通过使用Burn，我们可以：

✅ 消除Python依赖
✅ 实现完全可移植的部署
✅ 利用Rust的性能优势
✅ 集成到现有的Rust项目中

随着更多功能的实现，Burn训练器将成为生产环境中的可靠选择。
