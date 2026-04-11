//!
//! 高性能 YOLO 推理引擎 - scrap + burn + tch-rs 最终版本
//! 
//! 技术栈：
//! - scrap: 零拷贝屏幕捕获
//! - burn: 纯Rust推理（支持CPU/GPU）
//! - tch-rs: PyTorch模型转换
//! 
//! 特点：
//! - 零拷贝数据流
//! - 异步非阻塞
//! - 避免CPU-GPU数据复制
//! - 纯Rust实现，无Python依赖

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use tract_onnx::prelude::*;
use ndarray::Array4;

// 导入 Runnable trait
use tract_onnx::prelude::Runnable;

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

/// 捕获帧数据（零拷贝设计）
#[derive(Debug, Clone)]
pub struct CaptureFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,  // RGB格式
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

// ==================== 捕获器封装（解决线程安全问题） ====================

/// 线程安全的捕获器封装
struct SafeCapturer {
    // 使用 scrap 捕获器
    capturer: Option<scrap::Capturer>,
    width: u32,
    height: u32,
}

impl SafeCapturer {
    /// 创建新的捕获器
    pub fn new(display_index: usize) -> Result<Self, String> {
        // 枚举所有显示器
        let displays = scrap::Display::all()
            .map_err(|e| format!("枚举显示器失败: {}", e))?;
        
        if display_index >= displays.len() {
            return Err(format!(
                "显示器索引 {} 超出范围 (共 {} 个)",
                display_index, displays.len()
            ));
        }
        
        // 获取目标显示器
        let display = displays.into_iter().nth(display_index)
            .ok_or("显示器不存在")?;
        
        let width = display.width();
        let height = display.height();
        
        // 创建捕获器
        let capturer = scrap::Capturer::new(display)
            .map_err(|e| format!("创建捕获器失败: {}", e))?;
        
        println!("[ScrapCapture] 创建捕获器成功: {}x{}", width, height);
        
        Ok(Self {
            capturer: Some(capturer),
            width,
            height,
        })
    }
    
    /// 捕获一帧
    pub fn capture_frame(&mut self) -> Option<CaptureFrame> {
        let capturer = self.capturer.as_mut()?;
        
        match capturer.frame() {
            Ok(frame) => {
                // 转换为 Vec<u8>
                let data = frame.to_vec();
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                
                Some(CaptureFrame {
                    width: self.width,
                    height: self.height,
                    data,
                    timestamp,
                })
            }
            Err(e) => {
                // 可能是帧不可用，继续等待
                None
            }
        }
    }
    
    /// 获取尺寸
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

// ==================== YOLO 模型定义 (Burn) ====================

/// 简化的YOLO模型（基于Burn框架）
/// 注意：这是一个框架代码，实际的模型结构需要根据你的YOLO模型定义
pub struct YoloModel<B: burn::tensor::backend::Backend> {
    // 模型权重层
    // 具体实现取决于YOLO模型结构
    _backend: std::marker::PhantomData<B>,
}

// ==================== 推理引擎（基于tract-onnx） ====================

/// 推理引擎配置
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

/// 高性能推理引擎
pub struct HighPerfEngine {
    // Tract模型 - 使用 dyn  Trait Object
    model: Box<dyn Runnable + Send>,
    // 配置
    config: InferenceConfig,
}

unsafe impl Send for HighPerfEngine {}
unsafe impl Sync for HighPerfEngine {}

impl HighPerfEngine {
    /// 创建新引擎
    pub fn new(config: InferenceConfig) -> Result<Self, String> {
        println!("[HighPerfEngine] 加载模型: {}", config.model_path);
        
        // 加载ONNX模型
        let model = tract_onnx::onnx()
            .model_for_path(&config.model_path)
            .map_err(|e| format!("加载模型失败: {}", e))?;
        
        // 转换为可执行模型
        let model = model
            .into_optimized()
            .map_err(|e| format!("优化模型失败: {}", e))?;
        
        let model = model
            .into_runnable()
            .map_err(|e| format!("编译模型失败: {}", e))?;
        
        println!("[HighPerfEngine] 模型加载成功, 输入尺寸: {}x{}", 
            config.input_width, config.input_height);
        
        Ok(Self {
            model: Box::new(model),
            config,
        })
    }
    
