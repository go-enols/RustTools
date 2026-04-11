//! Desktop Capture Service - Optimized Rust Implementation
//! 
//! Performance optimizations:
//! 1. Model compiled once, reused for all inferences
//! 2. Pre-allocated buffers for image processing
//! 3. Fast nearest-neighbor resize instead of Lanczos3
//! 4. Reduced image quality for faster encoding
//! 5. Standard threads instead of tokio (Monitor is not Send)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use xcap::Monitor;
use tauri::{AppHandle, Emitter};
use image::{DynamicImage, Rgb, imageops::FilterType};
use tract_onnx::prelude::*;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

/// Default class names for common YOLO models (COCO 80 classes)
const DEFAULT_CLASS_NAMES: [&str; 80] = [
    "person", "bicycle", "car", "motorbike", "aeroplane", "bus", "train", "truck", "boat",
    "traffic light", "fire hydrant", "stop sign", "parking meter", "bench", "bird", "cat",
    "dog", "horse", "sheep", "cow", "elephant", "bear", "zebra", "giraffe", "backpack",
    "umbrella", "handbag", "tie", "suitcase", "frisbee", "skis", "snowboard", "sports ball",
    "kite", "baseball bat", "baseball glove", "skateboard", "surfboard", "tennis racket",
    "bottle", "wine glass", "cup", "fork", "knife", "spoon", "bowl", "banana", "apple",
    "sandwich", "orange", "broccoli", "carrot", "hot dog", "pizza", "donut", "cake", "chair",
    "sofa", "potted plant", "bed", "dining table", "toilet", "tvmonitor", "laptop", "mouse",
    "remote", "keyboard", "cell phone", "microwave", "oven", "toaster", "sink", "refrigerator",
    "book", "clock", "vase", "scissors", "teddy bear", "hair drier", "toothbrush",
];

/// African wildlife specific class names
const WILDLIFE_CLASS_NAMES: [&str; 4] = [
    "elephant", "buffalo", "rhino", "zebra"
];

/// Monitor information
#[derive(Debug, Clone, serde::Serialize)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

/// Desktop capture status
#[derive(Debug, Clone, serde::Serialize)]
pub struct DesktopCaptureStatus {
    pub active_sessions: Vec<String>,
    pub total_sessions: usize,
}

/// Detection box
#[derive(Debug, Clone, serde::Serialize)]
pub struct AnnotationBox {
    pub id: String,
    pub class_id: usize,
    pub class_name: String,
    pub confidence: f32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Desktop capture frame with detections
#[derive(Debug, Clone, serde::Serialize)]
pub struct DesktopCaptureFrame {
    pub session_id: String,
    pub image: String, // Base64 encoded JPEG
    pub boxes: Vec<AnnotationBox>,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub timestamp: u64,
}

/// Desktop capture session info
struct DesktopSession {
    #[allow(dead_code)]
    model_path: String,
    #[allow(dead_code)]
    confidence: f32,
    #[allow(dead_code)]
    monitor_idx: usize,
    #[allow(dead_code)]
    fps_limit: f32,
    is_running: Arc<Mutex<bool>>,
    handle: Option<thread::JoinHandle<()>>,
}

/// Type alias for tract runnable model (SimplePlan)
type TractModel = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

/// Desktop Capture Service
pub struct DesktopCaptureService {
    sessions: Arc<Mutex<HashMap<String, DesktopSession>>>,
}

impl DesktopCaptureService {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub async fn get_status(&self) -> DesktopCaptureStatus {
        let sessions = self.sessions.lock().unwrap();
        let active_sessions: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| *session.is_running.lock().unwrap())
            .map(|(id, _)| id.clone())
            .collect();
        
        DesktopCaptureStatus {
            active_sessions,
            total_sessions: sessions.len(),
        }
    }
    
