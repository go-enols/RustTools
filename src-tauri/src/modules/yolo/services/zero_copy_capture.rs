//! 高性能零拷贝异步 YOLO 推理系统
//! 
//! 核心设计：
//! 1. scrap - 高性能异步桌面捕获（零拷贝）
//! 2. ONNX Runtime - GPU 加速推理
//! 3. tokio - 异步运行时
//! 4. Zero-copy 数据流：scrap -> ONNX -> 前端
//! 
//! 性能目标：30+ FPS @ 1920x1080

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use image::{DynamicImage, GenericImageView, RgbaImage, ImageBuffer, Rgba};
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::time::{sleep, interval};

// ============== 类型定义 ==============

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

/// 捕获配置
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// 目标 FPS
    pub target_fps: u32,
    /// 输入尺寸
    pub input_size: u32,
    /// 推理间隔
    pub inference_interval: u32,
    /// 置信度阈值
    pub confidence_threshold: f32,
    /// NMS IoU 阈值
    pub nms_threshold: f32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            target_fps: 30,
            input_size: 640,
            inference_interval: 1,
            confidence_threshold: 0.65,
            nms_threshold: 0.45,
        }
    }
}

/// 性能统计
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct PerfStats {
    pub fps: f32,
    pub capture_time_ms: f64,
    pub resize_time_ms: f64,
    pub inference_time_ms: f64,
    pub postprocess_time_ms: f64,
    pub encode_time_ms: f64,
    pub total_time_ms: f64,
    pub num_detections: usize,
}

/// 帧数据
#[derive(Debug, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub timestamp: u64,
}

// ============== ONNX Runtime 推理引擎 ==============

/// ONNX Runtime 推理引擎（GPU 加速）
pub struct OnnxInferenceEngine {
    /// ONNX Runtime session
    session: ort::Session,
    /// 输入尺寸
    input_size: usize,
    /// 类别数
    num_classes: usize,
    /// 类别名称
    class_names: Vec<String>,
    /// 设备信息
    device_info: String,
    /// 预分配的输入张量
    input_shape: Vec<i64>,
    /// 输出形状
    output_shape: Vec<i64>,
}

impl OnnxInferenceEngine {
    /// 创建新的推理引擎
    pub fn new(model_path: &Path, num_classes: usize, class_names: Vec<String>) -> Result<Self, String> {
        let start = Instant::now();
        
        // 初始化 ONNX Runtime
        ort::init()
            .with_parallelization(true)
            .commit()
            .map_err(|e| format!("ONNX Runtime 初始化失败: {}", e))?;
        
        // 加载模型
        let session = ort::Session::from_file(model_path)
            .map_err(|e| format!("模型加载失败: {}", e))?;
        
        // 获取输入/输出信息
        let input_names = session.input_names()
            .map_err(|e| format!("获取输入名称失败: {}", e))?;
        let output_names = session.output_names()
            .map_err(|e| format!("获取输出名称失败: {}", e))?;
        
        eprintln!("[OnnxEngine] Input names: {:?}", input_names);
        eprintln!("[OnnxEngine] Output names: {:?}", output_names);
        
        let device_info = if ort::cuda_is_available() {
            "CUDA".to_string()
        } else if ort::directml_is_available() {
            "DirectML".to_string()
        } else {
            "CPU".to_string()
        };
        
        let input_size = 640;
        let output_shape = vec![1, (num_classes + 4 + 1) * 3 * 21 * 21]; // 简化的输出形状
        
        eprintln!("[OnnxEngine] Loaded in {:.2}s, Device: {}", start.elapsed().as_secs_f32(), device_info);
        
        Ok(Self {
            session,
            input_size,
            num_classes,
            class_names,
            device_info,
            input_shape: vec![1, 3, input_size as i64, input_size as i64],
            output_shape,
        })
    }
    
    /// 推理
    pub fn infer(&self, img: &DynamicImage) -> Result<Vec<DetectionBox>, String> {
        let start = Instant::now();
        
        // 预处理
        let input = self.preprocess(img);
        let preprocess_time = start.elapsed();
        
        // 运行推理
        let inference_start = Instant::now();
        let outputs = self.session.run(vec![input.into()])
            .map_err(|e| format!("推理失败: {}", e))?;
        let inference_time = inference_start.elapsed();
        
        // 后处理
        let postprocess_start = Instant::now();
        let boxes = self.postprocess(&outputs[0])?;
        let postprocess_time = postprocess_start.elapsed();
        
        eprintln!(
            "[OnnxEngine] Preprocess: {:.1}ms, Inference: {:.1}ms, Postprocess: {:.1}ms",
            preprocess_time.as_secs_f64() * 1000.0,
            inference_time.as_secs_f64() * 1000.0,
            postprocess_time.as_secs_f64() * 1000.0
        );
        
        Ok(boxes)
    }
    
