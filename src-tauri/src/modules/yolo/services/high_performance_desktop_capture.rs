//! 高性能桌面捕获服务 - 优化版本
//! 
//! 优化要点：
//! 1. 仅传输检测框，不传输完整图像
//! 2. 前端使用本地预览 + 检测框叠加
//! 3. 自适应帧率控制
//! 4. 预加载模型避免重复加载

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

use super::high_performance_inference::HighPerformanceInferenceEngine;

/// 非洲野生动物类别
const WILDLIFE_CLASS_NAMES: [&str; 4] = [
    "elephant", "buffalo", "rhino", "zebra"
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

/// 捕获状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct CaptureStatus {
    pub fps: f32,
    pub inference_time_ms: f64,
    pub frame_count: u32,
}

/// 桌面捕获帧（仅检测框）
#[derive(Debug, Clone, serde::Serialize)]
pub struct DetectionFrame {
    pub session_id: String,
    pub boxes: Vec<AnnotationBox>,
    pub fps: f32,
    pub inference_time_ms: f64,
    pub width: u32,
    pub height: u32,
    pub timestamp: u64,
}

/// 桌面捕获配置
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub model_path: String,
    pub confidence: f32,
    pub monitor_idx: usize,
    pub fps_limit: u32,
    pub use_optimized_inference: bool,
}

/// 捕获会话
struct CaptureSession {
    config: CaptureConfig,
    is_running: Arc<ParkMutex<bool>>,
    handle: Option<thread::JoinHandle<()>>,
}

/// 高性能桌面捕获服务
pub struct HighPerformanceDesktopCaptureService {
    sessions: Arc<ParkMutex<HashMap<String, CaptureSession>>>,
    model_cache: Arc<ParkMutex<HashMap<String, Arc<HighPerformanceInferenceEngine>>>>,
}

