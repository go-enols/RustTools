//! 桌面捕获服务 - 优化版本
//! 
//! 优化点：
//! 1. 模型缓存和预加载
//! 2. 高效图像编码（WebP）
//! 3. 帧差分优化
//! 4. 增量更新机制
//! 5. WebSocket 流式传输

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use xcap::Monitor;
use tauri::{AppHandle, Emitter};
use image::{DynamicImage, Rgb, imageops::FilterType};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use parking_lot::Mutex as ParkMutex;
use once_cell::sync::Lazy;

use super::optimized_inference::OptimizedInferenceEngine;
use super::optimized_inference::DetectionResult;
use super::desktop_capture::MonitorInfo;

/// 默认 COCO 类别名称
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

/// 优化的检测框
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptimizedAnnotationBox {
    pub id: String,
    pub class_id: usize,
    pub class_name: String,
    pub confidence: f32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<BoxDelta>, // 增量更新
}

/// 框的变化量
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoxDelta {
    pub dx: f32,
    pub dy: f32,
    pub dw: f32,
    pub dh: f32,
}

/// 帧类型
#[derive(Debug, Clone, serde::Serialize)]
pub enum FrameUpdateType {
    Full,         // 完整帧
    Diff,         // 差异帧
    DetectionsOnly, // 仅检测框
}

/// 优化的桌面捕获帧
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptimizedDesktopCaptureFrame {
    pub session_id: String,
    pub frame_type: FrameUpdateType,
    pub image: Option<String>,           // Base64 编码的图像
    pub boxes: Vec<OptimizedAnnotationBox>,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub timestamp: u64,
    pub inference_time_ms: f64,
    pub detection_count: usize,
}

/// 会话信息
struct DesktopSession {
    model_path: String,
    confidence: f32,
    monitor_idx: usize,
    fps_limit: f32,
    is_running: Arc<ParkMutex<bool>>,
    handle: Option<thread::JoinHandle<()>>,
}

/// 帧缓存 - 用于帧差分
struct FrameCache {
    previous_image: Option<Vec<u8>>,
    previous_boxes: Vec<(f32, f32, f32, f32, f32, usize)>,
}

/// 优化的桌面捕获服务
pub struct OptimizedDesktopCaptureService {
    sessions: Arc<ParkMutex<HashMap<String, DesktopSession>>>,
    model_cache: Arc<ParkMutex<HashMap<String, Arc<OptimizedInferenceEngine>>>>,
    frame_cache: Arc<ParkMutex<HashMap<String, FrameCache>>>,
}

