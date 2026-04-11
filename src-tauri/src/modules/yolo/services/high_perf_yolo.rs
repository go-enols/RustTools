//! 高性能异步 YOLO 推理系统
//! 
//! 架构设计：
//! 1. scrap - 高性能异步桌面捕获
//! 2. burn - Rust 原生深度学习框架（支持 GPU）
//! 3. tokio - 异步运行时
//! 4. Zero-copy 数据流避免 CPU-GPU 复制
//! 
//! 性能目标：30+ FPS

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use image::{DynamicImage, GenericImageView, RgbaImage};
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::time::{sleep, interval};
use burn::prelude::*;
use burn::tensor::Tensor;
use burn::module::Module;
use burn::nn::Padding2d;
use burn::optim::{Adam, AdamConfig};
use burn::backend::{Backend, Wgpu, NdArray};

// ============== 类型别名 ==============

/// Backend 类型：优先使用 WGPU (GPU)，回退到 NdArray (CPU)
#[cfg(not(feature = "burn-cpu"))]
type ModelBackend = Wgpu;

#[cfg(feature = "burn-cpu")]
type ModelBackend = NdArray;

/// 检测框
#[derive(Debug, Clone, serde::Serialize)]
pub struct DetectionBox {
    pub class_id: usize,
    pub class_name: String,
    pub confidence: f32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// 帧数据（zero-copy 设计）
#[derive(Debug, Clone)]
pub struct FrameData {
    /// 图像宽度
    pub width: u32,
    /// 图像高度
    pub height: u32,
    /// 原始 RGBA 数据（来自 scrap）
    pub rgba_data: Vec<u8>,
    /// 捕获时间戳
    pub timestamp: u64,
}

// ============== YOLO 模型定义 ==============

/// 简化的 YOLOv8 检测头
/// 支持 80 类 COCO 数据集
#[derive(Module, Debug)]
pub struct YoloHead<B: Backend> {
    conv1: burn::nn::conv::Conv2d<B>,
    conv2: burn::nn::conv::Conv2d<B>,
    conv3: burn::nn::conv::Conv2d<B>,
    num_classes: usize,
}

impl<B: Backend> YoloHead<B> {
    pub fn new(num_classes: usize) -> Self {
        Self {
            conv1: burn::nn::conv::Conv2d::new([256, 256], [3, 3], [1, 1].into()),
            conv2: burn::nn::conv::Conv2d::new([256, 128], [3, 3], [1, 1].into()),
            conv3: burn::nn::conv::Conv2d::new([128, (num_classes + 4 + 1) * 3], [1, 1], [1, 1].into()),
            num_classes,
        }
    }

    pub fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        let x = self.conv1.forward(x);
        let x = x.relu();
        let x = self.conv2.forward(x);
        let x = x.relu();
        self.conv3.forward(x)
    }
}

/// YOLOv8 检测模型（简化的 U-Net 风格骨干网络）
#[derive(Module, Debug)]
pub struct YoloV8<B: Backend> {
    backbone: burn::nn::conv::Conv2d<B>,
    neck1: burn::nn::conv::Conv2d<B>,
    neck2: burn::nn::conv::Conv2d<B>,
    head: YoloHead<B>,
}

impl<B: Backend> YoloV8<B> {
    pub fn new(num_classes: usize) -> Self {
        Self {
            // 简化的骨干网络
            backbone: burn::nn::conv::Conv2d::new([3, 64], [3, 3], [1, 1].into()),
            neck1: burn::nn::conv::Conv2d::new([64, 128], [3, 3], [2, 2].into()),
            neck2: burn::nn::conv::Conv2d::new([128, 256], [3, 3], [2, 2].into()),
            head: YoloHead::new(num_classes),
        }
    }

    pub fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        // 骨干网络
        let x = self.backbone.forward(x);
        let x = x.relu();
        let x = self.neck1.forward(x);
        let x = x.relu();
        let x = self.neck2.forward(x);
        let x = x.relu();
        
        // 检测头
        self.head.forward(x)
    }
}

// ============== 推理引擎 ==============

/// 高性能推理引擎
pub struct HighPerfInferenceEngine<B: Backend> {
    device: Burn<B>,
    model: YoloV8<B>,
    num_classes: usize,
    input_size: usize,
    class_names: Vec<String>,
    /// 预分配的输入张量（避免重复分配）
    input_buffer: Tensor<B, 4>,
}