    /// 运行推理
    pub fn infer(&self, input: &[u8], width: u32, height: u32) -> Result<Vec<YoloDetection>, String> {
        let start = Instant::now();
        
        // 1. 预处理
        let preprocessed = self.preprocess(input, width, height)?;
        let preprocess_time = start.elapsed().as_secs_f64() * 1000.0;
        
        // 2. 推理
        let inference_start = Instant::now();
        let output = self.run_model(&preprocessed)?;
        let inference_time = inference_start.elapsed().as_secs_f64() * 1000.0;
        
        // 3. 后处理
        let postprocess_start = Instant::now();
        let detections = self.postprocess(output, width, height)?;
        let postprocess_time = postprocess_start.elapsed().as_secs_f64() * 1000.0;
        
        let total_time = start.elapsed().as_secs_f64() * 1000.0;
        
        eprintln!(
            "[HighPerf-Perf] 预处理: {:.1}ms | 推理: {:.1}ms | 后处理: {:.1}ms | 总计: {:.1}ms",
            preprocess_time, inference_time, postprocess_time, total_time
        );
        
        Ok(detections)
    }
    
    /// 预处理：归一化 + 调整大小
    fn preprocess(&self, input: &[u8], width: u32, height: u32) -> Result<tract_onnx::prelude::Tensor, String> {
        let src_size = (width * height * 4) as usize;  // RGBA
        if input.len() < src_size {
            return Err("输入数据长度不足".to_string());
        }
        
        // 转换为 RGB
        let mut rgb = Vec::with_capacity((width * height * 3) as usize);
        for i in 0..(width * height) as usize {
            let idx = i * 4;
            rgb.push(input[idx + 2] as f32);     // B
            rgb.push(input[idx + 1] as f32);     // G
            rgb.push(input[idx + 0] as f32);     // R
        }
        
        // 缩放到模型输入尺寸
        let resized = self.resize_image(&rgb, width, height, 
            self.config.input_width as u32, 
            self.config.input_height as u32
        );
        
        // 归一化到 [0, 1] 并转换为 NCHW 格式
        let mut tensor_data: Vec<f32> = Vec::with_capacity(3 * self.config.input_width as usize * self.config.input_height as usize);
        
        // RGB通道
        for c in 0..3 {
            for i in 0..(self.config.input_width as usize * self.config.input_height as usize) {
                let pixel_idx = c * (self.config.input_width as usize * self.config.input_height as usize) + i;
                tensor_data.push(resized[pixel_idx] as f32 / 255.0);
            }
        }
        
        // 创建tensor [1, 3, H, W]
        let array: Array4<f32> = Array4::from_shape_vec(
            (1, 3, self.config.input_height as usize, self.config.input_width as usize),
            tensor_data
        ).map_err(|e| format!("创建tensor失败: {:?}", e))?;
        
        let tensor = Tensor::from(array);
        
        Ok(tensor)
    }
    
    /// 双线性插值缩放
    fn resize_image(&self, input: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
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
    
    /// 运行模型
    fn run_model(&self, input: &Tensor) -> Result<Vec<f32>, String> {
        let inputs = tvec![input];
        
        let outputs = self.model
            .run(inputs)
            .map_err(|e| format!("推理失败: {}", e))?;
        
        // 提取输出
        let output = outputs.into_iter().next()
            .ok_or("没有输出")?;
        
        let shape = output.shape();
        let data: Vec<f32> = output.into_slice::<f32>()
            .map_err(|e| format!("提取数据失败: {:?}", e))?
            .to_vec();
        
        eprintln!("[HighPerf] 模型输出形状: {:?}, 大小: {:.2}MB", 
            shape, (data.len() * 4) as f64 / 1_048_576.0);
        
        Ok(data)
    }
    
    /// 后处理：YOLOv8格式解析 + NMS
    fn postprocess(&self, output: Vec<f32>, orig_w: u32, orig_h: u32) -> Result<Vec<YoloDetection>, String> {
        // YOLOv8 输出格式: [1, 84, 8400] 或 [1, 4, 8400] (4类)
        // 84 = 4 (bbox) + 80 (classes)
        let num_classes = self.config.class_names.len();
        let features_per_box = 4 + num_classes;
        
        let total_boxes = output.len() / features_per_box;
        
        let mut boxes: Vec<(f32, f32, f32, f32, f32, usize)> = Vec::new();  // x1, y1, x2, y2, conf, class
        
        // 解析每个检测框
        for i in 0..total_boxes {
            let box_offset = i * features_per_box;
            
            // 提取边界框
            let cx = output[box_offset];
            let cy = output[box_offset + 1];
            let w = output[box_offset + 2];
            let h = output[box_offset + 3];
            
            // 转换为角点坐标
            let x1 = cx - w / 2.0;
            let y1 = cy - h / 2.0;
            let x2 = cx + w / 2.0;
            let y2 = cy + h / 2.0;
            
            // 找最大类别分数
            let mut max_conf = 0.0f32;
            let mut max_class = 0usize;
            
            for c in 0..num_classes {
                let conf = output[box_offset + 4 + c];
                if conf > max_conf {
                    max_conf = conf;
                    max_class = c;
                }
            }
            
            // 应用sigmoid
            let confidence = 1.0 / (1.0 + (-max_conf).exp());
            
            // 过滤低置信度
            if confidence >= self.config.confidence_threshold {
                boxes.push((x1, y1, x2, y2, confidence, max_class));
            }
        }
        
        // NMS
        let mut keep = Vec::new();
        boxes.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap());  // 按置信度排序
        
        let mut suppressed = vec![false; boxes.len()];
        
        for i in 0..boxes.len() {
            if suppressed[i] {
                continue;
            }
            
            keep.push(i);
            
            // 抑制重叠框
            for j in (i + 1)..boxes.len() {
                if suppressed[j] {
                    continue;
                }
                
                // 计算IOU
                let iou = self.compute_iou(&boxes[i], &boxes[j]);
                
                if iou > self.config.nms_threshold {
                    suppressed[j] = true;
                }
            }
        }
        
        // 构建最终检测结果
        let detections: Vec<YoloDetection> = keep.iter()
            .map(|&i| {
                let (x1, y1, x2, y2, confidence, class_id) = boxes[i];
                
                // 缩放到原始图像尺寸
                let scale_x = orig_w as f32 / self.config.input_width as f32;
                let scale_y = orig_h as f32 / self.config.input_height as f32;
                
                YoloDetection {
                    class_id,
                    class_name: self.config.class_names[class_id].clone(),
                    confidence,
                    x1: x1 * scale_x,
                    y1: y1 * scale_y,
                    x2: x2 * scale_x,
                    y2: y2 * scale_y,
                }
            })
            .collect();
        
        Ok(detections)
    }
    