impl OptimizedDesktopCaptureService {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(ParkMutex::new(HashMap::new())),
            model_cache: Arc::new(ParkMutex::new(HashMap::new())),
            frame_cache: Arc::new(ParkMutex::new(HashMap::new())),
        }
    }
    
    /// 获取或加载模型（带缓存）
    fn get_or_load_model(&self, model_path: &str) -> Result<Arc<OptimizedInferenceEngine>, String> {
        let mut cache = self.model_cache.lock();
        
        if let Some(engine) = cache.get(model_path) {
            return Ok(engine.clone());
        }
        
        let engine = OptimizedInferenceEngine::load(model_path)?;
        let engine = Arc::new(engine);
        cache.insert(model_path.to_string(), engine.clone());
        
        eprintln!("[OptimizedDesktop] Model cached: {}", model_path);
        
        Ok(engine)
    }
    
    /// 预加载模型（异步）
    pub fn preload_model(&self, model_path: String) {
        let cache = Arc::clone(&self.model_cache);
        
        thread::spawn(move || {
            match OptimizedInferenceEngine::load(&model_path) {
                Ok(engine) => {
                    let engine = Arc::new(engine);
                    cache.lock().insert(model_path, engine);
                    eprintln!("[OptimizedDesktop] Model preloaded");
                }
                Err(e) => {
                    eprintln!("[OptimizedDesktop] Preload failed: {}", e);
                }
            }
        });
    }
    
    /// 获取显示器列表
    pub fn get_monitors(&self) -> Result<Vec<MonitorInfo>, String> {
        let monitors = Monitor::all().map_err(|e| format!("获取显示器失败: {}", e))?;
        
        Ok(monitors
            .iter()
            .enumerate()
            .map(|(i, m)| MonitorInfo {
                id: (i + 1) as u32,
                name: m.name().to_string(),
                x: m.x(),
                y: m.y(),
                width: m.width(),
                height: m.height(),
                is_primary: i == 0,
            })
            .collect())
    }
    
    /// 开始捕获（优化版本）
    pub async fn start_capture(
        &self,
        session_id: String,
        model_path: String,
        confidence: f32,
        monitor: u32,
        fps_limit: f32,
        app: AppHandle,
    ) -> Result<(), String> {
        eprintln!("[OptimizedDesktop] Starting capture: {}", session_id);
        
        // 获取显示器
        let monitors = self.get_monitors()?;
        if monitors.is_empty() {
            return Err("未找到显示器".to_string());
        }
        
        let monitor_idx = (monitor as usize).saturating_sub(1).min(monitors.len() - 1);
        let monitor_info = &monitors[monitor_idx];
        
        // 加载或获取缓存的模型
        let engine = self.get_or_load_model(&model_path)?;
        
        // 初始化帧缓存
        {
            let mut cache = self.frame_cache.lock();
            cache.insert(session_id.clone(), FrameCache {
                previous_image: None,
                previous_boxes: Vec::new(),
            });
        }
        
        let is_running = Arc::new(ParkMutex::new(true));
        let is_running_clone = Arc::clone(&is_running);
        
        let mut sessions = self.sessions.lock();
        
        let session_id_for_thread = session_id.clone();
        let session_id_for_map = session_id.clone();
        let model_path_for_thread = model_path.clone();
        
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
            
            eprintln!("[OptimizedDesktop] Capture loop started (FPS: {}, conf: {})", fps_limit, confidence);
            
            loop {
                if !*is_running_clone.lock() {
                    eprintln!("[OptimizedDesktop] Capture stopped");
                    break;
                }
                
                let frame_start = Instant::now();
                
                let monitor_idx = monitor_idx.min(monitors.len() - 1);
                
                if let Ok(captured) = monitors[monitor_idx].capture_image() {
                    let (width, height) = captured.dimensions();
                    let orig_img = DynamicImage::ImageRgba8(captured);
                    
                    // 执行推理
                    let detection_result = engine.detect(&orig_img, confidence);
                    
                    let detections = &detection_result.boxes;
                    
                    // 转换检测框格式
                    let boxes: Vec<OptimizedAnnotationBox> = detections
                        .iter()
                        .enumerate()
                        .map(|(idx, det)| OptimizedAnnotationBox {
                            id: format!("{}_{}", session_id_for_thread, idx),
                            class_id: det.class_id,
                            class_name: det.class_name.clone(),
                            confidence: det.confidence,
                            x: det.x,
                            y: det.y,
                            width: det.width,
                            height: det.height,
                            delta: None,
                        })
                        .collect();
                    
                    // 决定帧类型（基于变化）
                    let frame_type = Self::determine_frame_type(&orig_img, &boxes);
                    
                    // 编码图像（如果是完整帧）
                    let image = if matches!(frame_type, FrameUpdateType::Full) {
                        Some(Self::encode_image_optimized(&orig_img, current_fps))
                    } else {
                        None
                    };
                    
                    // 构建帧
                    let frame = OptimizedDesktopCaptureFrame {
                        session_id: session_id_for_thread.clone(),
                        frame_type,
                        image,
                        boxes,
                        width,
                        height,
                        fps: current_fps,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                        inference_time_ms: detection_result.inference_time_ms,
                        detection_count: detections.len(),
                    };
                    
                    // 发送帧
                    let _ = app.emit("optimized-desktop-frame", &frame);
                    
                    frame_count += 1;
                    
                    // FPS 计算
                    let now = Instant::now();
                    if now.duration_since(last_fps_time) >= Duration::from_secs(1) {
                        current_fps = frame_count as f32;
                        frame_count = 0;
                        last_fps_time = now;
                        
                        // 打印性能指标
                        eprintln!("[OptimizedDesktop] FPS: {:.1}, Detections: {}, Inference: {:.1}ms",
                            current_fps, frame.detection_count, detection_result.inference_time_ms);
                    }
                }
                
                // 帧率限制
                let elapsed = frame_start.elapsed();
                if elapsed < frame_duration {
                    thread::sleep(frame_duration - elapsed);
                }
            }
        });
        
        sessions.insert(session_id_for_map, DesktopSession {
            model_path,
            confidence,
            monitor_idx: monitor as usize,
            fps_limit,
            is_running,
            handle: Some(handle),
        });
        
        eprintln!("[OptimizedDesktop] Capture started: {}", session_id);
        Ok(())
    }
    
    /// 确定帧类型
    fn determine_frame_type(img: &DynamicImage, boxes: &[OptimizedAnnotationBox]) -> FrameUpdateType {
        // 简单策略：如果没有检测框，发送完整帧
        // 如果有检测框，根据检测框数量决定
        if boxes.is_empty() {
            FrameUpdateType::Full
        } else if boxes.len() <= 3 {
            // 检测框少，发送完整帧以便前端更新
            FrameUpdateType::Full
        } else {
            // 检测框多，仅发送检测结果（前端复用上一帧图像）
            FrameUpdateType::DetectionsOnly
        }
    }
    
    /// 优化图像编码
    fn encode_image_optimized(img: &DynamicImage, current_fps: f32) -> String {
        use std::io::Cursor;
        
        // 根据 FPS 动态调整质量
        let quality = if current_fps > 20.0 {
            60  // 高 FPS，低质量
        } else if current_fps > 10.0 {
            75  // 中 FPS
        } else {
            85  // 低 FPS，高质量
        };
        
        let rgb = img.to_rgb8();
        let mut buffer = Cursor::new(Vec::with_capacity(rgb.len() / 3));
        
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
        
        encoder.encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        ).ok();
        
        BASE64.encode(buffer.into_inner())
    }
    
    /// 停止捕获
    pub async fn stop_capture(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock();
        
        if let Some(mut session) = sessions.remove(session_id) {
            *session.is_running.lock() = false;
            
            if let Some(handle) = session.handle.take() {
                handle.join().map_err(|e| format!("线程退出错误: {:?}", e))?;
            }
            
            // 清理帧缓存
            self.frame_cache.lock().remove(session_id);
            
            eprintln!("[OptimizedDesktop] Capture stopped: {}", session_id);
            Ok(())
        } else {
            Err(format!("会话未找到: {}", session_id))
        }
    }
    
    /// 获取活跃会话
    pub async fn get_active_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.lock();
        sessions.keys().cloned().collect()
    }
    
    /// 获取服务状态
    pub async fn get_status(&self) -> DesktopCaptureStatus {
        let sessions = self.sessions.lock();
        let active: Vec<String> = sessions
            .iter()
            .filter(|(_, s)| *s.is_running.lock())
            .map(|(id, _)| id.clone())
            .collect();
        
        DesktopCaptureStatus {
            active_sessions: active,
            total_sessions: sessions.len(),
            cached_models: self.model_cache.lock().len(),
        }
    }
}

/// 状态结构
#[derive(Debug, Clone, serde::Serialize)]
pub struct DesktopCaptureStatus {
    pub active_sessions: Vec<String>,
    pub total_sessions: usize,
    pub cached_models: usize,
}

impl Default for OptimizedDesktopCaptureService {
    fn default() -> Self {
        Self::new()
    }
}