impl HighPerformanceDesktopCaptureService {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(ParkMutex::new(HashMap::new())),
            model_cache: Arc::new(ParkMutex::new(HashMap::new())),
        }
    }
    
    /// 获取显示器列表
    pub fn get_monitors(&self) -> Result<Vec<MonitorInfo>, String> {
        let monitors = Monitor::all()
            .map_err(|e| format!("获取显示器失败: {}", e))?;
        
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
    
    /// 获取或加载模型
    fn get_or_load_model(&self, model_path: &str) -> Result<Arc<HighPerformanceInferenceEngine>, String> {
        let mut cache = self.model_cache.lock();
        
        if let Some(engine) = cache.get(model_path) {
            eprintln!("[HPDesktop] 模型缓存命中: {}", model_path);
            return Ok(Arc::clone(engine));
        }
        
        eprintln!("[HPDesktop] 加载模型: {}", model_path);
        let start = Instant::now();
        
        let engine = HighPerformanceInferenceEngine::load(model_path)?;
        let engine = Arc::new(engine);
        
        eprintln!("[HPDesktop] 模型加载完成: {:.2}s", start.elapsed().as_secs_f64());
        
        cache.insert(model_path.to_string(), Arc::clone(&engine));
        Ok(engine)
    }
    
    /// 预加载模型
    pub fn preload_model(&self, model_path: &str) {
        let path = model_path.to_string();
        let cache = Arc::clone(&self.model_cache);
        
        thread::spawn(move || {
            match HighPerformanceInferenceEngine::load(&path) {
                Ok(engine) => {
                    let mut cache = cache.lock();
                    cache.insert(path, Arc::new(engine));
                    eprintln!("[HPDesktop] 模型预加载完成");
                }
                Err(e) => {
                    eprintln!("[HPDesktop] 模型预加载失败: {}", e);
                }
            }
        });
    }
    
    /// 开始捕获
    pub async fn start_capture(
        &self,
        session_id: String,
        config: CaptureConfig,
        app: AppHandle,
    ) -> Result<(), String> {
        eprintln!("[HPDesktop] 启动捕获: {}", session_id);
        eprintln!("[HPDesktop] 模型: {}", config.model_path);
        eprintln!("[HPDesktop] 置信度: {}", config.confidence);
        eprintln!("[HPDesktop] FPS限制: {}", config.fps_limit);
        
        // 获取显示器
        let monitors = self.get_monitors()?;
        if monitors.is_empty() {
            return Err("未找到显示器".to_string());
        }
        
        let monitor_idx = config.monitor_idx.min(monitors.len() - 1);
        let monitor_info = &monitors[monitor_idx];
        
        eprintln!("[HPDesktop] 显示器: {} ({}x{})", 
            monitor_info.name, monitor_info.width, monitor_info.height);
        
        // 加载模型
        let engine = self.get_or_load_model(&config.model_path)?;
        
        let is_running = Arc::new(ParkMutex::new(true));
        let is_running_clone = Arc::clone(&is_running);
        
        // 创建会话
        {
            let mut sessions = self.sessions.lock();
            sessions.insert(session_id.clone(), CaptureSession {
                config: config.clone(),
                is_running: Arc::clone(&is_running),
                handle: None,
            });
        }
        
        // 启动捕获线程
        let session_id_clone = session_id.clone();
        let engine_clone = Arc::clone(&engine);
        
        let handle = thread::spawn(move || {
            let frame_duration = Duration::from_secs_f64(1.0 / config.fps_limit as f64);
            let mut frame_count = 0u32;
            let mut last_fps_time = Instant::now();
            let mut current_fps = 0.0f32;
            let mut total_inference_time = 0.0f64;
            
            // 获取显示器列表（避免重复调用）
            let monitors = Monitor::all().unwrap_or_default();
            if monitors.is_empty() {
                eprintln!("[HPDesktop] 无可用显示器");
                return;
            }
            
            let monitor = &monitors[monitor_idx.min(monitors.len() - 1)];
            
            loop {
                // 检查是否停止
                if !*is_running_clone.lock() {
                    eprintln!("[HPDesktop] 捕获已停止");
                    break;
                }
                
                let frame_start = Instant::now();
                
                // 捕获屏幕
                if let Ok(captured) = monitor.capture_image() {
                    let (width, height) = captured.dimensions();
                    let orig_img = DynamicImage::ImageRgba8(captured);
                    
                    // 推理
                    let result = engine_clone.detect(&orig_img, config.confidence);
                    
                    total_inference_time += result.inference_time_ms;
                    frame_count += 1;
                    
                    // 转换检测框
                    let boxes: Vec<AnnotationBox> = result.boxes
                        .into_iter()
                        .enumerate()
                        .map(|(idx, b)| {
                            let class_name = if b.class_id < WILDLIFE_CLASS_NAMES.len() {
                                WILDLIFE_CLASS_NAMES[b.class_id].to_string()
                            } else {
                                b.class_name
                            };
                            
                            AnnotationBox {
                                id: format!("{}_{}", session_id_clone, idx),
                                class_id: b.class_id,
                                class_name,
                                confidence: b.confidence,
                                x: b.x,
                                y: b.y,
                                width: b.width,
                                height: b.height,
                            }
                        })
                        .collect();
                    
                    // 计算 FPS
                    let now = Instant::now();
                    if now.duration_since(last_fps_time) >= Duration::from_secs(1) {
                        current_fps = frame_count as f32;
                        frame_count = 0;
                        last_fps_time = now;
                    }
                    
                    // 发送检测结果（仅检测框）
                    let frame = DetectionFrame {
                        session_id: session_id_clone.clone(),
                        boxes,
                        fps: current_fps,
                        inference_time_ms: result.inference_time_ms,
                        width,
                        height,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                    };
                    
                    let _ = app.emit("hp-desktop-frame", &frame);
                    
                    // 帧率控制
                    let elapsed = frame_start.elapsed();
                    if elapsed < frame_duration {
                        thread::sleep(frame_duration - elapsed);
                    }
                }
            }
            
            // 统计
            if frame_count > 0 {
                let avg_inference = total_inference_time / frame_count as f64;
                eprintln!("[HPDesktop] 平均推理时间: {:.2}ms", avg_inference);
            }
        });
        
        // 更新会话
        {
            let mut sessions = self.sessions.lock();
            if let Some(session) = sessions.get_mut(&session_id) {
                session.handle = Some(handle);
            }
        }
        
        eprintln!("[HPDesktop] 捕获启动成功");
        Ok(())
    }
    
    /// 停止捕获
    pub async fn stop_capture(&self, session_id: &str) -> Result<(), String> {
        eprintln!("[HPDesktop] 停止捕获: {}", session_id);
        
        let mut sessions = self.sessions.lock();
        
        if let Some(mut session) = sessions.remove(session_id) {
            *session.is_running.lock() = false;
            
            if let Some(handle) = session.handle.take() {
                handle.join().map_err(|e| format!("线程退出失败: {:?}", e))?;
            }
            
            eprintln!("[HPDesktop] 捕获已停止: {}", session_id);
            Ok(())
        } else {
            Err(format!("会话不存在: {}", session_id))
        }
    }
    
    /// 获取捕获状态
    pub fn get_status(&self) -> Vec<String> {
        let sessions = self.sessions.lock();
        sessions
            .keys()
            .cloned()
            .collect()
    }
    
    /// 获取活动会话数
    pub fn active_sessions(&self) -> usize {
        let sessions = self.sessions.lock();
        sessions.values()
            .filter(|s| *s.is_running.lock())
            .count()
    }
}

impl Default for HighPerformanceDesktopCaptureService {
    fn default() -> Self {
        Self::new()
    }
}

/// 捕获配置（从前端接收）
#[derive(Debug, serde::Deserialize)]
pub struct CaptureConfigFrontend {
    pub model_path: String,
    pub confidence: f32,
    pub monitor: u32,
    pub fps_limit: u32,
}

impl From<CaptureConfigFrontend> for CaptureConfig {
    fn from(frontend: CaptureConfigFrontend) -> Self {
        Self {
            model_path: frontend.model_path,
            confidence: frontend.confidence,
            monitor_idx: (frontend.monitor as usize).saturating_sub(1),
            fps_limit: frontend.fps_limit,
            use_optimized_inference: true,
        }
    }
}