    /// Get available monitors
    pub fn get_monitors(&self) -> Result<Vec<MonitorInfo>, String> {
        let monitors = Monitor::all().map_err(|e| format!("Failed to get monitors: {}", e))?;
        
        let monitor_infos: Vec<MonitorInfo> = monitors
            .iter()
            .enumerate()
            .map(|(i, m)| {
                MonitorInfo {
                    id: (i + 1) as u32,
                    name: m.name().to_string(),
                    x: m.x(),
                    y: m.y(),
                    width: m.width(),
                    height: m.height(),
                    is_primary: i == 0,
                }
            })
            .collect();
        
        Ok(monitor_infos)
    }
    
    /// Load YOLO model - returns compiled runnable model
    pub fn load_yolo_model(model_path: &str) -> Result<TractModel, String> {
        let resolved_path = crate::modules::yolo::services::model_converter::resolve_inference_model_path(model_path)?;

        eprintln!(
            "[Desktop] Loading YOLO model from: {} -> {}",
            model_path,
            resolved_path.display()
        );
        
        // Load and compile model ONCE
        let model = tract_onnx::onnx()
            .model_for_path(&resolved_path)
            .map_err(|e| format!("Failed to load model: {}", e))?
            .with_input_fact(0, f32::fact(&[1, 3, 640, 640]).into())
            .map_err(|e| format!("Failed to configure input: {}", e))?
            .into_typed()
            .map_err(|e| format!("Failed to type model: {}", e))?
            .into_runnable()
            .map_err(|e| format!("Failed to compile model: {}", e))?;
        
        eprintln!("[Desktop] Model compiled successfully");
        
        Ok(model)
    }
    
    /// Fast image preprocessing for YOLO (optimized)
    fn preprocess_image_fast(img: &DynamicImage, target_size: usize) -> Result<Tensor, String> {
        // Use fast nearest-neighbor resize instead of slow Lanczos3
        let resized = img.resize_exact(
            target_size as u32,
            target_size as u32,
            FilterType::Nearest,
        );
        
        let rgb = resized.to_rgb8();
        let (height, width) = rgb.dimensions();
        
        // Pre-allocate with exact capacity
        let mut data = vec![0.0f32; 3 * height as usize * width as usize];
        
        // Fast RGB to BGR conversion with pre-computed indices
        let pixels = rgb.as_raw();
        for i in 0..(height as usize * width as usize) {
            let src_idx = i * 3;
            // RGB -> BGR
            data[i] = pixels[src_idx + 2] as f32 / 255.0;                    // R -> B
            data[(height as usize * width as usize) + i] = pixels[src_idx + 1] as f32 / 255.0;  // G -> G
            data[2 * (height as usize * width as usize) + i] = pixels[src_idx] as f32 / 255.0;   // B -> R
        }
        
        Tensor::from_shape(&[1, 3, height as usize, width as usize], &data)
            .map_err(|e| format!("Failed to create tensor: {}", e))
    }
    
    /// 🚀 超快图像预处理 - 假设输入已经是 640x640
    /// 跳过 resize 步骤，直接进行归一化和格式转换
    fn preprocess_image_fast_fixed(img: &DynamicImage) -> Result<Tensor, String> {
        let rgb = img.to_rgb8();
        let (height, width) = rgb.dimensions();
        
        // 验证图像尺寸（应该是 640x640）
        debug_assert_eq!(height, 640, "Image height should be 640");
        debug_assert_eq!(width, 640, "Image width should be 640");
        
        // Pre-allocate with exact capacity (固定大小 640*640*3)
        let mut data = vec![0.0f32; 3 * 640 * 640];
        
        // Fast RGB to BGR conversion with pre-computed indices
        let pixels = rgb.as_raw();
        let area = 640 * 640;
        
        for i in 0..area {
            let src_idx = i * 3;
            // RGB -> BGR (YOLO 期望 BGR)
            data[i] = pixels[src_idx + 2] as f32 / 255.0;           // R -> B
            data[area + i] = pixels[src_idx + 1] as f32 / 255.0;    // G -> G
            data[2 * area + i] = pixels[src_idx] as f32 / 255.0;   // B -> R
        }
        
        Tensor::from_shape(&[1, 3, 640, 640], &data)
            .map_err(|e| format!("Failed to create tensor: {}", e))
    }
    
