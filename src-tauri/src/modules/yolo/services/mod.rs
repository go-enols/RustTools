pub mod trainer;
pub mod video;
pub mod video_inference;
pub mod device;
pub mod desktop_capture;
pub mod async_capture;
pub mod inference_engine;
pub mod model_converter;
pub mod model_optimizer;
pub mod yolo_inference_core;
pub mod desktop_performance_test;
pub mod burn_trainer;  // Burn原生YOLO训练器 - 纯Rust实现
pub mod yolo_dataset;  // YOLO数据集加载器
pub mod yolo_loss;     // YOLO损失函数
pub mod env;          // Rust环境检测模块
// pub mod yolo_gpu_inference;  // 待完善 tch-rs 集成
// pub mod async_desktop_capture;  // 有线程安全问题，暂时禁用
// pub mod high_perf_yolo;  // 需要 burn 依赖，暂时禁用
// 注意：ort (ONNX Runtime) 依赖暂时禁用
// pub mod zero_copy_capture;
// pub mod opt_capture;  // 有编译错误 - 待修复
// pub mod high_performance_desktop_capture;  // 待完善
// pub mod rust_native_yolo;  // 已删除 - 使用 scrap_capture.rs
// pub mod scrap_capture;  // 暂时禁用 - scrap API 版本不匹配
// pub mod scrap_burn_yolo;  // 有线程安全问题
pub mod scrap_burn_final;  // 修复线程安全问题的最终版本
pub mod python_env;

pub use trainer::TrainerService;
pub use burn_trainer::TrainingEvent;
pub use video::VideoService;
pub use video_inference::VideoInferenceService;
pub use desktop_capture::{DesktopCaptureService, MonitorInfo, AnnotationBox, DesktopCaptureFrame};
pub use async_capture::{start_opt_capture, get_capture_stats, PerfConfig, load_optimized_model};
pub use model_converter::{detect_model_format, get_model_info, is_model_compatible, ModelFormat, ConversionResult};
pub use inference_engine::{InferenceEngine, DetectionBox, MemoryPool};
pub use yolo_inference_core::{
    load_model,
    detect,
    draw_boxes,
    encode_image,
    encode_fast,
    InferenceConfig,
    DetectionBox as CoreDetectionBox,
};
pub use burn_trainer::{
    BurnTrainer,
    YOLOConfig,
    YOLOModel,
    TrainingConfig,
    TrainingState,
    TrainingEvent as BurnTrainingEvent,
};
pub use yolo_dataset::{
    YOLODataset,
    DatasetConfig,
    BoundingBox,
    ImageAnnotation,
};
pub use yolo_loss::{
    YOLOLoss,
    YOLOLossConfig,
    YOLOTarget,
};
// async_desktop_capture 暂时禁用（有线程安全问题）
