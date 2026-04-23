#![allow(dead_code)]

//!
//! 高性能 YOLO 推理引擎 - scrap + tract-onnx 异步版本
//! 
//! 技术栈：
//! - scrap: 零拷贝屏幕捕获
//! - tract-onnx: 纯Rust ONNX推理（CPU）
//! - tokio: 异步运行时
//! 
//! 特点：
//! - 零拷贝数据流
//! - 异步非阻塞
//! - 纯Rust实现，无Python依赖

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tract_onnx::prelude::*;

// ==================== 类型定义 ====================

/// YOLO检测结果
#[derive(Debug, Clone)]
pub struct YoloDetection {
    pub class_id: usize,
    pub class_name: String,
    pub confidence: f32,
    pub x1: f32, pub y1: f32,
    pub x2: f32, pub y2: f32,
}

/// 捕获帧数据
#[derive(Debug, Clone)]
pub struct CaptureFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

/// 推理性能统计
#[derive(Debug, Clone, Default)]
pub struct InferenceStats {
    pub capture_time_ms: f64,
    pub preprocess_time_ms: f64,
    pub inference_time_ms: f64,
    pub postprocess_time_ms: f64,
    pub total_time_ms: f64,
    pub fps: f64,
    pub num_detections: usize,
}

/// 捕获会话状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionState {
    Idle,
    Running,
    Stopping,
}

/// 推理配置
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    pub model_path: String,
    pub class_names: Vec<String>,
    pub confidence_threshold: f32,
    pub nms_threshold: f32,
    pub input_width: i64,
    pub input_height: i64,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            model_path: String::new(),
            class_names: vec![
                "elephant".to_string(),
                "zebra".to_string(),
                "buffalo".to_string(),
                "rhino".to_string(),
            ],
            confidence_threshold: 0.65,
            nms_threshold: 0.45,
            input_width: 640,
            input_height: 640,
        }
    }
}

// ==================== 异步捕获服务 ====================