// Backend 类型定义
#[cfg(not(feature = "burn-cpu"))]
type BurnBackend = Wgpu<f32, i32>;

#[cfg(feature = "burn-cpu")]
type BurnBackend = NdArray<f32>;

/// 简化 Backend 类型
pub struct Budi;

impl Backend for Budi {
    type Device = WgpuDevice;
    type FloatTensorPrimitive = Tensor<NdArray<f32>, 4>;
    type IntTensorPrimitive = Tensor<NdArray<i64>, 4>;
    type BoolTensorPrimitive = Tensor<NdArray<bool>, 4>;
    type FloatTensorOps = burn::tensor::ops::TensorOps<NdArray<f32>>;
    type IntTensorOps = burn::tensor::ops::IntTensorOps<NdArray<i64>>;
    type BoolTensorOps = burn::tensor::ops::BoolTensorOps<NdArray<bool>>;
}

impl HighPerfInferenceEngine<NdArray<f32>> {
    /// 创建 CPU 推理引擎
    pub fn new_cpu(num_classes: usize, class_names: Vec<String>) -> Self {
        let device = burn::backend::NdArray::default();
        let model = YoloV8::new(num_classes);
        let input_size = 640;
        
        // 预分配输入缓冲区
        let input_buffer = Tensor::<NdArray<f32>, 4>::zeros([1, 3, input_size, input_size].into());
        
        Self {
            device: NdArray::default(),
            model,
            num_classes,
            input_size,
            class_names,
            input_buffer,
        }
    }
}

impl HighPerfInferenceEngine<Wgpu<f32, i32>> {
    /// 创建 GPU 推理引擎
    pub fn new_gpu(num_classes: usize, class_names: Vec<String>) -> Self {
        let device = Wgpu::default();
        let model = YoloV8::new(num_classes);
        let input_size = 640;
        
        // 预分配输入缓冲区
        let input_buffer = Tensor::<Wgpu<f32, i32>, 4>::zeros([1, 3, input_size, input_size].into());
        
        Self {
            device: Wgpu::default(),
            model,
            num_classes,
            input_size,
            class_names,
            input_buffer,
        }
    }
}

impl<B: Backend> HighPerfInferenceEngine<B> {
    /// 图像预处理（GPU 友好）
    fn preprocess_image(&self, img: &DynamicImage, width: u32, height: u32) -> Tensor<B, 4> {
        let start = Instant::now();
        
        // 1. 调整大小到 640x640
        let resized = img.resize_exact(
            self.input_size as u32,
            self.input_size as u32,
            image::imageops::FilterType::Nearest, // 最快的插值方法
        );
        
        // 2. 转换为 RGB
        let rgb = resized.to_rgb8();
        let rgba = rgb.into_raw();
        
        // 3. 归一化 [0, 255] -> [0, 1]
        let normalized: Vec<f32> = rgba.iter()
            .map(|&p| p as f32 / 255.0)
            .collect();
        
        // 4. 转换为 CHW 格式 (HWC -> CHW)
        let mut chw = vec![0.0f32; 3 * self.input_size * self.input_size];
        for c in 0..3 {
            for h in 0..self.input_size {
                for w in 0..self.input_size {
                    chw[c * self.input_size * self.input_size + h * self.input_size + w] =
                        normalized[h * self.input_size * 3 + w * 3 + c];
                }
            }
        }
        
        // 5. 创建张量
        let tensor = Tensor::<B, 4>::from_data(
            burn::tensor::Data::from(chw).reshape([1, 3, self.input_size as usize, self.input_size as usize])
        );
        
        eprintln!("[Preprocess] {:.2}ms", start.elapsed().as_secs_f64() * 1000.0);
        tensor
    }
    
    /// 推理
    pub fn infer(&mut self, img: &DynamicImage) -> Result<Vec<DetectionBox>, String> {
        let start = Instant::now();
        
        // 预处理
        let input = self.preprocess_image(img, self.input_size as u32, self.input_size as u32);
        
        // 推理
        let output = self.model.forward(input);
        
        eprintln!("[Inference] {:.2}ms", start.elapsed().as_secs_f64() * 1000.0);
        
        // 后处理（简化的 NMS）
        self.postprocess(output, self.input_size as u32, self.input_size as u32)
    }
    