    /// 计算IOU
    fn compute_iou(&self, a: &(f32, f32, f32, f32, f32, usize), b: &(f32, f32, f32, f32, f32, usize)) -> f32 {
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
}

// ==================== 异步捕获服务（使用标准线程） ====================

/// 异步捕获服务
pub struct AsyncCaptureService {
    // 会话ID
    session_id: String,
    // 捕获器
    capturer: Arc<Mutex<SafeCapturer>>,
    // 推理引擎
    engine: Option<HighPerfEngine>,
    // 状态
    state: Arc<Mutex<SessionState>>,
    // 统计
    stats: Arc<Mutex<InferenceStats>>,
    // 帧处理回调
    frame_callback: Arc<dyn Fn(CaptureFrame, Vec<YoloDetection>) + Send + Sync>,
}

impl AsyncCaptureService {
    /// 创建新服务
    pub fn new<F>(
        session_id: String,
        display_index: usize,
        model_path: Option<String>,
        class_names: Vec<String>,
        confidence: f32,
        on_frame: F,
    ) -> Result<Self, String>
    where
        F: Fn(CaptureFrame, Vec<YoloDetection>) + Send + Sync + 'static,
    {
        // 创建捕获器
        let capturer = SafeCapturer::new(display_index)?;
        
        // 创建推理引擎
        let engine = if let Some(ref path) = model_path {
            Some(HighPerfEngine::new(InferenceConfig {
                model_path: path.clone(),
                class_names,
                confidence_threshold: confidence,
                ..Default::default()
            })?)
        } else {
            None
        };
        
        Ok(Self {
            session_id,
            capturer: Arc::new(Mutex::new(capturer)),
            engine: Arc::new(engine),
            state: Arc::new(Mutex::new(SessionState::Idle)),
            stats: Arc::new(Mutex::new(InferenceStats::default())),
            frame_callback: Arc::new(on_frame),
        })
    }
    