    /// Run YOLO inference (optimized) - 支持 YOLOv8 格式 [1, 84, 8400]
    /// 
    /// 修复说明：
    /// - YOLOv8 输出格式: [batch, features, boxes] 即 [1, 84, 8400]
    /// - 84 = 4 (bbox: cx,cy,w,h) + 80 (classes)
    /// - 原代码错误地使用了 [batch, boxes, features] 格式
    fn run_inference(
        model: &TractModel,
        img: &DynamicImage,
        confidence: f32,
        orig_width: u32,
        orig_height: u32,
    ) -> Result<Vec<(f32, f32, f32, f32, f32, usize)>, String> {
        // 预处理
        let preprocess_start = Instant::now();
        let input = Self::preprocess_image_fast_fixed(img)?;
        let preprocess_time = preprocess_start.elapsed();
        
        // 模型推理
        let inference_start = Instant::now();
        let result = model.run(tvec![input.into()])
            .map_err(|e| format!("Inference failed: {}", e))?;
        let inference_time = inference_start.elapsed();
        
        let output = &result[0];
        let shape = output.shape();
        
        // 调试：打印输出形状和大小
        eprintln!("[DEBUG] Model output shape: {:?}, len: {}", shape, output.len());
        let output_size_mb = output.len() * 4 / (1024 * 1024);
        eprintln!("[DEBUG] Model output size: ~{} MB", output_size_mb);
        
        if shape.len() != 3 {
            return Err(format!("Unexpected output shape: {:?}", shape));
        }
        
        // YOLOv8 格式: [batch, features, boxes]
        // features = 84 = 4 (bbox) + 80 (classes)
        let batch_size = shape[0] as usize;
        let num_features = shape[1] as usize;
        let num_boxes = shape[2] as usize;
        
        if batch_size != 1 {
            return Err(format!("Expected batch size 1, got {}", batch_size));
        }
        
        let num_classes = if num_features > 4 {
            num_features - 4
        } else {
            return Err(format!("Invalid output shape: expected 84 features (4 bbox + 80 classes), got {}", num_features));
        };
        
        // 如果是 YOLOv8 格式 [1, 84, 8400]，打印日志（仅在第一次推理时）
        if num_features == 84 && num_boxes == 8400 {
            eprintln!("[Desktop] 检测到 YOLOv8 格式输出 [1, 84, 8400]");
        }
        
        let scale_x = orig_width as f32 / 640.0;
        let scale_y = orig_height as f32 / 640.0;
        
        let output_data = output.to_array_view::<f32>()
            .map_err(|e| format!("Failed to access output: {}", e))?;
        
        // 后处理
        let postprocess_start = Instant::now();
        let mut detections = Vec::with_capacity(100); // Pre-allocate
        
        // YOLOv8 格式: [batch, features, boxes]
        // features[0:4] = bbox (cx, cy, w, h)
        // features[4:84] = class scores (80 classes)
        for i in 0..num_boxes {
            // 找最大类别分数
            let mut max_score = 0.0f32;
            let mut max_class = 0usize;
            
            for c in 0..num_classes {
                // YOLOv8 格式: output_data[[batch, feature_idx, box_idx]]
                let score = output_data[[0, c + 4, i]];
                if score > max_score {
                    max_score = score;
                    max_class = c;
                }
            }
            
            // 应用置信度阈值（YOLOv8 的 score 已经是 sigmoid 后的值，在 0-1 之间）
            if max_score >= confidence {
                // 读取边界框坐标 (YOLOv8 格式)
                let cx = output_data[[0, 0, i]];
                let cy = output_data[[0, 1, i]];
                let w = output_data[[0, 2, i]];
                let h = output_data[[0, 3, i]];
                
                // YOLOv8 格式: bbox 坐标已经是绝对像素值 (0-640)
                // 不需要乘以 640！
                let cx_abs = cx;  // 已经是绝对坐标
                let cy_abs = cy;
                let w_abs = w;
                let h_abs = h;
                
                // 转换为 [x1, y1, x2, y2] 格式
                let x1 = (cx_abs - w_abs / 2.0).max(0.0) * scale_x;
                let y1 = (cy_abs - h_abs / 2.0).max(0.0) * scale_y;
                let x2 = (cx_abs + w_abs / 2.0).min(640.0) * scale_x;
                let y2 = (cy_abs + h_abs / 2.0).min(640.0) * scale_y;
                
                detections.push((x1, y1, x2, y2, max_score, max_class));
            }
        }
        let postprocess_time = postprocess_start.elapsed();
        
        // 保存原始检测数量(在移动之前)
        let initial_detections_count = detections.len();
        
        // NMS
        let nms_start = Instant::now();
        let result = Self::nms(detections, 0.45);
        let nms_time = nms_start.elapsed();
        
        let total_time = preprocess_time + inference_time + postprocess_time + nms_time;
        eprintln!("[PERF-Inference] 预处理: {:.1}ms | 推理: {:.1}ms | 后处理: {:.1}ms | NMS: {:.1}ms | 总计: {:.1}ms (初始: {}, 最终: {})",
            preprocess_time.as_secs_f32() * 1000.0,
            inference_time.as_secs_f32() * 1000.0,
            postprocess_time.as_secs_f32() * 1000.0,
            nms_time.as_secs_f32() * 1000.0,
            total_time.as_secs_f32() * 1000.0,
            initial_detections_count,
            result.len()
        );
        
        Ok(result)
    }
    
