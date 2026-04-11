//! Desktop Capture Service - Performance Analysis Version
//! 
//! 这个版本添加了详细的性能计时,用于分析瓶颈
//! 运行后查看终端输出中的 [PERF] 日志

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

/// Annotation box
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

/// Desktop capture frame
#[derive(Debug, Clone, serde::Serialize)]
pub struct DesktopCaptureFrame {
    pub session_id: String,
    pub image: String,
    pub boxes: Vec<AnnotationBox>,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub timestamp: u64,
}

/// Desktop session info
struct DesktopSession {
    model_path: String,
    confidence: f32,
    monitor_idx: usize,
    fps_limit: f32,
    is_running: Arc<Mutex<bool>>,
    handle: Option<thread::JoinHandle<()>>,
}

/// Desktop capture service
pub struct DesktopCaptureService {
    sessions: Arc<Mutex<HashMap<String, DesktopSession>>>,
}

impl DesktopCaptureService {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Get available monitors
    pub fn get_monitors(&self) -> Result<Vec<MonitorInfo>, String> {
        let monitors = Monitor::all().map_err(|e| format!("Failed to get monitors: {}", e))?;
        
        Ok(monitors
            .into_iter()
            .enumerate()
            .map(|(idx, m)| MonitorInfo {
                id: idx as u32,
                name: m.name(),
                x: 0,
                y: 0,
                width: m.width(),
                height: m.height(),
                is_primary: idx == 0,
            })
            .collect())
    }
    
    /// Load YOLO ONNX model
    fn load_yolo_model(model_path: &str) -> Result<TractModel, String> {
        let model = std::fs::read(model_path)
            .map_err(|e| format!("Failed to read model: {}", e))?;
        
        let tract_model = tract_onnx::onnx()
            .model_for_read(&mut std::io::Cursor::new(&model))
            .map_err(|e| format!("Failed to load model: {}", e))?
            .into_runnable()
            .map_err(|e| format!("Failed to build model: {}", e))?;
        
        Ok(tract_model)
    }
    
    /// Fast image preprocessing (optimized for YOLO)
    fn preprocess_image_fast_fixed(img: &DynamicImage) -> Result<Tensor, String> {
        let rgb = img.resize(640, 640, FilterType::Nearest)
            .to_rgb8();
        
        let data: Vec<f32> = rgb.as_raw()
            .iter()
            .flat_map(|&p| {
                // Normalize to [0, 1] and convert RGB to BGR (for models trained on BGR)
                let r = p as f32 / 255.0;
                let g = p as f32 / 255.0;
                let b = p as f32 / 255.0;
                vec![b, g, r] // BGR format
            })
            .collect();
        
        Tensor::from_shape(&[1, 3, 640, 640], &data)
            .map_err(|e| format!("Failed to create tensor: {}", e))
    }
    
    /// Run YOLO inference (performance analysis version)
    /// 
    /// 性能计时点:
    /// 1. 模型推理 (tract)
    /// 2. 后处理 (找最大类、NMS)
    fn run_inference(
        model: &TractModel,
        img: &DynamicImage,
        confidence: f32,
        orig_width: u32,
        orig_height: u32,
    ) -> Result<Vec<(f32, f32, f32, f32, f32, usize)>, String> {
        // Preprocessing
        let preprocess_start = Instant::now();
        let input = Self::preprocess_image_fast_fixed(img)?;
        let preprocess_time = preprocess_start.elapsed();
        eprintln!("[PERF-Inference] 预处理: {:.2}ms", preprocess_time.as_secs_f32() * 1000.0);
        
        // Inference
        let inference_start = Instant::now();
        let result = model.run(tvec![input.into()])
            .map_err(|e| format!("Inference failed: {}", e))?;
        let inference_time = inference_start.elapsed();
        eprintln!("[PERF-Inference] 模型推理: {:.2}ms", inference_time.as_secs_f32() * 1000.0);
        
        let output = &result[0];
        let shape = output.shape();
        
        if shape.len() != 3 {
            return Err(format!("Unexpected output shape: {:?}", shape));
        }
        
        // YOLOv8 格式: [batch, features, boxes]
        let batch_size = shape[0] as usize;
        let num_features = shape[1] as usize;
        let num_boxes = shape[2] as usize;
        
        if batch_size != 1 {
            return Err(format!("Expected batch size 1, got {}", batch_size));
        }
        
        let num_classes = if num_features > 4 {
            num_features - 4
        } else {
            return Err(format!("Invalid output shape: expected 84 features, got {}", num_features));
        };
        
        let scale_x = orig_width as f32 / 640.0;
        let scale_y = orig_height as f32 / 640.0;
        
        let output_data = output.to_array_view::<f32>()
            .map_err(|e| format!("Failed to access output: {}", e))?;
        
        let mut detections = Vec::with_capacity(100);
        
        // Post-processing (find max class + NMS)
        let postprocess_start = Instant::now();
        for i in 0..num_boxes {
            let mut max_score = 0.0f32;
            let mut max_class = 0usize;
            
            // 找最大类别分数
            for c in 0..num_classes {
                let score = output_data[[0, c + 4, i]];
                if score > max_score {
                    max_score = score;
                    max_class = c;
                }
            }
            
            // 置信度阈值过滤
            if max_score >= confidence {
                let cx = output_data[[0, 0, i]];
                let cy = output_data[[0, 1, i]];
                let w = output_data[[0, 2, i]];
                let h = output_data[[0, 3, i]];
                
                // 转换为绝对坐标 (YOLOv8 已经是绝对坐标 0-640)
                let x1 = (cx - w / 2.0).max(0.0) * scale_x;
                let y1 = (cy - h / 2.0).max(0.0) * scale_y;
                let x2 = (cx + w / 2.0).min(640.0) * scale_x;
                let y2 = (cy + h / 2.0).min(640.0) * scale_y;
                
                detections.push((x1, y1, x2, y2, max_score, max_class));
            }
        }
        let postprocess_time = postprocess_start.elapsed();
        eprintln!("[PERF-Inference] 后处理 (找类+NMS过滤): {:.2}ms (初始检测数: {})", 
            postprocess_time.as_secs_f32() * 1000.0, detections.len());
        
        // NMS
        let nms_start = Instant::now();
        let result = Self::nms(detections, 0.45);
        let nms_time = nms_start.elapsed();
        eprintln!("[PERF-Inference] NMS: {:.2}ms (最终检测数: {})", 
            nms_time.as_secs_f32() * 1000.0, result.len());
        
        let total_time = preprocess_time + inference_time + postprocess_time + nms_time;
        eprintln!("[PERF-Inference] 总计: {:.2}ms", total_time.as_secs_f32() * 1000.0);
        
        Ok(result)
    }
    
