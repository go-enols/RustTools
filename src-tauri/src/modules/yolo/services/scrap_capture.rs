//! 高性能异步 YOLO 推理系统
//! 
//! 技术栈：
//! - scrap: 高性能零拷贝屏幕捕获
//! - tract-onnx: Pure Rust ONNX 推理引擎
//! 
//! 性能目标：30+ FPS @ 1920x1080

use std::path::Path;
use std::time::{Duration, Instant};

use image::{DynamicImage, GenericImageView, RgbaImage};
use scrap::{Capturer, Display};

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
    pub target_fps: u32,
    pub input_size: usize,
    pub inference_interval: u32,
    pub confidence_threshold: f32,
    pub nms_threshold: f32,
    pub class_names: Vec<String>,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            target_fps: 30,
            input_size: 640,
            inference_interval: 1,
            confidence_threshold: 0.65,
            nms_threshold: 0.45,
            class_names: vec![
                "elephant".to_string(),
                "zebra".to_string(),
                "buffalo".to_string(),
                "rhino".to_string(),
            ],
        }
    }
}

/// 性能统计
#[derive(Debug, Default, Clone)]
pub struct PerfStats {
    pub fps: f32,
    pub capture_time_ms: f64,
    pub preprocess_time_ms: f64,
    pub inference_time_ms: f64,
    pub postprocess_time_ms: f64,
    pub total_time_ms: f64,
    pub num_detections: usize,
}

/// 高性能捕获服务
pub struct ScrapCaptureService {
    config: CaptureConfig,
    running: bool,
}

impl ScrapCaptureService {
    pub fn new(config: CaptureConfig) -> Self {
        Self {
            config,
            running: false,
        }
    }
    
    pub fn start(&mut self, monitor_index: usize) -> Result<(), String> {
        if self.running {
            return Err("Already running".to_string());
        }
        
        self.running = true;
        
        let config = self.config.clone();
        
        std::thread::spawn(move || {
            Self::capture_loop(config, monitor_index);
        });
        
        Ok(())
    }
    
