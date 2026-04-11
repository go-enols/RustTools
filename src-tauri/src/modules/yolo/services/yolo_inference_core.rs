//! YOLO推理核心模块
//! 
//! 纯Rust实现，无Python依赖

use std::path::Path;
use image::{DynamicImage, Rgb, imageops::FilterType};
use tract_onnx::prelude::tvec;

/// 默认COCO类别名称
pub const DEFAULT_CLASS_NAMES: [&str; 80] = [
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
    pub confidence: f32,
    pub iou_threshold: f32,
    pub input_size: usize,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            confidence: 0.65,
            iou_threshold: 0.45,
            input_size: 640,
        }
    }
}

/// 预分配内存池
pub struct MemoryPool {
    pub input_buffer: Vec<f32>,
}

impl MemoryPool {
    pub fn new(size: usize) -> Self {
        Self {
            input_buffer: vec![0.0f32; 3 * size * size],
        }
    }
}

/// 加载并编译YOLO模型
pub fn load_model<P: AsRef<Path>>(model_path: P) -> Result<tract_onnx::prelude::SimplePlan<
    tract_onnx::prelude::TypedFact, 
    Box<dyn tract_onnx::prelude::TypedOp>, 
    tract_onnx::prelude::Graph<tract_onnx::prelude::TypedFact, Box<dyn tract_onnx::prelude::TypedOp>>
>, String> {
    use tract_onnx::prelude::*;
    
    let path = model_path.as_ref();
    
    if !path.exists() {
        return Err(format!("模型文件不存在: {}", path.display()));
    }
    
    eprintln!("[Inference] 加载模型: {}", path.display());
    
    // 加载并编译模型
    tract_onnx::onnx()
        .model_for_path(path)
        .map_err(|e| format!("模型加载失败: {}", e))?
        .with_input_fact(0, f32::fact(&[1, 3, 640, 640]).into())
        .map_err(|e| format!("输入配置失败: {}", e))?
        .into_typed()
        .map_err(|e| format!("类型推理失败: {}", e))?
        .into_runnable()
        .map_err(|e| format!("模型编译失败: {}", e))
}

/// 检测目标
pub fn detect(
    model: &tract_onnx::prelude::SimplePlan<
        tract_onnx::prelude::TypedFact, 
        Box<dyn tract_onnx::prelude::TypedOp>, 
        tract_onnx::prelude::Graph<tract_onnx::prelude::TypedFact, Box<dyn tract_onnx::prelude::TypedOp>>
    >,
    img: &DynamicImage,
    confidence: f32,
    input_size: usize,
) -> Vec<DetectionBox> {
    // 预处理
    let input = preprocess(img, input_size);
    
    // 推理
    let result = match model.run(tvec![input.into()]) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[Inference] 推理失败: {}", e);
            return vec![];
        }
    };
    
    // 后处理
    postprocess(&result[0], img.width(), img.height(), confidence, input_size)
}

/// 预处理 - 优化的图像转换
fn preprocess(img: &DynamicImage, input_size: usize) -> tract_onnx::prelude::Tensor {
    // 使用Nearest Neighbor进行快速resize
    let resized = img.resize_exact(
        input_size as u32,
        input_size as u32,
        FilterType::Nearest,
    );
    
    let rgb = resized.to_rgb8();
    let pixels = rgb.as_raw();
    let (height, width) = rgb.dimensions();
    
    let area = (height as usize) * (width as usize);
    let mut buffer = vec![0.0f32; 3 * area];
    
    // RGB -> BGR + 归一化
    for i in 0..area {
        let src_idx = i * 3;
        buffer[i] = pixels[src_idx + 2] as f32 / 255.0;            // B
        buffer[area + i] = pixels[src_idx + 1] as f32 / 255.0;     // G
        buffer[2 * area + i] = pixels[src_idx] as f32 / 255.0;     // R
    }
    
    tract_onnx::prelude::Tensor::from_shape(&[1, 3, height as usize, width as usize], &buffer)
        .expect("Tensor创建失败")
}