/// 异步捕获服务 - 使用标准线程管理
pub struct AsyncCaptureService {
    session_id: String,
    config: InferenceConfig,
    state: Arc<Mutex<SessionState>>,
    stats: Arc<Mutex<InferenceStats>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl AsyncCaptureService {
    pub fn new(
        session_id: String,
        _display_index: usize,
        config: InferenceConfig,
    ) -> Result<Self, String> {
        Ok(Self {
            session_id,
            config,
            state: Arc::new(Mutex::new(SessionState::Idle)),
            stats: Arc::new(Mutex::new(InferenceStats::default())),
            handle: None,
        })
    }
    
    pub fn start(&mut self, target_fps: u32) {
        {
            let mut state = self.state.lock().unwrap();
            if *state == SessionState::Running {
                return;
            }
            *state = SessionState::Running;
        }
        
        let config = self.config.clone();
        let state = Arc::clone(&self.state);
        let stats = Arc::clone(&self.stats);
        let session_id = self.session_id.clone();
        
        let frame_delay = Duration::from_millis((1000 / target_fps) as u64);
        
        println!("[AsyncCapture] 启动捕获线程 (FPS: {}, 帧间隔: {:?})", target_fps, frame_delay);
        
        self.handle = Some(std::thread::spawn(move || {
            // 在线程中加载模型和创建捕获器
            let (engine, mut capturer, width, height) = match Self::init_capture(&config) {
                Ok((e, c, w, h)) => {
                    println!("[AsyncCapture] 初始化成功");
                    (Some(e), Some(c), w, h)
                }
                Err(e) => {
                    eprintln!("[AsyncCapture] 初始化失败: {}", e);
                    (None, None, 0, 0)
                }
            };
            
            let mut last_capture = Instant::now();
            let mut frame_count = 0u64;
            let mut fps_timer = Instant::now();
            
            while {
                let current_state = state.lock().unwrap();
                *current_state == SessionState::Running
            } {
                let frame_start = Instant::now();
                
                // 捕获帧
                let frame = if let (Some(ref mut capt), w, h) = (capturer.as_mut(), width, height) {
                    match capt.frame() {
                        Ok(f) => {
                            let data = f.to_vec();
                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64;
                            Some(CaptureFrame { width: w, height: h, data, timestamp })
                        }
                        Err(_) => None,
                    }
                } else {
                    None
                };
                
                let capture_time = frame_start.elapsed().as_secs_f64() * 1000.0;
                
                if let Some(frame) = frame {
                    let (detections, inference_time) = if let Some(ref eng) = engine {
                        let start = Instant::now();
                        let boxes = Self::run_inference(eng, &frame.data, frame.width, frame.height, &config);
                        let time = start.elapsed().as_secs_f64() * 1000.0;
                        (boxes, time)
                    } else {
                        (Vec::new(), 0.0)
                    };
                    
                    let total_time = frame_start.elapsed().as_secs_f64() * 1000.0;
                    
                    {
                        let mut s = stats.lock().unwrap();
                        s.capture_time_ms = capture_time;
                        s.inference_time_ms = inference_time;
                        s.total_time_ms = total_time;
                        s.num_detections = detections.len();
                    }
                    
                    frame_count += 1;
                    if fps_timer.elapsed().as_secs() >= 1 {
                        let fps = frame_count as f64 / fps_timer.elapsed().as_secs_f64();
                        let s = stats.lock().unwrap();
                        
                        eprintln!(
                            "[AsyncCapture-Perf] FPS: {:.1}, 捕获: {:.1}ms, 推理: {:.1}ms, 总计: {:.1}ms, 检测: {}",
                            fps, capture_time, inference_time, total_time, detections.len()
                        );
                        
                        frame_count = 0;
                        fps_timer = Instant::now();
                    }
                }
                
                let elapsed = last_capture.elapsed();
                if elapsed < frame_delay {
                    std::thread::sleep(frame_delay - elapsed);
                }
                last_capture = Instant::now();
            }
            
            println!("[AsyncCapture] 捕获线程退出 (session: {})", session_id);
        }));
    }
    
    fn init_capture(config: &InferenceConfig) -> Result<(
        SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
        scrap::Capturer,
        u32,
        u32,
    ), String> {
        // 加载模型
        let engine = Self::load_model(&config.model_path)?;
        println!("[AsyncCapture] 模型加载成功");
        
        // 创建捕获器
        let displays = scrap::Display::all()
            .map_err(|e| format!("枚举显示器失败: {}", e))?;
        
        if displays.is_empty() {
            return Err("未找到显示器".to_string());
        }
        
        let display = displays.into_iter().next().unwrap();
        let width = display.width() as u32;
        let height = display.height() as u32;
        
        let capturer = scrap::Capturer::new(display)
            .map_err(|e| format!("创建捕获器失败: {}", e))?;
        
        println!("[AsyncCapture] 捕获器创建成功: {}x{}", width, height);
        
        Ok((engine, capturer, width, height))
    }
    
    pub fn load_model(model_path: &str) -> Result<SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>, String> {
        tract_onnx::onnx()
            .model_for_path(model_path)
            .map_err(|e| format!("模型加载失败: {}", e))?
            .with_input_fact(0, TypedFact::shape::<f32, _>(&[1, 3, 640, 640]).into())
            .map_err(|e| format!("输入配置失败: {}", e))?
            .into_typed()
            .map_err(|e| format!("类型推理失败: {}", e))?
            .into_optimized()
            .map_err(|e| format!("模型优化失败: {}", e))?
            .into_runnable()
            .map_err(|e| format!("模型编译失败: {}", e))
    }
    
    fn run_inference(
        engine: &SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
        data: &[u8],
        width: u32,
        height: u32,
        config: &InferenceConfig,
    ) -> Vec<YoloDetection> {
        let input = match Self::preprocess(data, width, height, config.input_width as u32, config.input_height as u32) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[AsyncCapture] 预处理失败: {}", e);
                return Vec::new();
            }
        };
        