    /// Non-Maximum Suppression
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
    
    /// Fast JPEG encoding with quality control
    fn encode_image_fast(img: &DynamicImage) -> Result<String, String> {
        use std::io::Cursor;
        
        let encode_start = Instant::now();
        
        // Resize if needed
        let resize_start = Instant::now();
        let rgb = if img.width() > 640 || img.height() > 640 {
            img.resize(640, 640, FilterType::Nearest)
                .to_rgb8()
        } else {
            img.to_rgb8()
        };
        let resize_time = resize_start.elapsed();
        
        // JPEG encoding
        let jpeg_start = Instant::now();
        let quality = 40;
        let mut buffer = Cursor::new(Vec::with_capacity(rgb.len() / 4));
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
        encoder.encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        ).map_err(|e| format!("Failed to encode: {}", e))?;
        let jpeg_time = jpeg_start.elapsed();
        let jpeg_size = buffer.get_ref().len();
        
        // Base64 encoding
        let base64_start = Instant::now();
        let encoded = BASE64.encode(buffer.into_inner());
        let base64_time = base64_start.elapsed();
        let base64_size = encoded.len();
        
        let total_encode_time = encode_start.elapsed();
        eprintln!("[PERF-Encode] Resize: {:.2}ms | JPEG: {:.2}ms ({:.1}KB) | Base64: {:.2}ms ({:.1}KB) | 总计: {:.2}ms",
            resize_time.as_secs_f32() * 1000.0,
            jpeg_time.as_secs_f32() * 1000.0,
            jpeg_size as f32 / 1024.0,
            base64_time.as_secs_f32() * 1000.0,
            base64_size as f32 / 1024.0,
            total_encode_time.as_secs_f32() * 1000.0
        );
        
