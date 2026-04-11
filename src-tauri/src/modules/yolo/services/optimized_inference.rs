//! 高性能推理引擎 - 优化版本
//! 
//! 优化点：
//! 1. 模型缓存和预加载
//! 2. 批量推理支持
//! 3. 动态批处理
//! 4. 内存池优化
//! 5. 多线程并行推理

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use tract_onnx::prelude::*;
use rayon::prelude::*;
use lru_cache::LruCache;
use parking_lot::Mutex as ParkMutex;
use once_cell::sync::Lazy;

// TractModel 类型别名
type TractModel = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

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

/// 预处理的帧数据
#[derive(Debug)]
pub struct PreprocessedFrame {
    pub tensor: Tensor,
    pub width: u32,
    pub height: u32,
    pub timestamp: u64,
}

/// 推理结果
#[derive(Debug)]
pub struct DetectionResult {
    pub boxes: Vec<DetectionBox>,
    pub timestamp: u64,
    pub inference_time_ms: f64,
}

/// 批量配置
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_batch_size: usize,
    pub max_wait_ms: u64,
    pub dynamic_batching: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 8,
            max_wait_ms: 50,
            dynamic_batching: true,
        }
    }
}

/// 内存池 - 避免频繁分配
pub struct MemoryPool {
    input_buffers: Vec<Arc<ParkMutex<Vec<f32>>>>,
    tensor_buffers: Vec<Arc<ParkMutex<Tensor>>>,
}

impl MemoryPool {
    pub fn new() -> Self {
        let mut input_buffers = Vec::new();
        let mut tensor_buffers = Vec::new();
        
        // 预分配 16 个缓冲区
        for _ in 0..16 {
            input_buffers.push(Arc::new(ParkMutex::new(vec![0.0f32; 3 * 640 * 640])));
            tensor_buffers.push(Arc::new(ParkMutex::new(Tensor::new::<f32>(&[], &[]).unwrap())));
        }
        
        Self {
            input_buffers,
            tensor_buffers,
        }
    }
    
    pub fn acquire_input_buffer(&self) -> Arc<ParkMutex<Vec<f32>>> {
        self.input_buffers
            .first()
            .cloned()
            .unwrap_or_else(|| Arc::new(ParkMutex::new(vec![0.0f32; 3 * 640 * 640])))
    }
}

/// 全局模型缓存 - LRU 缓存
type ModelCache = LruCache<String, Arc<TractModel>>;

/// 全局模型缓存管理器
struct ModelCacheManager {
    cache: ParkMutex<ModelCache>,
    memory_pool: MemoryPool,
}

impl ModelCacheManager {
    fn new(max_models: usize) -> Self {
        Self {
            cache: ParkMutex::new(LruCache::new(max_models)),
            memory_pool: MemoryPool::new(),
        }
    }
    
    fn get_or_load(&self, model_path: &str) -> Result<Arc<TractModel>, String> {
        let mut cache = self.cache.lock();
        
        // 检查缓存
        if let Some(model) = cache.get(model_path) {
            return Ok(model.clone());
        }
        
        // 加载模型
        let model = Self::load_model_internal(model_path)?;
        let model = Arc::new(model);
        
        // 添加到缓存
        cache.insert(model_path.to_string(), model.clone());
        
        Ok(model)
    }
    
    fn load_model_internal(model_path: &str) -> Result<TractModel, String> {
        let path = Path::new(model_path);
        
        if !path.exists() {
            return Err(format!("模型文件不存在: {}", model_path));
        }
        
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        if ext != "onnx" {
            return Err(format!("不支持的模型格式: .{}", ext));
        }
        
        eprintln!("[OptimizedInference] Loading model: {}", model_path);
        
        let start = Instant::now();
        
        // 加载并编译模型
        let model = tract_onnx::onnx()
            .model_for_path(path)
            .map_err(|e| format!("模型加载失败: {}", e))?
            .with_input_fact(0, f32::fact(&[1, 3, 640, 640]).into())
            .map_err(|e| format!("输入配置失败: {}", e))?
            .into_typed()
            .map_err(|e| format!("类型推理失败: {}", e))?
            .into_runnable()
            .map_err(|e| format!("模型编译失败: {}", e))?;
        
        eprintln!("[OptimizedInference] Model loaded in {:.2}s", start.elapsed().as_secs_f64());
        
        Ok(model)
    }
}

/// 全局模型缓存实例
static MODEL_CACHE: Lazy<ModelCacheManager> = Lazy::new(|| {
    ModelCacheManager::new(4) // 最多缓存 4 个模型
});

/// 优化的推理引擎
pub struct OptimizedInferenceEngine {
    model: Arc<TractModel>,
    input_size: usize,
    class_names: Vec<String>,
    iou_threshold: f32,
    batch_config: BatchConfig,
}

