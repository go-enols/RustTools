//! 核心推理引擎 - 最小优化版本
//! 
//! 专注于：
//! 1. 模型缓存
//! 2. 预分配内存
//! 3. 高性能预处理
//! 4. 优化的NMS

use std::path::Path;
use std::sync::Arc;
use parking_lot::Mutex;
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use tract_onnx::prelude::*;

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

/// 推理配置
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    pub input_size: usize,
    pub confidence_threshold: f32,
    pub iou_threshold: f32,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            input_size: 640,
            confidence_threshold: 0.5,
            iou_threshold: 0.45,
        }
    }
}

/// 核心推理引擎
pub struct InferenceCore {
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
    config: InferenceConfig,
    class_names: Vec<String>,
    input_buffer: Vec<f32>,
}

impl InferenceCore {
    /// 加载模型
    pub fn load<P: AsRef<Path>>(model_path: P, config: InferenceConfig) -> Result<Self, String> {
        let path = model_path.as_ref();
        
        if !path.exists() {
            return Err(format!("模型文件不存在: {}", path.display()));
        }
        
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        if ext != "onnx" {
            return Err(format!("仅支持ONNX格式，当前: .{}", ext));
        }
        
        eprintln!("[InferenceCore] Loading model: {}", path.display());
        
        // 加载并编译模型
        let model = tract_onnx::onnx()
            .model_for_path(path)
            .map_err(|e| format!("模型加载失败: {}", e))?
            .with_input_fact(0, f32::fact(&[1, 3, config.input_size as i32, config.input_size as i32]).into())
            .map_err(|e| format!("输入配置失败: {}", e))?
            .into_typed()
            .map_err(|e| format!("类型推理失败: {}", e))?
            .into_runnable()
            .map_err(|e| format!("模型编译失败: {}", e))?;
        
        eprintln!("[InferenceCore] Model compiled successfully");
        
        Ok(Self {
            model,
            config,
            class_names: DEFAULT_CLASS_NAMES.iter().map(|s| s.to_string()).collect(),
            input_buffer: vec![0.0f32; 3 * config.input_size * config.input_size],
        })
    }
    
    /// 检测目标
    pub fn detect(&mut self, img: &DynamicImage) -> Vec<DetectionBox> {
        // 预处理
        let input = self.preprocess(img);
        
        // 推理
        let result = match self.model.run(tvec![input.into()]) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[InferenceCore] Error: {}", e);
                return vec![];
            }
        };
        
        // 后处理
        self.postprocess(&result[0], img.width(), img.height())
    }
    
    /// 快速预处理
    #[inline]
    fn preprocess(&mut self, img: &DynamicImage) -> Tensor {
        // 使用 Nearest-Neighbor 加速（最快）
        let resized = img.resize_exact(
            self.config.input_size as u32,
            self.config.input_size as u32,
            FilterType::Nearest,
        );
        
        let rgb = resized.to_rgb8();
        let (height, width) = rgb.dimensions();
        let pixels = rgb.as_raw();
        let area = (height as usize) * (width as usize);
        
        // RGB -> BGR + 归一化
        for i in 0..area {
            let src_idx = i * 3;
            self.input_buffer[i] = pixels[src_idx + 2] as f32 / 255.0;           // B
            self.input_buffer[area + i] = pixels[src_idx + 1] as f32 / 255.0;   // G
            self.input_buffer[2 * area + i] = pixels[src_idx] as f32 / 255.0;    // R
        }
        
        Tensor::from_shape(&[1, 3, height as usize, width as usize], self.input_buffer.as_slice())
            .expect("Tensor creation failed")
    }
    
    /// 后处理 + NMS
    #[inline]
    fn postprocess(&self, output: &Tensor, orig_width: u32, orig_height: u32) -> Vec<DetectionBox> {
        let shape = output.shape();
        
        if shape.len() != 3 {
            return vec![];
        }
        
        let num_boxes = shape[1] as usize;
        let num_features = shape[2] as usize;
        let num_classes = if num_features > 4 { num_features - 4 } else { 0 };
        
        let scale_x = orig_width as f32 / self.config.input_size as f32;
        let scale_y = orig_height as f32 / self.config.input_size as f32;
        
        let output_data = match output.to_array_view::<f32>() {
            Ok(d) => d,
            Err(_) => return vec![],
        };
        
        // 第一遍：收集高置信度检测
        let mut detections: Vec<(f32, f32, f32, f32, f32, usize)> = Vec::with_capacity(100);
        
        for i in 0..num_boxes {
            // 找最大类别
            let mut max_score = 0.0f32;
            let mut max_class = 0usize;
            
            for c in 0..num_classes {
                let score = output_data[[0, i, c + 4]];
                if score > max_score {
                    max_score = score;
                    max_class = c;
                }
            }
            
            // 置信度过滤
            if max_score >= self.config.confidence_threshold {
                let cx = output_data[[0, i, 0]];
                let cy = output_data[[0, i, 1]];
                let w = output_data[[0, i, 2]];
                let h = output_data[[0, i, 3]];
                
                // 中心点 -> 左上右下
                let x1 = (cx - w / 2.0).max(0.0) * scale_x;
                let y1 = (cy - h / 2.0).max(0.0) * scale_y;
                let x2 = (cx + w / 2.0).min(self.config.input_size as f32) * scale_x;
                let y2 = (cy + h / 2.0).min(self.config.input_size as f32) * scale_y;
                
                detections.push((x1, y1, x2, y2, max_score, max_class));
            }
        }
        
        // NMS
        let nms_result = self.nms(detections);
        
        // 转换为 DetectionBox
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
    #[inline]
    fn nms(&self, mut boxes: Vec<(f32, f32, f32, f32, f32, usize)>) -> Vec<(f32, f32, f32, f32, f32, usize)> {
        if boxes.len() <= 1 {
            return boxes;
        }
        
        // 按置信度排序
        boxes.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap());
        
        let mut keep = Vec::with_capacity(boxes.len());
        
        while let Some(best) = boxes.pop() {
            keep.push(best);
            boxes.retain(|box_| {
                if box_.5 != best.5 {
                    return true;
                }
                self.calculate_iou(&best, box_) < self.config.iou_threshold
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

/// 模型缓存管理器
pub struct ModelCache {
    cache: Mutex<std::collections::HashMap<String, Arc<InferenceCore>>>,
    default_config: InferenceConfig,
}

impl ModelCache {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(std::collections::HashMap::new()),
            default_config: InferenceConfig::default(),
        }
    }
    
    pub fn get(&self, model_path: &str, confidence: f32) -> Result<Arc<InferenceCore>, String> {
        let cache_key = format!("{}_{}", model_path, confidence);
        
        // 检查缓存
        {
            let cache = self.cache.lock();
            if let Some(engine) = cache.get(&cache_key) {
                return Ok(engine.clone());
            }
        }
        
        // 加载模型
        let mut config = self.default_config.clone();
        config.confidence_threshold = confidence;
        
        let engine = InferenceCore::load(model_path, config)?;
        let engine = Arc::new(engine);
        
        // 添加到缓存
        {
            let mut cache = self.cache.lock();
            cache.insert(cache_key, engine.clone());
        }
        
        Ok(engine)
    }
}

impl Default for ModelCache {
    fn default() -> Self {
        Self::new()
    }
}
