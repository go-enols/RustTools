//! 优化后的桌面捕获服务
//! 
//! 使用 xcap 进行屏幕捕获

use std::time::{Duration, Instant};
use std::sync::Arc;

use image::{DynamicImage, RgbaImage};

/// 检测框
#[derive(Debug, Clone)]
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
    pub target_fps: u32,
    pub input_size: usize,
    pub inference_interval: u32,
    pub confidence_threshold: f32,
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

/// 捕获会话
pub struct CaptureSession {
    config: CaptureConfig,
}

impl CaptureSession {
    pub fn new(config: CaptureConfig) -> Self {
        Self { config }
    }
    
    pub async fn start(&mut self, monitor_index: usize) -> Result<(), String> {
        use xcap::Monitor;
        
        let monitors = Monitor::all()
            .map_err(|e| format!("Failed to get monitors: {}", e))?;
        
        if monitor_index >= monitors.len() {
            return Err(format!("Monitor {} not found", monitor_index));
        }
        
        let monitor = &monitors[monitor_index];
        let width = monitor.width();
        let height = monitor.height();
        
        eprintln!("[CaptureSession] Capturing {}x{} at {} FPS", width, height, self.config.target_fps);
        
        let frame_interval = Duration::from_micros(1_000_000 / self.config.target_fps as u64);
        let mut frame_count: u64 = 0;
        let mut last_fps_time = Instant::now();
        let mut frames_since_last = 0u32;
        
        loop {
            let start = Instant::now();
            
            // 捕获帧
            let image_data = monitor.capture_image()
                .map_err(|e| format!("Capture failed: {}", e))?;
            
            let capture_time = start.elapsed();
            
            frame_count += 1;
            
            // 帧跳过逻辑
            if frame_count % self.config.inference_interval as u64 != 0 {
                frames_since_last += 1;
                
                if last_fps_time.elapsed() >= Duration::from_secs(1) {
                    let fps = frames_since_last as f32 / last_fps_time.elapsed().as_secs_f32();
                    eprintln!(
                        "[PERF] FPS: {:.1}, Capture: {:.1}ms",
                        fps,
                        capture_time.as_secs_f64() * 1000.0
                    );
                    frames_since_last = 0;
                    last_fps_time = Instant::now();
                }
                
                // 帧率限制
                if start.elapsed() < frame_interval {
                    tokio::time::sleep(frame_interval - start.elapsed()).await;
                }
                continue;
            }
            
            // TODO: 在这里集成推理引擎
            // 转换为 DynamicImage: DynamicImage::ImageRgba8(RgbaImage::from_raw(width, height, image_data)?)
            
            frames_since_last += 1;
            
            // 更新统计
            if last_fps_time.elapsed() >= Duration::from_secs(1) {
                let fps = frames_since_last as f32 / last_fps_time.elapsed().as_secs_f32();
                
                eprintln!(
                    "[PERF] FPS: {:.1}, Capture: {:.1}ms, Detections: 0",
                    fps,
                    capture_time.as_secs_f64() * 1000.0
                );
                
                frames_since_last = 0;
                last_fps_time = Instant::now();
            }
            
            // 帧率限制
            if start.elapsed() < frame_interval {
                tokio::time::sleep(frame_interval - start.elapsed()).await;
            }
        }
    }
}