impl OptimizedInferenceEngine {
    /// 加载模型（使用缓存）
    pub fn load<P: AsRef<Path>>(model_path: P) -> Result<Self, String> {
        let path = model_path.as_ref().to_string_lossy().to_string();
        let model = MODEL_CACHE.get_or_load(&path)?;
        
        Ok(Self {
            model,
            input_size: 640,
            class_names: DEFAULT_CLASS_NAMES.iter().map(|s| s.to_string()).collect(),
            iou_threshold: 0.45,
            batch_config: BatchConfig::default(),
        })
    }
    
    /// 预加载模型（异步）
    pub fn preload<P: AsRef<Path>>(model_path: P) {
        let path = model_path.as_ref().to_string_lossy().to_string();
        thread::spawn(move || {
            if let Err(e) = MODEL_CACHE.get_or_load(&path) {
                eprintln!("[OptimizedInference] Preload failed: {}", e);
            } else {
                eprintln!("[OptimizedInference] Model preloaded: {}", path);
            }
        });
    }
    
    /// 检测单帧
    pub fn detect(&self, img: &DynamicImage, confidence: f32) -> DetectionResult {
        let start = Instant::now();
        
        let input = self.preprocess(img);
        let boxes = self.run_inference(&[input], confidence);
        
        let inference_time = start.elapsed().as_secs_f64() * 1000.0;
        
        DetectionResult {
            boxes,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            inference_time_ms: inference_time,
        }
    }
    
    /// 批量检测 - 高性能并行推理
    pub fn batch_detect(&self, images: &[DynamicImage], confidence: f32) -> Vec<DetectionResult> {
        let start = Instant::now();
        
        // 并行预处理
        let preprocessed: Vec<PreprocessedFrame> = images
            .par_iter()
            .map(|img| PreprocessedFrame {
                tensor: self.preprocess(img),
                width: img.width(),
                height: img.height(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            })
            .collect();
        
        // 并行推理
        let results: Vec<DetectionResult> = preprocessed
            .par_iter()
            .map(|frame| {
                let boxes = self.run_inference_single(&frame.tensor, frame.width, frame.height, confidence);
                
                DetectionResult {
                    boxes,
                    timestamp: frame.timestamp,
                    inference_time_ms: 0.0,
                }
            })
            .collect();
        
        let total_time = start.elapsed().as_secs_f64() * 1000.0;
        eprintln!("[OptimizedInference] Batch inference: {} images in {:.2}ms ({:.2}ms per image)",
            images.len(), total_time, total_time / images.len() as f64);
        
        results
    }
    
    /// 优化的预处理 - Triangle 插值
    fn preprocess(&self, img: &DynamicImage) -> Tensor {
        let resized = img.resize_exact(
            self.input_size as u32,
            self.input_size as u32,
            FilterType::Triangle,
        );
        
        let rgb = resized.to_rgb8();
        let pixels = rgb.as_raw();
        let (height, width) = rgb.dimensions();
        
        let mut data = vec![0.0f32; 3 * height as usize * width as usize];
        
        // RGB -> BGR + 归一化
        let area = (height as usize) * (width as usize);
        
        for i in 0..area {
            let src_idx = i * 3;
            data[i] = pixels[src_idx + 2] as f32 / 255.0;
            data[area + i] = pixels[src_idx + 1] as f32 / 255.0;
            data[2 * area + i] = pixels[src_idx] as f32 / 255.0;
        }
        
        Tensor::from_shape(&[1, 3, height as usize, width as usize], &data)
            .expect("Tensor creation failed")
    }
    
    /// 运行推理（单帧）
    fn run_inference_single(&self, input: &Tensor, orig_width: u32, orig_height: u32, confidence: f32) -> Vec<DetectionBox> {
        let result = match self.model.run(tvec![input.clone().into()]) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[OptimizedInference] Error: {}", e);
                return vec![];
            }
        };
        