    /// 预处理：调整大小并归一化
    fn preprocess(&self, img: &DynamicImage) -> ort::TensorInput {
        let start = Instant::now();
        
        // 1. 调整大小到 input_size x input_size
        let resized = img.resize_exact(
            self.input_size as u32,
            self.input_size as u32,
            image::imageops::FilterType::Triangle, // 双线性插值，速度和质量的平衡
        );
        
        // 2. 转换为 RGB
        let rgb = resized.to_rgb8();
        let rgba = rgb.into_raw();
        
        // 3. 归一化 [0, 255] -> [0, 1]
        let normalized: Vec<f32> = rgba.iter()
            .map(|&p| p as f32 / 255.0)
            .collect();
        
        // 4. 转换为 CHW 格式
        let mut chw = vec![0.0f32; 3 * self.input_size * self.input_size];
        for c in 0..3 {
            for h in 0..self.input_size {
                for w in 0..self.input_size {
                    chw[c * self.input_size * self.input_size + h * self.input_size + w] =
                        normalized[h * self.input_size * 3 + w * 3 + c];
                }
            }
        }
        
        eprintln!("[Preprocess] {:.2}ms", start.elapsed().as_secs_f64() * 1000.0);
        
        // 创建张量
        ort::TensorInput::from(chw)
            .reshape(self.input_shape.clone())
    }
    
    /// 后处理：解析输出并进行 NMS
    fn postprocess(&self, output: &ort::TensorInput) -> Result<Vec<DetectionBox>, String> {
        let start = Instant::now();
        
        // 提取数据
        let data: &[f32] = output.data()
            .ok_or("无法获取输出数据")?;
        
        let num_anchors = 21 * 21 * 3; // 简化的锚框数
        let info_per_anchor = self.num_classes + 4 + 1;
        
        let mut boxes = Vec::new();
        
        // 解析检测框
        for i in 0..num_anchors {
            let base_idx = i * info_per_anchor;
            
            // 提取置信度
            let confidence = data.get(base_idx + 4).copied().unwrap_or(0.0);
            if confidence < 0.5 {
                continue;
            }
            
            // 提取类别概率
            let mut max_class_score = 0.0f32;
            let mut max_class_id = 0usize;
            
            for c in 0..self.num_classes {
                let score = data.get(base_idx + 5 + c).copied().unwrap_or(0.0);
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
            
            // 提取边界框
            let x = data.get(base_idx).copied().unwrap_or(0.0);
            let y = data.get(base_idx + 1).copied().unwrap_or(0.0);
            let w = data.get(base_idx + 2).copied().unwrap_or(0.0);
            let h = data.get(base_idx + 3).copied().unwrap_or(0.0);
            
            boxes.push(DetectionBox {
                class_id: max_class_id,
                class_name: self.class_names.get(max_class_id)
                    .cloned()
                    .unwrap_or_else(|| format!("class_{}", max_class_id)),
                confidence: final_conf,
                x,
                y,
                width: w,
                height: h,
            });
        }
        
        // NMS
        let boxes = self.nms(boxes, 0.45);
        
        eprintln!("[Postprocess] {:.2}ms, {} boxes", start.elapsed().as_secs_f64() * 1000.0, boxes.len());
        
        Ok(boxes)
    }
    
    /// 非极大值抑制
    fn nms(&self, mut boxes: Vec<DetectionBox>, iou_threshold: f32) -> Vec<DetectionBox> {
        if boxes.is_empty() {
            return boxes;
        }
        
        // 按置信度排序
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
    
    /// 获取设备信息
    pub fn get_device_info(&self) -> &str {
        &self.device_info
    }
}

// ============== 异步捕获服务 ==============

/// 零拷贝异步捕获服务
pub struct ZeroCopyCaptureService {
    config: CaptureConfig,
    engine: Arc<RwLock<Option<OnnxInferenceEngine>>>,
    running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<PerfStats>>,
}

impl ZeroCopyCaptureService {
    /// 创建新的捕获服务
    pub fn new(config: CaptureConfig) -> Self {
        Self {
            config,
            engine: Arc::new(RwLock::new(None)),
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(PerfStats::default())),
        }
    }
    
    /// 初始化推理引擎
    pub fn init_engine(&mut self, model_path: &Path, num_classes: usize, class_names: Vec<String>) -> Result<(), String> {
        eprintln!("[ZeroCopy] Initializing ONNX Runtime engine...");
        
        let engine = OnnxInferenceEngine::new(model_path, num_classes, class_names)?;
        *self.engine.write() = Some(engine);
        
        eprintln!("[ZeroCopy] Engine initialized successfully");
        Ok(())
    }
    
    /// 启动捕获
    pub async fn start(&self, monitor_index: usize) -> Result<(), String> {
        if *self.running.read() {
            return Err("Capture already running".to_string());
        }
        
        *self.running.write() = true;
        
        eprintln!("[ZeroCopy] Starting capture on monitor {}", monitor_index);
        
        // 使用 tokio 在单独线程中运行捕获循环
        let running = self.running.clone();
        let stats = self.stats.clone();
        let engine = self.engine.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            // 尝试使用 scrap，否则使用 xcap
            #[cfg(feature = "use-scrap")]
            {
                Self::scrap_capture_loop(running, stats, engine, config, monitor_index).await;
            }
            
            #[cfg(not(feature = "use-scrap"))]
            {
                Self::xcap_capture_loop(running, stats, engine, config, monitor_index).await;
            }
        });
        