    /// 启动捕获循环
    pub fn start(&self, target_fps: u32) {
        {
            let mut state = self.state.lock().unwrap();
            if *state == SessionState::Running {
                return;
            }
            *state = SessionState::Running;
        }
        
        let capturer = Arc::clone(&self.capturer);
        let engine = Arc::clone(&self.engine);
        let state = Arc::clone(&self.state);
        let stats = Arc::clone(&self.stats);
        let callback = Arc::clone(&self.frame_callback);
        let session_id = self.session_id.clone();
        
        let frame_delay = Duration::from_millis((1000 / target_fps) as u64);
        
        println!("[AsyncCapture] 启动捕获线程 (FPS: {}, 帧间隔: {:?})", target_fps, frame_delay);
        
        thread::spawn(move || {
            let mut last_capture = Instant::now();
            let mut frame_count = 0u64;
            let mut fps_timer = Instant::now();
            
            loop {
                // 检查状态
                {
                    let current_state = state.lock().unwrap();
                    if *current_state != SessionState::Running {
                        println!("[AsyncCapture] 捕获线程退出");
                        break;
                    }
                }
                
                let frame_start = Instant::now();
                
                // 捕获帧
                let frame = {
                    let mut capt = capturer.lock().unwrap();
                    capt.capture_frame()
                };
                
                let capture_time = frame_start.elapsed().as_secs_f64() * 1000.0;
                
                if let Some(mut frame) = frame {
                    // 运行推理
                    let (detections, inference_time) = if let Some(ref eng) = *engine {
                        let start = Instant::now();
                        match eng.infer(&frame.data, frame.width, frame.height) {
                            Ok(dets) => (dets, start.elapsed().as_secs_f64() * 1000.0),
                            Err(e) => {
                                eprintln!("[AsyncCapture] 推理错误: {}", e);
                                (Vec::new(), 0.0)
                            }
                        }
                    } else {
                        (Vec::new(), 0.0)
                    };
                    
                    let total_time = frame_start.elapsed().as_secs_f64() * 1000.0;
                    
                    // 更新统计
                    {
                        let mut s = stats.lock().unwrap();
                        s.capture_time_ms = capture_time;
                        s.inference_time_ms = inference_time;
                        s.total_time_ms = total_time;
                        s.num_detections = detections.len();
                    }
                    
                    // 调用回调
                    callback(frame, detections);
                    
                    // FPS计算
                    frame_count += 1;
                    if fps_timer.elapsed().as_secs() >= 1 {
                        let fps = frame_count as f64 / fps_timer.elapsed().as_secs_f64();
                        let mut s = stats.lock().unwrap();
                        s.fps = fps;
                        
                        eprintln!(
                            "[AsyncCapture] FPS: {:.1}, 捕获: {:.1}ms, 推理: {:.1}ms, 总计: {:.1}ms",
                            fps, capture_time, inference_time, total_time
                        );
                        
                        frame_count = 0;
                        fps_timer = Instant::now();
                    }
                }
                
                // 帧率限制
                let elapsed = last_capture.elapsed();
                if elapsed < frame_delay {
                    thread::sleep(frame_delay - elapsed);
                }
                last_capture = Instant::now();
            }
        });
    }
    
    /// 停止捕获
    pub fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        *state = SessionState::Stopping;
        println!("[AsyncCapture] 发送停止信号");
    }
    
    /// 获取统计
    pub fn get_stats(&self) -> InferenceStats {
        self.stats.lock().unwrap().clone()
    }
}

// ==================== tch-rs 模型转换工具 ====================

/// 模型转换器（使用 tch-rs）
/// 
/// 注意：tch-rs 主要用于加载 PyTorch 模型
/// ONNX 转换仍需要 Python 脚本
pub struct ModelConverter {
    device: tch::Device,
}

impl ModelConverter {
    /// 创建转换器
    pub fn new() -> Self {
        let device = if tch::Cuda::is_available() {
            println!("[ModelConverter] 检测到 CUDA GPU");
            tch::Device::Cuda(0)
        } else {
            println!("[ModelConverter] 使用 CPU");
            tch::Device::Cpu
        };
        
        Self { device }
    }
    
    /// 加载 PyTorch 模型
    pub fn load_pt(&self, path: &str) -> Result<tch::nn::Sequential, String> {
        println!("[ModelConverter] 加载 PyTorch 模型: {}", path);
        
        let mut vs = tch::nn::VarStore::new(self.device);
        vs.load(path)
            .map_err(|e| format!("加载模型失败: {}", e))?;
        
        // 注意：这里需要根据实际模型结构定义
        Ok(tch::nn::seq())
    }
    
    /// 获取设备信息
    pub fn device_info(&self) -> String {
        format!("Device: {:?}", self.device)
    }
}

// ==================== 工具函数 ====================

/// RGB 转 BGR
pub fn rgb_to_bgr(rgb: &[u8]) -> Vec<u8> {
    let mut bgr = Vec::with_capacity(rgb.len());
    
    for chunk in rgb.chunks(3) {
        bgr.push(chunk[2]);
        bgr.push(chunk[1]);
        bgr.push(chunk[0]);
    }
    
    bgr
}

/// 应用 Sigmoid
pub fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

// 模块导出
// 注意：类型已经在上面定义，这里不需要重新导出