/// 后处理 - 包含NMS
fn postprocess(
    output: &tract_onnx::prelude::Tensor,
    orig_width: u32,
    orig_height: u32,
    confidence: f32,
    input_size: usize,
) -> Vec<DetectionBox> {
    let shape = output.shape();
    
    if shape.len() != 3 {
        return vec![];
    }
    
    let num_boxes = shape[1] as usize;
    let num_features = shape[2] as usize;
    let num_classes = if num_features > 4 { num_features - 4 } else { 0 };
    
    let scale_x = orig_width as f32 / input_size as f32;
    let scale_y = orig_height as f32 / input_size as f32;
    
    let output_data = match output.to_array_view::<f32>() {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    
    // 第一遍：收集高置信度检测
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
            let x2 = (cx + w / 2.0).min(input_size as f32) * scale_x;
            let y2 = (cy + h / 2.0).min(input_size as f32) * scale_y;
            
            detections.push((x1, y1, x2, y2, max_score, max_class));
        }
    }
    
    // NMS
    let nms_result = nms(detections, 0.45);
    
    // 转换为DetectionBox
    nms_result.into_iter().map(|(x1, y1, x2, y2, conf, class_id)| {
        DetectionBox {
            class_id,
            class_name: DEFAULT_CLASS_NAMES.get(class_id)
                .cloned()
                .unwrap_or_else(|| format!("class_{}", class_id).leak())
                .to_string(),
            confidence: conf,
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }).collect()
}

/// 高效NMS算法
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
        boxes.retain(|box_| {
            if box_.5 != best.5 {
                return true;
            }
            calculate_iou(&best, box_) < iou_threshold
        });
    }
    
    keep
}

/// 计算IoU
#[inline]
fn calculate_iou(
    box1: &(f32, f32, f32, f32, f32, usize),
    box2: &(f32, f32, f32, f32, f32, usize),
) -> f32 {
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

/// 绘制检测框
pub fn draw_boxes(img: &DynamicImage, boxes: &[DetectionBox]) -> DynamicImage {
    let mut rgb = img.to_rgb8();
    let (width, height) = rgb.dimensions();
    
    let colors: [[u8; 3]; 8] = [
        [255, 107, 107], [78, 205, 196], [69, 183, 209],
        [150, 206, 180], [255, 234, 167], [221, 160, 221],
        [255, 159, 67], [199, 199, 199],
    ];
    
    for box_ in boxes {
        let color = colors[box_.class_id % 8];
        
        let x1 = (box_.x as i32).clamp(0, width as i32 - 1);
        let y1 = (box_.y as i32).clamp(0, height as i32 - 1);
        let x2 = ((box_.x + box_.width) as i32).clamp(0, width as i32);
        let y2 = ((box_.y + box_.height) as i32).clamp(0, height as i32);
        
        let thickness = 3;
        
        for x in x1..x2 {
            for t in 0..thickness {
                if y1 + t >= 0 && y1 + t < height as i32 && x >= 0 && x < width as i32 {
                    rgb.put_pixel(x as u32, (y1 + t) as u32, Rgb(color));
                }
                if y2 - 1 - t >= 0 && y2 - 1 - t < height as i32 && x >= 0 && x < width as i32 {
                    rgb.put_pixel(x as u32, (y2 - 1 - t) as u32, Rgb(color));
                }
            }
        }
        
        for y in y1..y2 {
            for t in 0..thickness {
                if x1 + t >= 0 && x1 + t < width as i32 && y >= 0 && y < height as i32 {
                    rgb.put_pixel((x1 + t) as u32, y as u32, Rgb(color));
                }
                if x2 - 1 - t >= 0 && x2 - 1 - t < width as i32 && y >= 0 && y < height as i32 {
                    rgb.put_pixel((x2 - 1 - t) as u32, y as u32, Rgb(color));
                }
            }
        }
    }
    
    DynamicImage::ImageRgb8(rgb)
}

/// 编码图像为Base64
pub fn encode_image(img: &DynamicImage, quality: u8) -> Result<String, String> {
    use std::io::Cursor;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;
    
    let rgb = img.to_rgb8();
    let mut buffer = Cursor::new(Vec::with_capacity(rgb.len() / 4));
    
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
    encoder.encode(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    ).map_err(|e| format!("编码失败: {}", e))?;
    
    Ok(BASE64.encode(buffer.into_inner()))
}

/// 快速编码（低质量预览）
pub fn encode_fast(img: &DynamicImage) -> String {
    let small = img.resize(960, 540, FilterType::Triangle);
    encode_image(&small, 60).unwrap_or_default()
}