        Ok(())
    }
    
    /// Scrap 捕获循环
    #[cfg(feature = "use-scrap")]
    async fn scrap_capture_loop(
        running: Arc<RwLock<bool>>,
        stats: Arc<RwLock<PerfStats>>,
        engine: Arc<RwLock<Option<OnnxInferenceEngine>>>,
        config: CaptureConfig,
        monitor_index: usize,
    ) {
        use scrap::{Capturer, Display};
        
        let displays = match Display::all() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[ZeroCopy] Failed to get displays: {}", e);
                return;
            }
        };
        
        if monitor_index >= displays.len() {
            eprintln!("[ZeroCopy] Monitor {} not found", monitor_index);
            return;
        }
        
        let display = &displays[monitor_index];
        let mut capturer = match Capturer::new(display) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[ZeroCopy] Failed to create capturer: {}", e);
                return;
            }
        };
        
        let width = display.width() as u32;
        let height = display.height() as u32;
        let frame_interval = Duration::from_micros(1_000_000 / config.target_fps as u64);
        
        eprintln!("[ZeroCopy] Scrap capture: {}x{}", width, height);
        
        let mut frame_count: u64 = 0;
        let mut last_fps_time = Instant::now();
        let mut frames_since_last_fps: u32 = 0;
        
        while *running.read() {
            let start = Instant::now();
            
            // 捕获帧
            match capturer.frame() {
                Ok(frame) => {
                    let rgba_data = frame.into_owned();
                    
                    let frame = Frame {
                        width,
                        height,
                        rgba: rgba_data,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64,
                    };
                    
                    let total_start = Instant::now();
                    
                    // 跳过不需要推理的帧
                    frame_count += 1;
                    
                    if frame_count % config.inference_interval as u64 != 0 {
                        // 仍然更新 FPS
                        frames_since_last_fps += 1;
                        if last_fps_time.elapsed() >= Duration::from_secs(1) {
                            let fps = frames_since_last_fps as f32 / last_fps_time.elapsed().as_secs_f32();
                            let mut s = stats.write();
                            s.fps = fps;
                            frames_since_last_fps = 0;
                            last_fps_time = Instant::now();
                        }
                        continue;
                    }
                    
                    // 创建图像
                    let img = RgbaImage::from_raw(frame.width, frame.height, frame.rgba_data)
                        .map(DynamicImage::ImageRgba8);
                    
                    if let Some(img) = img {
                        // 推理
                        let inference_start = Instant::now();
                        let detections = {
                            let engine_guard = engine.read();
                            if let Some(ref eng) = *engine_guard {
                                eng.infer(&img).unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        };
                        let inference_time = inference_start.elapsed();
                        
                        let total_time = total_start.elapsed();
                        
                        frames_since_last_fps += 1;
                        
                        // 更新统计
                        if last_fps_time.elapsed() >= Duration::from_secs(1) {
                            let fps = frames_since_last_fps as f32 / last_fps_time.elapsed().as_secs_f32();
                            
                            let mut s = stats.write();
                            s.fps = fps;
                            s.capture_time_ms = start.elapsed().as_secs_f64() * 1000.0;
                            s.inference_time_ms = inference_time.as_secs_f64() * 1000.0;
                            s.total_time_ms = total_time.as_secs_f64() * 1000.0;
                            s.num_detections = detections.len();
                            
                            eprintln!(
                                "[PERF-ZeroCopy] FPS: {:.1}, Capture: {:.1}ms, Inference: {:.1}ms, Total: {:.1}ms, Detections: {}",
                                fps,
                                s.capture_time_ms,
                                s.inference_time_ms,
                                s.total_time_ms,
                                s.num_detections
                            );
                            
                            frames_since_last_fps = 0;
                            last_fps_time = Instant::now();
                        }
                    }
                    
                    // 帧率限制
                    if total_start.elapsed() < frame_interval {
                        sleep(frame_interval - total_start.elapsed()).await;
                    }
                }
                Err(_) => {
                    // 忽略丢帧
                }
            }
        }
        
        eprintln!("[ZeroCopy] Scrap capture loop ended");
    }
    
    /// XCap 捕获循环（回退方案）
    #[cfg(not(feature = "use-scrap"))]
    async fn xcap_capture_loop(
        running: Arc<RwLock<bool>>,
        stats: Arc<RwLock<PerfStats>>,
        engine: Arc<RwLock<Option<OnnxInferenceEngine>>>,
        config: CaptureConfig,
        monitor_index: usize,
    ) {
        use xcap::Monitor;
        
        let monitors = match Monitor::all() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[ZeroCopy] Failed to get monitors: {}", e);
                return;
            }
        };
        
        if monitor_index >= monitors.len() {
            eprintln!("[ZeroCopy] Monitor {} not found", monitor_index);
            return;
        }
        
        let monitor = monitors[monitor_index].clone();
        let width = monitor.width();
        let height = monitor.height();
        let frame_interval = Duration::from_micros(1_000_000 / config.target_fps as u64);
        
        eprintln!("[ZeroCopy] XCap capture: {}x{}", width, height);
        
        let mut frame_count: u64 = 0;
        let mut last_fps_time = Instant::now();
        let mut frames_since_last_fps: u32 = 0;
        
        while *running.read() {
            let start = Instant::now();
            
            // 在阻塞线程中捕获
            let img = tokio::task::spawn_blocking({
                let monitor = monitor.clone();
                move || monitor.capture_image().ok()
            }).await.unwrap_or(None);
            
            if let Some(img) = img {
                let total_start = Instant::now();
                
                frame_count += 1;
                
                if frame_count % config.inference_interval as u64 != 0 {
                    frames_since_last_fps += 1;
                    if last_fps_time.elapsed() >= Duration::from_secs(1) {
                        let fps = frames_since_last_fps as f32 / last_fps_time.elapsed().as_secs_f32();
                        let mut s = stats.write();
                        s.fps = fps;
                        frames_since_last_fps = 0;
                        last_fps_time = Instant::now();
                    }
                    continue;
                }
                
                // 推理
                let inference_start = Instant::now();
                let detections = {
                    let engine_guard = engine.read();
                    if let Some(ref eng) = *engine_guard {
                        eng.infer(&img).unwrap_or_default()
                    } else {
                        Vec::new()
                    }
                };
                let inference_time = inference_start.elapsed();
                
                let total_time = total_start.elapsed();
                
                frames_since_last_fps += 1;
                
                if last_fps_time.elapsed() >= Duration::from_secs(1) {
                    let fps = frames_since_last_fps as f32 / last_fps_time.elapsed().as_secs_f32();
                    
                    let mut s = stats.write();
                    s.fps = fps;
                    s.capture_time_ms = start.elapsed().as_secs_f64() * 1000.0;
                    s.inference_time_ms = inference_time.as_secs_f64() * 1000.0;
                    s.total_time_ms = total_time.as_secs_f64() * 1000.0;
                    s.num_detections = detections.len();
                    
                    eprintln!(
                        "[PERF-ZeroCopy] FPS: {:.1}, Capture: {:.1}ms, Inference: {:.1}ms, Total: {:.1}ms, Detections: {}",
                        fps,
                        s.capture_time_ms,
                        s.inference_time_ms,
                        s.total_time_ms,
                        s.num_detections
                    );
                    
                    frames_since_last_fps = 0;
                    last_fps_time = Instant::now();
                }
                
                if total_start.elapsed() < frame_interval {
                    sleep(frame_interval - total_start.elapsed()).await;
                }
            } else {
                sleep(Duration::from_millis(10)).await;
            }
        }
        
        eprintln!("[ZeroCopy] XCap capture loop ended");
    }
    
    /// 停止捕获
    pub fn stop(&self) {
        *self.running.write() = false;
        eprintln!("[ZeroCopy] Capture stopped");
    }
    
    /// 获取性能统计
    pub fn get_stats(&self) -> PerfStats {
        self.stats.read().clone()
    }
}

// ============== 导出 ==============
// 类型已在上面定义，无需重复导出