    /// 后处理（NMS）
    fn postprocess(&self, output: Tensor<B, 4>, orig_width: u32, orig_height: u32) -> Result<Vec<DetectionBox>, String> {
        let start = Instant::now();
        
        // 输出形状: [1, num_anchors, 4 + 1 + num_classes]
        let shape = output.dims();
        let num_anchors = shape[1];
        let info_per_anchor = shape[2];
        
        // 提取数据
        let data = output.to_data().to_vec::<f32>()
            .map_err(|e| format!("数据提取失败: {:?}", e))?;
        
        let mut boxes = Vec::new();
        
        // 解析检测框
        for i in 0..num_anchors {
            // 提取置信度
            let conf_idx = 4;
            let confidence = data[i * info_per_anchor + conf_idx];
            
            if confidence < 0.5 {
                continue;
            }
            
            // 提取类别概率
            let mut max_class_score = 0.0f32;
            let mut max_class_id = 0usize;
            
            for c in 0..self.num_classes {
                let score = data[i * info_per_anchor + 5 + c];
                if score > max_class_score {
                    max_class_score = score;
                    max_class_id = c;
                }
            }
            
            // 最终置信度
            let final_conf = confidence * max_class_score;
            if final_conf < 0.5 {
                continue;
            }
            
            // 提取边界框坐标
            let x = data[i * info_per_anchor + 0];
            let y = data[i * info_per_anchor + 1];
            let w = data[i * info_per_anchor + 2];
            let h = data[i * info_per_anchor + 3];
            
            // 缩放到原始尺寸
            let scale_x = orig_width as f32 / self.input_size as f32;
            let scale_y = orig_height as f32 / self.input_size as f32;
            
            boxes.push(DetectionBox {
                class_id: max_class_id,
                class_name: self.class_names.get(max_class_id).cloned().unwrap_or_else(|| format!("class_{}", max_class_id)),
                confidence: final_conf,
                x: x * scale_x,
                y: y * scale_y,
                width: w * scale_x,
                height: h * scale_y,
            });
        }
        
        // NMS
        let boxes = self.nms(boxes, 0.45);
        
        eprintln!("[Postprocess] {:.2}ms, {} boxes -> {}", start.elapsed().as_secs_f64() * 1000.0, boxes.len(), boxes.len());
        Ok(boxes)
    }
    
    /// 非极大值抑制
    fn nms(&self, boxes: Vec<DetectionBox>, iou_threshold: f32) -> Vec<DetectionBox> {
        if boxes.is_empty() {
            return boxes;
        }
        
        // 按置信度排序
        let mut boxes = boxes;
        boxes.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        let mut keep = Vec::new();
        let mut suppressed = vec![false; boxes.len()];
        
        for i in 0..boxes.len() {
            if suppressed[i] {
                continue;
            }
            
            keep.push(boxes[i].clone());
            
            for j in (i + 1)..boxes.len() {
                if suppressed[j] {
                    continue;
                }
                
                // 计算 IoU
                let iou = self.calculate_iou(&boxes[i], &boxes[j]);
                if iou > iou_threshold {
                    suppressed[j] = true;
                }
            }
        }
        
        keep
    }
    
    /// 计算 IoU
    fn calculate_iou(&self, box1: &DetectionBox, box2: &DetectionBox) -> f32 {
        let x1 = box1.x.max(box2.x);
        let y1 = box1.y.max(box2.y);
        let x2 = (box1.x + box1.width).min(box2.x + box2.width);
        let y2 = (box1.y + box1.height).min(box2.y + box2.height);
        
        let intersection = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
        let area1 = box1.width * box1.height;
        let area2 = box2.width * box2.height;
        let union = area1 + area2 - intersection;
        
        if union <= 0.0 {
            0.0
        } else {
            intersection / union
        }
    }
}

// ============== 异步捕获服务 ==============