    /// Non-Maximum Suppression (optimized)
    fn nms(
        mut boxes: Vec<(f32, f32, f32, f32, f32, usize)>,
        iou_threshold: f32,
    ) -> Vec<(f32, f32, f32, f32, f32, usize)> {
        if boxes.len() <= 1 {
            return boxes;
        }
        
        boxes.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap());
        
        let mut keep = Vec::with_capacity(boxes.len());
        
        while let Some(best) = boxes.pop() {
            keep.push(best);
            boxes.retain(|box_| Self::calculate_iou(&best, box_) < iou_threshold);
        }
        
        keep
    }
    
    /// Calculate IoU
    fn calculate_iou(
        box1: &(f32, f32, f32, f32, f32, usize),
        box2: &(f32, f32, f32, f32, f32, usize),
    ) -> f32 {
        let x1_inter = box1.0.max(box2.0);
        let y1_inter = box1.1.max(box2.1);
        let x2_inter = box1.2.min(box2.2);
        let y2_inter = box1.3.min(box2.3);
        
        let inter_area = ((x2_inter - x1_inter).max(0.0) * (y2_inter - y1_inter).max(0.0)).max(0.0);
        let area1 = (box1.2 - box1.0).max(0.0) * (box1.3 - box1.1).max(0.0);
        let area2 = (box2.2 - box2.0).max(0.0) * (box2.3 - box2.1).max(0.0);
        let union_area = area1 + area2 - inter_area;
        
        if union_area > 0.0 {
            inter_area / union_area
        } else {
            0.0
        }
    }
    
    /// Fast JPEG encoding with quality control - 🚀 优化版本
    fn encode_image_fast(img: &DynamicImage) -> Result<String, String> {
        use std::io::Cursor;
        
        // 🚀 性能优化：如果图像很大，先 resize 到 640x640（进一步缩小）
        let rgb = if img.width() > 640 || img.height() > 640 {
            img.resize(640, 640, FilterType::Nearest)
                .to_rgb8()
        } else {
            img.to_rgb8()
        };
        
        // 🚀 性能优化：降低 JPEG 质量到 40（从 70/60 降低）
        // 这样可以大幅加快编码速度，同时保持足够的视觉效果
        let quality = 40;
        
        let mut buffer = Cursor::new(Vec::with_capacity(rgb.len() / 4)); // Pre-allocate smaller
        
        // Use jpeg encoder with quality setting
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
        encoder.encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        ).map_err(|e| format!("Failed to encode: {}", e))?;
        
        Ok(BASE64.encode(buffer.into_inner()))
    }
    
    /// Draw detection boxes (optimized)
    fn draw_boxes(img: &DynamicImage, boxes: &[(f32, f32, f32, f32, f32, usize)]) -> DynamicImage {
        let mut rgb = img.to_rgb8();
        let (width, height) = rgb.dimensions();
        
        let colors: [[u8; 3]; 8] = [
            [255, 107, 107], [78, 205, 196], [69, 183, 209],
            [150, 206, 180], [255, 234, 167], [221, 160, 221],
            [255, 159, 67], [199, 199, 199],
        ];
        
        for (x1, y1, x2, y2, _, class_id) in boxes.iter() {
            let color = colors[*class_id as usize % 8];
            
            let x1 = (*x1 as i32).clamp(0, width as i32 - 1);
            let y1 = (*y1 as i32).clamp(0, height as i32 - 1);
            let x2 = (*x2 as i32).clamp(0, width as i32 - 1);
            let y2 = (*y2 as i32).clamp(0, height as i32 - 1);
            
            // Draw rectangle with thickness 3
            let thickness = 3;
            
            for x in x1..x2 {
                for t in 0..thickness {
                    if y1 + t >= 0 && y1 + t < height as i32 && x >= 0 && x < width as i32 {
                        rgb.put_pixel(x as u32, (y1 + t) as u32, Rgb(color));
                    }
                    if y2 - t >= 0 && y2 - t < height as i32 && x >= 0 && x < width as i32 {
                        rgb.put_pixel(x as u32, (y2 - t) as u32, Rgb(color));
                    }
                }
            }
            
            for y in y1..y2 {
                for t in 0..thickness {
                    if x1 + t >= 0 && x1 + t < width as i32 && y >= 0 && y < height as i32 {
                        rgb.put_pixel((x1 + t) as u32, y as u32, Rgb(color));
                    }
                    if x2 - t >= 0 && x2 - t < width as i32 && y >= 0 && y < height as i32 {
                        rgb.put_pixel((x2 - t) as u32, y as u32, Rgb(color));
                    }
                }
            }
        }
        
        DynamicImage::ImageRgb8(rgb)
    }
    
    /// Start capture with optimized YOLO inference
    pub async fn start_capture(
        &self,
        session_id: String, // Changed to owned String
        model_path: String,
        confidence: f32,
        monitor: u32,
        fps_limit: u32,
        app: AppHandle,
    ) -> Result<(), String> {
        eprintln!("[Desktop] Starting capture for session: {}", session_id);
        
        let monitors = self.get_monitors()?;
        if monitors.is_empty() {
            return Err("No monitors found".to_string());
        }
        
        let monitor_idx = (monitor as usize).saturating_sub(1).min(monitors.len() - 1);
        let monitor_info = &monitors[monitor_idx];
        
        eprintln!("[Desktop] Monitor: {} ({}x{})", monitor_info.name, monitor_info.width, monitor_info.height);
        
        // Load model once before the loop
        let yolo_model = if !model_path.is_empty() {
            match Self::load_yolo_model(&model_path) {
                Ok(model) => {
                    eprintln!("[Desktop] YOLO model loaded successfully");
                    Some(model)
                }
                Err(e) => {
                    eprintln!("[Desktop] Warning: Failed to load YOLO model: {}", e);
                    None
                }
            }
        } else {
            eprintln!("[Desktop] No model specified");
            None
        };
        
        let is_running = Arc::new(Mutex::new(true));
        let is_running_clone = Arc::clone(&is_running);
        
        let mut sessions = self.sessions.lock().unwrap();
        
        let session_id_for_handle = session_id.clone(); // Clone for thread
        let session_id_for_sessions = session_id.clone(); // Clone for HashMap
        
        let handle = thread::spawn(move || {
            // 🚀 性能优化：限制最大帧率为 15 FPS（从用户设置的 fps_limit 降低）
            // 这样可以大幅减少前端压力和网络带宽
            let target_fps = (fps_limit as f32).min(15.0);
            let frame_duration = Duration::from_secs_f64(1.0 / target_fps as f64);
            
            let mut frame_count = 0u32;
            let mut last_fps_time = Instant::now();
            let mut current_fps = 0.0f32;
            
            // 🚀 性能优化：帧跳过机制
            // 用于追踪上一帧的处理时间
            let mut last_frame_time = Instant::now();
            
            eprintln!("[Desktop] Capture loop started (Target FPS: {}, Conf: {}, Max: {})", target_fps, confidence, fps_limit);
            
            // Get monitor list once (avoid repeated Monitor::all() calls)
            let monitors = Monitor::all().unwrap_or_default();
            if monitors.is_empty() {
                eprintln!("[Desktop] No monitors available");
                return;
            }
            
            loop {
                // Check if we should stop
                if !*is_running_clone.lock().unwrap() {
                    eprintln!("[Desktop] Capture loop stopped");
                    break;
                }
                
                // 🚀 性能优化：帧跳过机制 - 使用 sleep 而不是忙等待
                // 如果当前帧的处理时间还没到帧间隔，等待一段时间
                let elapsed_since_last = last_frame_time.elapsed();
                if elapsed_since_last < frame_duration {
                    // 还没到时间，睡眠等待
                    let sleep_time = frame_duration - elapsed_since_last;
                    thread::sleep(sleep_time);
                }
                last_frame_time = Instant::now();
                
                let frame_start = Instant::now();
                let mut perf_stats = String::new(); // 性能统计字符串
                
                // Ensure monitor index is valid
                let monitor_idx = monitor_idx.min(monitors.len() - 1);
                
                // Capture screen
                if let Ok(captured) = monitors[monitor_idx].capture_image() {
                    let capture_time = frame_start.elapsed();
                    perf_stats.push_str(&format!("[PERF] 捕获: {:.1}ms", capture_time.as_secs_f32() * 1000.0));
                    
                    let (orig_width, orig_height) = captured.dimensions();
                    let orig_img = DynamicImage::ImageRgba8(captured);
                    
                    // 🚀 性能优化：立即 resize 到 640x640 用于推理
                    // 这样可以大幅减少内存占用和推理时间
                    let resize_start = Instant::now();
                    let inference_img = orig_img.resize_exact(
                        640u32, 
                        640u32, 
                        FilterType::Nearest  // 使用 Nearest Neighbor 加速 resize
                    );
                    let resize_time = resize_start.elapsed();
                    perf_stats.push_str(&format!(" | Resize: {:.1}ms", resize_time.as_secs_f32() * 1000.0));
                    
                    // Run inference if model is loaded
                    // 使用 640x640 小图进行推理，而不是全分辨率
                    let inference_start = Instant::now();
                    let encode_start = inference_start; // 初始化编码开始时间
                    let boxes = if let Some(ref model) = yolo_model {
                        match Self::run_inference(model, &inference_img, confidence, orig_width, orig_height) {
                            Ok(detections) => {
                                if !detections.is_empty() {
                                    eprintln!("[Desktop] Detected {} objects", detections.len());
                                    for (x1, y1, x2, y2, conf, class_id) in &detections {
                                        eprintln!("[Desktop]   - Class {} at ({:.0}, {:.0}, {:.0}, {:.0}) conf {:.2}", 
                                            class_id, x1, y1, x2, y2, conf);
                                    }
                                }
                                
                                detections.into_iter().enumerate().map(|(idx, (x1, y1, x2, y2, conf, class_id))| {
                                    let class_name = if class_id < DEFAULT_CLASS_NAMES.len() {
                                        DEFAULT_CLASS_NAMES[class_id].to_string()
                                    } else if class_id < WILDLIFE_CLASS_NAMES.len() {
                                        WILDLIFE_CLASS_NAMES[class_id].to_string()
                                    } else {
                                        format!("Object {}", class_id)
                                    };
                                    
                                    AnnotationBox {
                                        id: format!("{}_{}", session_id_for_handle, idx),
                                        class_id,
                                        class_name,
                                        confidence: conf,
                                        x: x1,
                                        y: y1,
                                        width: x2 - x1,
                                        height: y2 - y1,
                                    }
                                }).collect()
                            }
                            Err(e) => {
                                eprintln!("[Desktop] Inference error: {}", e);
                                vec![]
                            }
                        }
                    } else {
                        vec![]
                    };
                    let inference_time = inference_start.elapsed();
                    perf_stats.push_str(&format!(" | 推理: {:.1}ms", inference_time.as_secs_f32() * 1000.0));
                    
                    // 🚀 性能优化：使用 640x640 小图进行画框，而不是全分辨率
                    // 这样可以大幅加速图像处理
                    let encode_start = Instant::now();
                    let display_img = if !boxes.is_empty() {
                        let box_coords: Vec<_> = boxes.iter()
                            .map(|b| {
                                // 将检测框坐标映射到 640x640 空间
                                let scale_x = 640.0 / orig_width as f32;
                                let scale_y = 640.0 / orig_height as f32;
                                (
                                    b.x * scale_x,
                                    b.y * scale_y,
                                    (b.x + b.width) * scale_x,
                                    (b.y + b.height) * scale_y,
                                    b.confidence,
                                    b.class_id
                                )
                            })
                            .collect();
                        Self::draw_boxes(&inference_img, &box_coords)
                    } else {
                        inference_img
                    };
                    
                    // 🚀 性能优化：编码 640x640 小图，而不是全分辨率
                    // 这样可以大幅加速编码和传输
                    let encode_end = Instant::now();
                    if let Ok(encoded) = Self::encode_image_fast(&display_img) {
                        let encode_time = encode_end.elapsed();
                        perf_stats.push_str(&format!(" | 编码: {:.1}ms", encode_time.as_secs_f32() * 1000.0));
                        
                        let frame = DesktopCaptureFrame {
                            session_id: session_id_for_handle.clone(),
                            image: encoded,
                            boxes,
                            width: 640,  // 返回 640x640 的宽度
                            height: 640, // 返回 640x640 的高度
                            fps: current_fps,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                        };
                        
                        let _ = app.emit("desktop-capture-frame", &frame);
                    }
                    
                    // 打印性能统计
                    let total_time = frame_start.elapsed();
                    perf_stats.push_str(&format!(" | 总计: {:.1}ms", total_time.as_secs_f32() * 1000.0));
                    eprintln!("{}", perf_stats);
                    
                    frame_count += 1;
                    
                    // Calculate FPS
                    let now = Instant::now();
                    if now.duration_since(last_fps_time) >= Duration::from_secs(1) {
                        current_fps = frame_count as f32;
                        frame_count = 0;
                        last_fps_time = now;
                        eprintln!("[Desktop] FPS: {}", current_fps);
                    }
                }
                
                // Frame rate limiting
                let elapsed = frame_start.elapsed();
                if elapsed < frame_duration {
                    thread::sleep(frame_duration - elapsed);
                }
            }
        });
        
        sessions.insert(session_id_for_sessions, DesktopSession {
            model_path,
            confidence,
            monitor_idx: monitor as usize,
            fps_limit: fps_limit as f32,
            is_running,
            handle: Some(handle),
        });
        
        eprintln!("[Desktop] Capture started successfully");
        Ok(())
    }
    
    pub async fn stop_capture(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        
        if let Some(mut session) = sessions.remove(session_id) {
            *session.is_running.lock().unwrap() = false;
            
            if let Some(handle) = session.handle.take() {
                handle.join().map_err(|e| format!("Thread join error: {:?}", e))?;
            }
            
            eprintln!("[Desktop] Capture stopped: {}", session_id);
            Ok(())
        } else {
            Err(format!("Session {} not found", session_id))
        }
    }
    
    pub async fn get_active_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.lock().unwrap();
        sessions.keys().cloned().collect()
    }
}

impl Default for DesktopCaptureService {
    fn default() -> Self {
        Self::new()
    }
}