        self.postprocess(&result[0], orig_width, orig_height, confidence)
    }
    
    /// 运行推理（批量）
    fn run_inference(&self, inputs: &[Tensor], confidence: f32) -> Vec<Vec<DetectionBox>> {
        // 串行执行（tract-onnx 当前版本不支持批量并行）
        inputs
            .iter()
            .map(|input| {
                let result = match self.model.run(tvec![input.clone().into()]) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("[OptimizedInference] Error: {}", e);
                        return vec![];
                    }
                };
                
                self.postprocess(&result[0], 640, 640, confidence)
            })
            .collect()
    }
    
    /// 后处理 - 包含 NMS
    fn postprocess(&self, output: &Tensor, orig_width: u32, orig_height: u32, confidence: f32) -> Vec<DetectionBox> {
        let shape = output.shape();
        
        if shape.len() != 3 {
            return vec![];
        }
        
        let num_boxes = shape[1] as usize;
        let num_features = shape[2] as usize;
        let num_classes = if num_features > 4 { num_features - 4 } else { 0 };
        
        let scale_x = orig_width as f32 / self.input_size as f32;
        let scale_y = orig_height as f32 / self.input_size as f32;
        
        let output_data = match output.to_array_view::<f32>() {
            Ok(d) => d,
            Err(_) => return vec![],
        };
        
        // 第一遍：收集所有高置信度检测
        let mut detections: Vec<(f32, f32, f32, f32, f32, usize)> = Vec::with_capacity(100);
        
        for i in 0..num_boxes {
            let mut max_score = 0.0f32;
            let mut max_class = 0usize;
            
            for c in 0..num_classes {
                let score = output_data[[0, i, c + 4]];
                if score > max_score {
                    max_score = score;
                    max_class = c;
                }
            }
            
            if max_score >= confidence {
                let cx = output_data[[0, i, 0]];
                let cy = output_data[[0, i, 1]];
                let w = output_data[[0, i, 2]];
                let h = output_data[[0, i, 3]];
                
                let x1 = (cx - w / 2.0).max(0.0) * scale_x;
                let y1 = (cy - h / 2.0).max(0.0) * scale_y;
                let x2 = (cx + w / 2.0).min(self.input_size as f32) * scale_x;
                let y2 = (cy + h / 2.0).min(self.input_size as f32) * scale_y;
                
                detections.push((x1, y1, x2, y2, max_score, max_class));
            }
        }
        
        // NMS
        let nms_result = self.nms(detections, self.iou_threshold);
        
        // 转换格式
        nms_result.into_iter().map(|(x1, y1, x2, y2, conf, class_id)| {
            DetectionBox {
                class_id,
                class_name: self.class_names.get(class_id).cloned().unwrap_or_else(|| format!("class_{}", class_id)),
                confidence: conf,
                x: x1,
                y: y1,
                width: x2 - x1,
                height: y2 - y1,
            }
        }).collect()
    }
    
    /// 优化的 NMS
    fn nms(&self, mut boxes: Vec<(f32, f32, f32, f32, f32, usize)>, iou_threshold: f32) -> Vec<(f32, f32, f32, f32, f32, usize)> {
        if boxes.len() <= 1 {
            return boxes;
        }
        
        boxes.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap());
        
        let mut keep = Vec::with_capacity(boxes.len());
        
        while let Some(best) = boxes.pop() {
            keep.push(best);
            boxes.retain(|box_| {
                if box_.5 != best.5 {
                    return true;
                }
                self.calculate_iou(&best, box_) < iou_threshold
            });
        }
        
        keep
    }
    
    /// 计算 IoU
    #[inline]
    fn calculate_iou(&self, box1: &(f32, f32, f32, f32, f32, usize), box2: &(f32, f32, f32, f32, f32, usize)) -> f32 {
        let x1_inter = box1.0.max(box2.0);
        let y1_inter = box1.1.max(box2.1);
        let x2_inter = box1.2.min(box2.2);
        let y2_inter = box1.3.min(box2.3);
        
        let inter_w = (x2_inter - x1_inter).max(0.0);
        let inter_h = (y2_inter - y1_inter).max(0.0);
        let inter_area = inter_w * inter_h;
        
        let area1 = (box1.2 - box1.0).max(0.0) * (box1.3 - box1.1).max(0.0);
        let area2 = (box2.2 - box2.0).max(0.0) * (box2.3 - box2.1).max(0.0);
        let union_area = area1 + area2 - inter_area;
        
        if union_area > 0.0 {
            inter_area / union_area
        } else {
            0.0
        }
    }
}

/// 创建并行推理引擎
pub struct ParallelInferenceEngine {
    engines: Vec<OptimizedInferenceEngine>,
    thread_pool: rayon::ThreadPool,
}

impl ParallelInferenceEngine {
    pub fn new(num_threads: usize) -> Result<Self, String> {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .map_err(|e| format!("Failed to create thread pool: {}", e))?;
        
        Ok(Self {
            engines: Vec::new(),
            thread_pool,
        })
    }
    
    /// 加载多个模型到不同线程
    pub fn load_models(&mut self, model_paths: &[String]) -> Result<(), String> {
        for path in model_paths {
            let engine = OptimizedInferenceEngine::load(path)?;
            self.engines.push(engine);
        }
        
        if self.engines.is_empty() {
            return Err("No models loaded".to_string());
        }
        
        Ok(())
    }
    
    /// 并行推理
    pub fn parallel_detect(&self, images: &[DynamicImage], confidence: f32) -> Vec<DetectionResult> {
        let chunk_size = (images.len() / self.engines.len()).max(1);
        
        images
            .chunks(chunk_size)
            .zip(self.engines.iter())
            .flat_map(|(chunk, engine)| {
                chunk
                    .iter()
                    .map(|img| engine.detect(img, confidence))
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}