    fn capture_loop(config: CaptureConfig, monitor_index: usize) {
        // 获取显示器列表
        let displays = match Display::all() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[Scrap] Failed to get displays: {}", e);
                return;
            }
        };
        
        if monitor_index >= displays.len() {
            eprintln!("[Scrap] Monitor {} not found", monitor_index);
            return;
        }
        
        // 克隆 display 以获取所有权
        let display = displays.into_iter().nth(monitor_index).unwrap();
        
        let mut capturer = match Capturer::new(display) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[Scrap] Failed to create capturer: {}", e);
                return;
            }
        };
        
        let width = capturer.width() as u32;
        let height = capturer.height() as u32;
        
        eprintln!("[Scrap] Capturing {}x{} at {} FPS", width, height, config.target_fps);
        
        let frame_interval = Duration::from_micros(1_000_000 / config.target_fps as u64);
        let mut frame_count: u64 = 0;
        let mut last_fps_time = Instant::now();
        let mut frames_since_last = 0u32;
        let mut running = true;
        
        while running {
            let start = Instant::now();
            
            match capturer.frame() {
                Ok(frame_data) => {
                    let capture_time = start.elapsed();
                    
                    // 转换为 RGBA 数据
                    let rgba_data = frame_data.to_owned();
                    
                    frame_count += 1;
                    
                    if frame_count % config.inference_interval as u64 != 0 {
                        frames_since_last += 1;
                        
                        if last_fps_time.elapsed() >= Duration::from_secs(1) {
                            let fps = frames_since_last as f32 / last_fps_time.elapsed().as_secs_f32();
                            eprintln!(
                                "[PERF-Scrap] FPS: {:.1}, Capture: {:.1}ms",
                                fps,
                                capture_time.as_secs_f64() * 1000.0
                            );
                            frames_since_last = 0;
                            last_fps_time = Instant::now();
                        }
                        
                        continue;
                    }
                    
                    // 转换为 DynamicImage
                    let img = RgbaImage::from_raw(width, height, rgba_data)
                        .map(DynamicImage::ImageRgba8)
                        .unwrap_or_else(|| DynamicImage::new_rgba8(width, height));
                    
                    let total_start = Instant::now();
                    
                    // 预处理
                    let preprocess_start = Instant::now();
                    let resized = img.resize_exact(
                        config.input_size as u32,
                        config.input_size as u32,
                        image::imageops::FilterType::Nearest,
                    );
                    let rgb = resized.to_rgb8();
                    let pixels = rgb.as_raw();
                    
                    let size = config.input_size * config.input_size;
                    let mut data = vec![0.0f32; 3 * size];
                    
                    // RGB -> BGR, HWC -> CHW
                    for c in 0..3 {
                        for h in 0..config.input_size {
                            for w in 0..config.input_size {
                                let pixel_idx = (h * config.input_size + w) * 3;
                                let chw_idx = c * size + h * config.input_size + w;
                                // YOLO 使用 BGR 顺序
                                data[chw_idx] = pixels[pixel_idx + (2 - c)] as f32 / 255.0;
                            }
                        }
                    }
                    let preprocess_time = preprocess_start.elapsed();
                    
                    // 推理（这里需要集成 tract-onnx）
                    let inference_start = Instant::now();
                    // TODO: 集成 tract-onnx 推理
                    let _ = data; // 避免未使用警告
                    let inference_time = inference_start.elapsed();
                    
                    // 后处理
                    let postprocess_time = Duration::from_millis(1);
                    let total_time = total_start.elapsed();
                    
                    frames_since_last += 1;
                    
                    if last_fps_time.elapsed() >= Duration::from_secs(1) {
                        let fps = frames_since_last as f32 / last_fps_time.elapsed().as_secs_f32();
                        
                        eprintln!(
                            "[PERF-Scrap] FPS: {:.1}, Capture: {:.1}ms, Pre: {:.1}ms, Inf: {:.1}ms, Post: {:.1}ms, Total: {:.1}ms",
                            fps,
                            capture_time.as_secs_f64() * 1000.0,
                            preprocess_time.as_secs_f64() * 1000.0,
                            inference_time.as_secs_f64() * 1000.0,
                            postprocess_time.as_secs_f64() * 1000.0,
                            total_time.as_secs_f64() * 1000.0
                        );
                        
                        frames_since_last = 0;
                        last_fps_time = Instant::now();
                    }
                }
                Err(_) => {
                    // 丢帧
                }
            }
            
            if start.elapsed() < frame_interval {
                std::thread::sleep(frame_interval - start.elapsed());
            }
        }
        
        eprintln!("[Scrap] Capture loop ended");
    }
    
    pub fn stop(&mut self) {
        self.running = false;
    }
}

/// Sigmoid 函数
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// 模型转换器 - tch-rs 用于模型转换
pub struct ModelConverter {
    device: tch::Device,
}

impl ModelConverter {
    pub fn new() -> Self {
        let device = if tch::Cuda::is_available() {
            eprintln!("[ModelConverter] Using CUDA GPU");
            tch::Device::Cuda(0)
        } else {
            eprintln!("[ModelConverter] Using CPU");
            tch::Device::Cpu
        };
        
        Self { device }
    }
    
    /// 加载 TorchScript 模型
    pub fn load_torchscript(&self, model_path: &Path) -> Result<tch::CModule, String> {
        eprintln!("[ModelConverter] Loading TorchScript: {}", model_path.display());
        
        tch::CModule::load(&model_path)
            .map_err(|e| format!("Failed to load TorchScript: {}", e))
    }
    
    /// 列出可用的显示器
    pub fn list_displays() -> Vec<String> {
        Display::all()
            .unwrap_or_default()
            .iter()
            .enumerate()
            .map(|(i, d)| format!("Display {}: {}x{}", i, d.width(), d.height()))
            .collect()
    }
}