        let outputs = match engine.run(tvec![input.into()]) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[AsyncCapture] 推理失败: {}", e);
                return Vec::new();
            }
        };
        
        let output = &outputs[0];
        let shape = output.shape().to_vec();
        let output_data = match output.to_array_view::<f32>() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[AsyncCapture] 读取输出失败: {}", e);
                return Vec::new();
            }
        };
        
        Self::postprocess_from_array(output_data, shape, width, height, config)
    }
    
    fn preprocess(
        input: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
    ) -> Result<Tensor, String> {
        let src_size = (src_w * src_h * 4) as usize;
        if input.len() < src_size {
            return Err("输入数据长度不足".to_string());
        }
        
        let mut rgb: Vec<u8> = Vec::with_capacity((src_w * src_h * 3) as usize);
        for i in 0..(src_w * src_h) as usize {
            let idx = i * 4;
            rgb.push(input[idx + 2]);
            rgb.push(input[idx + 1]);
            rgb.push(input[idx + 0]);
        }
        
        let resized = Self::resize_bilinear(&rgb, src_w, src_h, dst_w, dst_h);
        
        let mut tensor_data: Vec<f32> = Vec::with_capacity((3 * dst_w * dst_h) as usize);
        
        for c in 0..3 {
            for i in 0..(dst_w * dst_h) as usize {
                tensor_data.push(resized[c * (dst_w * dst_h) as usize + i] as f32 / 255.0);
            }
        }
        
        let array = ndarray::Array4::from_shape_vec(
            (1, 3, dst_h as usize, dst_w as usize),
            tensor_data
        ).map_err(|e| format!("创建tensor失败: {:?}", e))?;
        
        Ok(Tensor::from(array))
    }
    
    fn resize_bilinear(input: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
        let mut output = vec![0u8; (dst_w * dst_h * 3) as usize];
        
        let scale_x = src_w as f32 / dst_w as f32;
        let scale_y = src_h as f32 / dst_h as f32;
        
        for y in 0..dst_h {
            for x in 0..dst_w {
                let src_x = x as f32 * scale_x;
                let src_y = y as f32 * scale_y;
                
                let x0 = src_x as u32;
                let y0 = src_y as u32;
                let x1 = (x0 + 1).min(src_w - 1);
                let y1 = (y0 + 1).min(src_h - 1);
                
                let fx = src_x - x0 as f32;
                let fy = src_y - y0 as f32;
                
                let idx00 = ((y0 * src_w + x0) * 3) as usize;
                let idx01 = ((y1 * src_w + x0) * 3) as usize;
                let idx10 = ((y0 * src_w + x1) * 3) as usize;
                let idx11 = ((y1 * src_w + x1) * 3) as usize;
                
                let dst_idx = ((y * dst_w + x) * 3) as usize;
                
                for c in 0..3 {
                    let v00 = input[idx00 + c] as f32;
                    let v01 = input[idx01 + c] as f32;
                    let v10 = input[idx10 + c] as f32;
                    let v11 = input[idx11 + c] as f32;
                    
                    let value = v00 * (1.0 - fx) * (1.0 - fy)
                        + v10 * fx * (1.0 - fy)
                        + v01 * (1.0 - fx) * fy
                        + v11 * fx * fy;
                    
                    output[dst_idx + c] = value as u8;
                }
            }
        }
        
        output
    }
    
    fn postprocess_from_array(
        output: ndarray::ArrayView<'_, f32, ndarray::Dim<ndarray::IxDynImpl>>,
        shape: Vec<usize>,
        orig_w: u32,
        orig_h: u32,
        config: &InferenceConfig,
    ) -> Vec<YoloDetection> {
        let num_classes = config.class_names.len();
        let features_per_box = 4 + num_classes;
        
        let num_boxes = shape[2];
        let mut boxes: Vec<(f32, f32, f32, f32, f32, usize)> = Vec::new();
        
        for i in 0..num_boxes {
            let cx = output[[0, 0, i]];
            let cy = output[[0, 1, i]];
            let w = output[[0, 2, i]];
            let h = output[[0, 3, i]];
            
            let x1 = cx - w / 2.0;
            let y1 = cy - h / 2.0;
            let x2 = cx + w / 2.0;
            let y2 = cy + h / 2.0;
            
            let mut max_conf = 0.0f32;
            let mut max_class = 0usize;
            
            for c in 0..num_classes {
                let conf = output[[0, 4 + c, i]];
                if conf > max_conf {
                    max_conf = conf;
                    max_class = c;
                }
            }
            
            let confidence = 1.0 / (1.0 + (-max_conf).exp());
            
            if confidence >= config.confidence_threshold {
                boxes.push((x1, y1, x2, y2, confidence, max_class));
            }
        }
        
        let mut keep = Vec::new();
        boxes.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap());
        
        let mut suppressed = vec![false; boxes.len()];
        
        for i in 0..boxes.len() {
            if suppressed[i] {
                continue;
            }
            
            keep.push(i);
            
            for j in (i + 1)..boxes.len() {
                if suppressed[j] {
                    continue;
                }
                
                let iou = Self::compute_iou(&boxes[i], &boxes[j]);
                
                if iou > config.nms_threshold {
                    suppressed[j] = true;
                }
            }
        }
        
        let detections: Vec<YoloDetection> = keep.iter()
            .map(|&i| {
                let (x1, y1, x2, y2, confidence, class_id) = boxes[i];
                
                let scale_x = orig_w as f32 / config.input_width as f32;
                let scale_y = orig_h as f32 / config.input_height as f32;
                
                YoloDetection {
                    class_id,
                    class_name: config.class_names[class_id].clone(),
                    confidence,
                    x1: x1 * scale_x,
                    y1: y1 * scale_y,
                    x2: x2 * scale_x,
                    y2: y2 * scale_y,
                }
            })
            .collect();
        
        detections
    }
    
    fn compute_iou(a: &(f32, f32, f32, f32, f32, usize), b: &(f32, f32, f32, f32, f32, usize)) -> f32 {
        let x1 = a.0.max(b.0);
        let y1 = a.1.max(b.1);
        let x2 = a.2.min(b.2);
        let y2 = a.3.min(b.3);
        
        let inter = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
        let area_a = (a.2 - a.0) * (a.3 - a.1);
        let area_b = (b.2 - b.0) * (b.3 - b.1);
        let union = area_a + area_b - inter;
        
        if union <= 0.0 {
            0.0
        } else {
            inter / union
        }
    }
    
    pub fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        *state = SessionState::Stopping;
        println!("[AsyncCapture] 发送停止信号");
    }
    
    pub fn get_stats(&self) -> InferenceStats {
        self.stats.lock().unwrap().clone()
    }
}