        Ok(encoded)
    }
    
    /// Draw detection boxes
    fn draw_boxes(img: &DynamicImage, boxes: &[(f32, f32, f32, f32, f32, usize)]) -> DynamicImage {
        let draw_start = Instant::now();
        
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
            
            // Draw rectangle
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
        
        let draw_time = draw_start.elapsed();
        eprintln!("[PERF-Draw] 画 {} 个框: {:.2}ms", boxes.len(), draw_time.as_secs_f32() * 1000.0);
        
        DynamicImage::ImageRgb8(rgb)
    }
    
    /// Start capture with performance analysis
    pub async fn start_capture(
        &self,
        session_id: String,
        model_path: String,
        confidence: f32,
        monitor: u32,
        fps_limit: u32,
        app: AppHandle,
    ) -> Result<(), String> {
        eprintln!("[Desktop] Starting capture with performance analysis for session: {}", session_id);
        
        let monitors = self.get_monitors()?;
        if monitors.is_empty() {
            return Err("No monitors found".to_string());
        }
        
        let monitor_idx = (monitor as usize).saturating_sub(1).min(monitors.len() - 1);
        let monitor_info = &monitors[monitor_idx];
        
        eprintln!("[Desktop] Monitor: {} ({}x{})", monitor_info.name, monitor_info.width, monitor_info.height);
        
        // Load model once before the loop
        let yolo_model = if !model_path.is_empty() {
            let model_load_start = Instant::now();
            match Self::load_yolo_model(&model_path) {
                Ok(model) => {
                    let model_load_time = model_load_start.elapsed();
                    eprintln!("[Desktop] ✅ YOLO model loaded in {:.2}ms", model_load_time.as_secs_f32() * 1000.0);
                    Some(model)
                }
                Err(e) => {
                    eprintln!("[Desktop] ❌ Failed to load YOLO model: {}", e);
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
        
        let session_id_for_handle = session_id.clone();
        let session_id_for_sessions = session_id.clone();
        
        let handle = thread::spawn(move || {
            // 限制最大帧率为 15 FPS
            let target_fps = (fps_limit as f32).min(15.0);
            let frame_duration = Duration::from_secs_f64(1.0 / target_fps as f64);
            
            let mut frame_count = 0u32;
            let mut last_fps_time = Instant::now();
            let mut current_fps = 0.0f32;
            
            eprintln!("\n========== 性能分析开始 ==========");
            eprintln!("目标帧率: {} FPS", target_fps);
            eprintln!("置信度阈值: {}", confidence);
            eprintln!("===================================\n");
            
            // Get monitor list once
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
                
                let frame_start = Instant::now();
                eprintln!("\n[PERF-Frame] ===== 第 {} 帧 =====", frame_count + 1);
                
                // Ensure monitor index is valid
                let monitor_idx = monitor_idx.min(monitors.len() - 1);
                
                // 1. Screen Capture
                let capture_start = Instant::now();
                if let Ok(captured) = monitors[monitor_idx].capture_image() {
                    let capture_time = capture_start.elapsed();
                    let (orig_width, orig_height) = captured.dimensions();
                    eprintln!("[PERF-Capture] 屏幕捕获: {:.2}ms (分辨率: {}x{})", 
                        capture_time.as_secs_f32() * 1000.0, orig_width, orig_height);
                    
                    let orig_img = DynamicImage::ImageRgba8(captured);
                    
                    // 2. Resize
                    let resize_start = Instant::now();
                    let inference_img = orig_img.resize_exact(
                        640u32, 
                        640u32, 
                        FilterType::Nearest
                    );
                    let resize_time = resize_start.elapsed();
                    eprintln!("[PERF-Resize] Resize到640x640: {:.2}ms", resize_time.as_secs_f32() * 1000.0);
                    
                    // 3. Run inference
                    let inference_start = Instant::now();
                    let boxes = if let Some(ref model) = yolo_model {
                        match Self::run_inference(model, &inference_img, confidence, orig_width, orig_height) {
                            Ok(detections) => {
                                if !detections.is_empty() {
                                    eprintln!("[Desktop] 检测到 {} 个目标:", detections.len());
                                    for (x1, y1, x2, y2, conf, class_id) in &detections {
                                        eprintln!("  - Class {} at ({:.0}, {:.0}, {:.0}, {:.0}) conf {:.2}", 
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
                    eprintln!("[PERF-Inference] 推理总耗时: {:.2}ms", inference_time.as_secs_f32() * 1000.0);
                    
                    // 4. Draw boxes
                    let display_img = if !boxes.is_empty() {
                        let box_coords: Vec<_> = boxes.iter()
                            .map(|b| {
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
                    
                    // 5. Encode
                    if let Ok(encoded) = Self::encode_image_fast(&display_img) {
                        let frame = DesktopCaptureFrame {
                            session_id: session_id_for_handle.clone(),
                            image: encoded,
                            boxes,
                            width: 640,
                            height: 640,
                            fps: current_fps,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                        };
                        
                        // 6. Emit frame
                        let emit_start = Instant::now();
                        let _ = app.emit("desktop-capture-frame", &frame);
                        let emit_time = emit_start.elapsed();
                        
                        let total_time = frame_start.elapsed();
                        eprintln!("[PERF-Emit] 发送帧: {:.2}ms", emit_time.as_secs_f32() * 1000.0);
                        eprintln!("[PERF-Frame] ========== 第 {} 帧总计: {:.2}ms ==========", 
                            frame_count + 1, total_time.as_secs_f32() * 1000.0);
                    }
                    
                    frame_count += 1;
                    
                    // Calculate FPS
                    let now = Instant::now();
                    if now.duration_since(last_fps_time) >= Duration::from_secs(1) {
                        current_fps = frame_count as f32;
                        frame_count = 0;
                        last_fps_time = now;
                        eprintln!("\n[Desktop] 📊 实际 FPS: {} | 帧处理时间: {:.2}ms", 
                            current_fps, 1000.0 / current_fps);
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
    
    /// Stop capture
    pub fn stop_capture(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        
        if let Some(session) = sessions.get_mut(session_id) {
            *session.is_running.lock().unwrap() = false;
            
            if let Some(handle) = session.handle.take() {
                handle.join().map_err(|e| format!("Thread join error: {:?}", e))?;
            }
            
            sessions.remove(session_id);
            Ok(())
        } else {
            Err(format!("Session not found: {}", session_id))
        }
    }
}
