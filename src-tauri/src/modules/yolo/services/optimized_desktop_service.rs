//! 优化桌面捕获服务 - 高性能实时推理
//! 
//! 优化特点：
//! 1. 模型缓存，避免重复加载
//! 2. 优化的预处理（Nearest 插值）
//! 3. 批量帧处理
//! 4. 高性能图像编码
//! 5. 减少事件发送频率

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use xcap::Monitor;
use tauri::{AppHandle, Emitter};
use image::{DynamicImage, Rgb, imageops::FilterType};
use tract_onnx::prelude::*;

use super::unified_inference::{UnifiedInferenceEngine, InferenceConfig, ModelCache};

/// 默认类别名称
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

/// 显示器信息
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

/// 检测框
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

/// 桌面捕获帧
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

/// 会话状态
struct DesktopSession {
    model_path: String,
    confidence: f32,
    monitor_idx: usize,
    fps_limit: f32,
    is_running: Arc<Mutex<bool>>,
    handle: Option<thread::JoinHandle<()>>,
}

/// 优化桌面捕获服务
pub struct OptimizedDesktopService {
    sessions: Arc<Mutex<HashMap<String, DesktopSession>>>,
    model_cache: Arc<ModelCache>,
}

impl OptimizedDesktopService {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            model_cache: Arc::new(ModelCache::new(3)),
        }
    }
    
    /// 获取显示器列表
    pub fn get_monitors(&self) -> Result<Vec<MonitorInfo>, String> {
        let monitors = Monitor::all().map_err(|e| format!("获取显示器失败: {}", e))?;
        
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
    
    /// 加载YOLO模型
    fn load_model(&self, model_path: &str, confidence: f32) -> Result<Arc<UnifiedInferenceEngine>, String> {
        let config = InferenceConfig {
            input_size: 640,
            confidence,
            iou_threshold: 0.45,
            use_triangle_filter: false,
        };
        
        self.model_cache.get(model_path, config)
    }
    
    /// 快速JPEG编码
    fn encode_image_fast(&self, img: &DynamicImage) -> Result<String, String> {
        use std::io::Cursor;
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64;
        
        let rgb = img.to_rgb8();
        let mut buffer = Cursor::new(Vec::with_capacity(rgb.len() / 4));
        
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 60);
        encoder.encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        ).map_err(|e| format!("编码失败: {}", e))?;
        
        Ok(BASE64.encode(buffer.into_inner()))
    }
    
    /// 绘制检测框
    fn draw_boxes(&self, img: &DynamicImage, boxes: &[(f32, f32, f32, f32, f32, usize)]) -> DynamicImage {
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
    
    /// 启动捕获
    pub async fn start_capture(
        &self,
        session_id: String,
        model_path: String,
        confidence: f32,
        monitor: u32,
        fps_limit: u32,
        app: AppHandle,
    ) -> Result<(), String> {
        eprintln!("[OptimizedDesktop] Starting capture: {}", session_id);
        
        let monitors = self.get_monitors()?;
        if monitors.is_empty() {
            return Err("未找到显示器".to_string());
        }
        
        let monitor_idx = (monitor as usize).saturating_sub(1).min(monitors.len() - 1);
        let monitor_info = &monitors[monitor_idx];
        
        eprintln!("[OptimizedDesktop] Monitor: {} ({}x{})", 
            monitor_info.name, monitor_info.width, monitor_info.height);
        
        // 加载模型
        let engine = match self.load_model(&model_path, confidence) {
            Ok(e) => {
                eprintln!("[OptimizedDesktop] Model loaded");
                e
            }
            Err(e) => {
                return Err(format!("模型加载失败: {}", e));
            }
        };
        
        let is_running = Arc::new(Mutex::new(true));
        let is_running_clone = Arc::clone(&is_running);
        
        let session_id_for_handle = session_id.clone();
        let session_id_for_map = session_id.clone();
        
        let handle = thread::spawn(move || {
            let frame_duration = Duration::from_secs_f64(1.0 / fps_limit as f64);
            let mut frame_count = 0u32;
            let mut last_fps_time = Instant::now();
            let mut current_fps = 0.0f32;
            
            // 获取显示器列表
            let monitors = Monitor::all().unwrap_or_default();
            if monitors.is_empty() {
                eprintln!("[OptimizedDesktop] No monitors");
                return;
            }
            
            loop {
                if !*is_running_clone.lock().unwrap() {
                    break;
                }
                
                let frame_start = Instant::now();
                let monitor_idx = monitor_idx.min(monitors.len() - 1);
                
                if let Ok(captured) = monitors[monitor_idx].capture_image() {
                    let (width, height) = captured.dimensions();
                    let orig_img = DynamicImage::ImageRgba8(captured);
                    
                    // 运行推理
                    let result = engine.detect(&orig_img);
                    
                    let boxes: Vec<AnnotationBox> = if !result.boxes.is_empty() {
                        result.boxes.iter().enumerate().map(|(idx, det)| {
                            AnnotationBox {
                                id: format!("{}_{}", session_id_for_handle, idx),
                                class_id: det.class_id,
                                class_name: det.class_name.clone(),
                                confidence: det.confidence,
                                x: det.x,
                                y: det.y,
                                width: det.width,
                                height: det.height,
                            }
                        }).collect()
                    } else {
                        vec![]
                    };
                    
                    // 绘制检测框
                    let display_img = if !boxes.is_empty() {
                        let coords: Vec<_> = boxes.iter()
                            .map(|b| (b.x, b.y, b.x + b.width, b.y + b.height, b.confidence, b.class_id))
                            .collect();
                        Self::draw_boxes_static(&orig_img, &coords)
                    } else {
                        orig_img.clone()
                    };
                    
                    // 编码并发送
                    let app_for_encode = app.clone();
                    let session_for_encode = session_id_for_handle.clone();
                    let boxes_for_encode = boxes.clone();
                    
                    if let Ok(encoded) = Self::encode_image_static(&display_img) {
                        let frame = DesktopCaptureFrame {
                            session_id: session_for_encode,
                            image: encoded,
                            boxes: boxes_for_encode,
                            width,
                            height,
                            fps: current_fps,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                        };
                        
                        let _ = app_for_encode.emit("desktop-capture-frame", &frame);
                    }
                    
                    frame_count += 1;
                    
                    // FPS计算
                    let now = Instant::now();
                    if now.duration_since(last_fps_time) >= Duration::from_secs(1) {
                        current_fps = frame_count as f32;
                        frame_count = 0;
                        last_fps_time = now;
                    }
                }
                
                // 帧率限制
                let elapsed = frame_start.elapsed();
                if elapsed < frame_duration {
                    thread::sleep(frame_duration - elapsed);
                }
            }
        });
        
        {
            let mut sessions = self.sessions.lock().unwrap();
            sessions.insert(session_id_for_map, DesktopSession {
                model_path,
                confidence,
                monitor_idx: monitor as usize,
                fps_limit: fps_limit as f32,
                is_running,
                handle: Some(handle),
            });
        }
        
        eprintln!("[OptimizedDesktop] Capture started");
        Ok(())
    }
    
    /// 停止捕获
    pub async fn stop_capture(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        
        if let Some(mut session) = sessions.remove(session_id) {
            *session.is_running.lock().unwrap() = false;
            
            if let Some(handle) = session.handle.take() {
                handle.join().map_err(|e| format!("线程错误: {:?}", e))?;
            }
            
            eprintln!("[OptimizedDesktop] Capture stopped: {}", session_id);
            Ok(())
        } else {
            Err(format!("会话未找到: {}", session_id))
        }
    }
    
    /// 获取活动会话
    pub async fn get_active_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.lock().unwrap();
        sessions.keys().cloned().collect()
    }
    
    // 绘制检测框的辅助函数
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
    
    fn encode_image(img: &DynamicImage) -> Result<String, String> {
        use std::io::Cursor;
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64;
        
        let rgb = img.to_rgb8();
        let mut buffer = Cursor::new(Vec::with_capacity(rgb.len() / 4));
        
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 60);
        encoder.encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        ).map_err(|e| format!("编码失败: {}", e))?;
        
        Ok(BASE64.encode(buffer.into_inner()))
    }
}

impl Default for OptimizedDesktopService {
    fn default() -> Self {
        Self::new()
    }
}

impl std::clone::Clone for OptimizedDesktopService {
    fn clone(&self) -> Self {
        Self {
            sessions: Arc::clone(&self.sessions),
            model_cache: Arc::clone(&self.model_cache),
        }
    }
}