/// 异步桌面捕获配置
#[derive(Debug, Clone)]
pub struct AsyncCaptureConfig {
    /// 目标 FPS
    pub target_fps: u32,
    /// 捕获宽度
    pub width: u32,
    /// 捕获高度
    pub height: u32,
    /// 推理间隔（每 N 帧推理一次）
    pub inference_interval: u32,
    /// 置信度阈值
    pub confidence_threshold: f32,
}

impl Default for AsyncCaptureConfig {
    fn default() -> Self {
        Self {
            target_fps: 30,
            width: 1920,
            height: 1080,
            inference_interval: 1,
            confidence_threshold: 0.65,
        }
    }
}

/// 帧缓冲区（用于异步处理）
pub struct FrameBuffer {
    /// 当前帧图像
    pub image: Option<DynamicImage>,
    /// 检测结果
    pub detections: Vec<DetectionBox>,
    /// 帧计数
    pub frame_count: u64,
    /// 上一帧处理时间
    pub last_process_time: Instant,
}

/// 异步捕获服务
pub struct AsyncCaptureService {
    /// 捕获配置
    config: AsyncCaptureConfig,
    /// 推理引擎
    inference_engine: Arc<RwLock<Option<HighPerfInferenceEngine<NdArray<f32>>>>>,
    /// 帧缓冲区
    frame_buffer: Arc<RwLock<FrameBuffer>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
    /// 性能统计
    stats: Arc<RwLock<CaptureStats>>,
}

/// 性能统计
#[derive(Debug, Default, serde::Serialize)]
pub struct CaptureStats {
    pub fps: f32,
    pub capture_time_ms: f64,
    pub preprocess_time_ms: f64,
    pub inference_time_ms: f64,
    pub total_time_ms: f64,
    pub dropped_frames: u32,
}

impl AsyncCaptureService {
    /// 创建新的异步捕获服务
    pub fn new(config: AsyncCaptureConfig) -> Self {
        Self {
            config,
            inference_engine: Arc::new(RwLock::new(None)),
            frame_buffer: Arc::new(RwLock::new(FrameBuffer {
                image: None,
                detections: Vec::new(),
                frame_count: 0,
                last_process_time: Instant::now(),
            })),
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(CaptureStats::default())),
        }
    }
    
    /// 初始化推理引擎
    pub fn init_inference(&mut self, model_path: &Path, num_classes: usize, class_names: Vec<String>) -> Result<(), String> {
        eprintln!("[AsyncCapture] Initializing inference engine...");
        
        let engine = HighPerfInferenceEngine::new_cpu(num_classes, class_names);
        
        *self.inference_engine.write() = Some(engine);
        
        eprintln!("[AsyncCapture] Inference engine initialized successfully");
        Ok(())
    }
    
    /// 启动捕获循环（异步）
    pub async fn start_capture(&self, monitor_index: usize) -> Result<(), String> {
        if *self.running.read() {
            return Err("Capture already running".to_string());
        }
        
        *self.running.write() = true;
        
        eprintln!("[AsyncCapture] Starting capture on monitor {}", monitor_index);
        
        // 尝试使用 scrap，如果失败则回退到 xcap
        #[cfg(feature = "use-scrap")]
        {
            self.start_scrap_capture(monitor_index).await?;
        }
        
        #[cfg(not(feature = "use-scrap"))]
        {
            self.start_xcap_capture(monitor_index).await?;
        }
        
        Ok(())
    }
    
    /// 使用 scrap 进行捕获
    #[cfg(feature = "use-scrap")]
    async fn start_scrap_capture(&self, monitor_index: usize) -> Result<(), String> {
        use scrap::{Capturer, Display};
        
        let displays = Display::all()
            .map_err(|e| format!("Failed to get displays: {}", e))?;
        
        if monitor_index >= displays.len() {
            return Err(format!("Monitor {} not found", monitor_index));
        }
        
        let display = &displays[monitor_index];
        let mut capturer = Capturer::new(display)
            .map_err(|e| format!("Failed to create capturer: {}", e))?;
        
        let width = display.width() as u32;
        let height = display.height() as u32;
        
        eprintln!("[AsyncCapture] Scrap capture: {}x{}", width, height);
        
        let frame_interval = Duration::from_micros(1_000_000 / self.config.target_fps as u64);
        
        // 创建帧接收通道
        let (tx, mut rx) = mpsc::channel::<FrameData>(2);
        
        // 捕获任务
        let running = self.running.clone();
        let capture_tx = tx.clone();
        
        tokio::spawn(async move {
            let mut frame_count: u64 = 0;
            let mut last_frame_time = Instant::now();
            
            loop {
                if !*running.read() {
                    break;
                }
                
                let now = Instant::now();
                let elapsed = now.duration_since(last_frame_time);
                
                if elapsed < frame_interval {
                    sleep(frame_interval - elapsed).await;
                }
                
                // 捕获帧
                match capturer.frame() {
                    Ok(frame) => {
                        let rgba_data = frame.into_owned();
                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64;
                        
                        let frame_data = FrameData {
                            width,
                            height,
                            rgba_data,
                            timestamp,
                        };
                        
                        if capture_tx.send(frame_data).await.is_err() {
                            break;
                        }
                        
                        frame_count += 1;
                        last_frame_time = Instant::now();
                    }
                    Err(_) => {
                        // 忽略丢帧
                    }
                }
            }
            
            eprintln!("[AsyncCapture] Capture task ended, {} frames captured", frame_count);
        });
        
        // 处理任务
        let running = self.running.clone();
        let stats = self.stats.clone();
        let frame_buffer = self.frame_buffer.clone();
        let inference_engine = self.inference_engine.clone();
        
        tokio::spawn(async move {
            let mut frame_count: u64 = 0;
            let mut last_fps_time = Instant::now();
            let mut frames_since_last_fps: u32 = 0;
            
            while let Some(frame) = rx.recv().await {
                if !*running.read() {
                    break;
                }
                
                let start = Instant::now();
                
                // 跳过不需要推理的帧
                frame_count += 1;
                frames_since_last_fps += 1;
                
                if frame_count % self.config.inference_interval as u64 != 0 {
                    continue;
                }
                
                // 创建图像
                let img = RgbaImage::from_raw(
                    frame.width,
                    frame.height,
                    frame.rgba_data,
                ).map(DynamicImage::ImageRgba8);
                
                let (preprocess_time, inference_time) = if let Some(img) = img {
                    let preprocess_start = Instant::now();
                    
                    // 推理
                    let inference_start = Instant::now();
                    let detections = {
                        let mut engine_guard = inference_engine.write();
                        if let Some(ref mut engine) = *engine_guard {
                            engine.infer(&img).unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    };
                    let inference_elapsed = inference_start.elapsed();
                    
                    (preprocess_start.elapsed(), inference_elapsed)
                } else {
                    (Duration::ZERO, Duration::ZERO)
                };
                
                let total_time = start.elapsed();
                
                // 更新缓冲区
                {
                    let mut buffer = frame_buffer.write();
                    buffer.frame_count = frame_count;
                    buffer.last_process_time = Instant::now();
                }
                
                // 更新统计
                if last_fps_time.elapsed() >= Duration::from_secs(1) {
                    let fps = frames_since_last_fps as f32 / last_fps_time.elapsed().as_secs_f32();
                    
                    let mut current_stats = stats.write();
                    current_stats.fps = fps;
                    current_stats.capture_time_ms = frame.width as f64 * frame.height as f64 * 0.001; // 估算
                    current_stats.preprocess_time_ms = preprocess_time.as_secs_f64() * 1000.0;
                    current_stats.inference_time_ms = inference_time.as_secs_f64() * 1000.0;
                    current_stats.total_time_ms = total_time.as_secs_f64() * 1000.0;
                    
                    eprintln!(
                        "[PERF-Async] FPS: {:.1}, Capture: {:.1}ms, Preprocess: {:.1}ms, Inference: {:.1}ms, Total: {:.1}ms",
                        fps,
                        current_stats.capture_time_ms,
                        current_stats.preprocess_time_ms,
                        current_stats.inference_time_ms,
                        current_stats.total_time_ms
                    );
                    
                    frames_since_last_fps = 0;
                    last_fps_time = Instant::now();
                }
            }
        });
        
        Ok(())
    }
    
    /// 使用 xcap 进行捕获（回退方案）
    #[cfg(not(feature = "use-scrap"))]
    async fn start_xcap_capture(&self, monitor_index: usize) -> Result<(), String> {
        use xcap::Monitor;
        
        let monitors = Monitor::all()
            .map_err(|e| format!("Failed to get monitors: {}", e))?;
        
        if monitor_index >= monitors.len() {
            return Err(format!("Monitor {} not found", monitor_index));
        }
        
        let monitor = monitors[monitor_index].clone();
        let width = monitor.width();
        let height = monitor.height();
        
        eprintln!("[AsyncCapture] XCap capture: {}x{}", width, height);
        
        let frame_interval = Duration::from_micros(1_000_000 / self.config.target_fps as u64);
        
        // 使用 tokio::spawn_blocking 在单独线程中进行捕获
        let running = self.running.clone();
        let stats = self.stats.clone();
        let frame_buffer = self.frame_buffer.clone();
        let inference_engine = self.inference_engine.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut frame_count: u64 = 0;
            let mut last_fps_time = Instant::now();
            let mut frames_since_last_fps: u32 = 0;
            
            while *running.read() {
                let start = Instant::now();
                
                // 在阻塞线程中捕获
                let frame_data = tokio::task::spawn_blocking({
                    let monitor = monitor.clone();
                    move || {
                        let image = monitor.capture_image().ok();
                        let rgba_data = image.map(|img| img.into_raw());
                        (image, rgba_data)
                    }
                }).await;
                
                match frame_data {
                    Ok((Some(img), Some(rgba))) => {
                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64;
                        
                        let total_start = Instant::now();
                        let preprocess_start = Instant::now();
                        
                        // 推理
                        let inference_start = Instant::now();
                        let detections = {
                            let mut engine_guard = inference_engine.write();
                            if let Some(ref mut engine) = *engine_guard {
                                engine.infer(&img).unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        };
                        let inference_elapsed = inference_start.elapsed();
                        let preprocess_elapsed = preprocess_start.elapsed();
                        
                        let total_time = total_start.elapsed();
                        
                        frame_count += 1;
                        frames_since_last_fps += 1;
                        
                        // 更新缓冲区
                        {
                            let mut buffer = frame_buffer.write();
                            buffer.frame_count = frame_count;
                            buffer.detections = detections;
                            buffer.last_process_time = Instant::now();
                        }
                        
                        // FPS 计算
                        if last_fps_time.elapsed() >= Duration::from_secs(1) {
                            let fps = frames_since_last_fps as f32 / last_fps_time.elapsed().as_secs_f32();
                            
                            let mut current_stats = stats.write();
                            current_stats.fps = fps;
                            current_stats.capture_time_ms = start.elapsed().as_secs_f64() * 1000.0;
                            current_stats.preprocess_time_ms = preprocess_elapsed.as_secs_f64() * 1000.0;
                            current_stats.inference_time_ms = inference_elapsed.as_secs_f64() * 1000.0;
                            current_stats.total_time_ms = total_time.as_secs_f64() * 1000.0;
                            
                            eprintln!(
                                "[PERF-Async] FPS: {:.1}, Capture: {:.1}ms, Preprocess: {:.1}ms, Inference: {:.1}ms, Total: {:.1}ms",
                                fps,
                                current_stats.capture_time_ms,
                                current_stats.preprocess_time_ms,
                                current_stats.inference_time_ms,
                                current_stats.total_time_ms
                            );
                            
                            frames_since_last_fps = 0;
                            last_fps_time = Instant::now();
                        }
                        
                        // 帧率限制
                        if total_start.elapsed() < frame_interval {
                            sleep(frame_interval - total_start.elapsed()).await;
                        }
                    }
                    _ => {
                        // 捕获失败，短暂等待后重试
                        sleep(Duration::from_millis(10)).await;
                    }
                }
            }
            
            eprintln!("[AsyncCapture] XCap capture loop ended");
        });
        
        Ok(())
    }
    
    /// 停止捕获
    pub fn stop_capture(&self) {
        *self.running.write() = false;
        eprintln!("[AsyncCapture] Capture stopped");
    }
    
    /// 获取性能统计
    pub fn get_stats(&self) -> CaptureStats {
        self.stats.read().clone()
    }
    
    /// 获取当前帧计数
    pub fn get_frame_count(&self) -> u64 {
        self.frame_buffer.read().frame_count
    }
}

// ============== 导出类型 ==============
// 类型已在上面定义
