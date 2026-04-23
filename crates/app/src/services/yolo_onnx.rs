//! YOLOv11 ONNX 推理引擎（纯 Rust，ort / ONNX Runtime）
//!
//! 使用 `ort` (ONNX Runtime Rust 绑定)，CPU 推理性能远高于 tract-onnx。
//! 输入: yolo11n.onnx（ultralytics 导出，输入 [1,3,640,640]，输出 [1,84,8400]）

use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::TensorRef;

#[cfg(feature = "cuda")]
use ort::execution_providers::CUDAExecutionProvider;

/// COCO 80 类标签
pub const COCO_CLASSES: &[&str] = &[
    "person", "bicycle", "car", "motorcycle", "airplane", "bus", "train", "truck", "boat",
    "traffic light", "fire hydrant", "stop sign", "parking meter", "bench", "bird", "cat",
    "dog", "horse", "sheep", "cow", "elephant", "bear", "zebra", "giraffe", "backpack",
    "umbrella", "handbag", "tie", "suitcase", "frisbee", "skis", "snowboard", "sports ball",
    "kite", "baseball bat", "baseball glove", "skateboard", "surfboard", "tennis racket",
    "bottle", "wine glass", "cup", "fork", "knife", "spoon", "bowl", "banana", "apple",
    "sandwich", "orange", "broccoli", "carrot", "hot dog", "pizza", "donut", "cake", "chair",
    "couch", "potted plant", "bed", "dining table", "toilet", "tv", "laptop", "mouse",
    "remote", "keyboard", "cell phone", "microwave", "oven", "toaster", "sink", "refrigerator",
    "book", "clock", "vase", "scissors", "teddy bear", "hair drier", "toothbrush",
];

pub const NUM_CLASSES: usize = 80;

/// 单个检测结果
#[derive(Debug, Clone)]
pub struct OnnxDetection {
    pub class_id: usize,
    pub confidence: f32,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

/// ONNX 推理引擎
pub struct YoloOnnxEngine {
    session: Session,
    conf_threshold: f32,
    nms_threshold: f32,
    input_size: usize,
    /// 预分配的输入 buffer，避免每帧重新分配 [1,3,640,640] f32
    input_buffer: Vec<f32>,
}

impl YoloOnnxEngine {
    pub fn new(model_path: &str) -> Result<Self, String> {
        let start = std::time::Instant::now();

        // 尝试多个路径定位模型
        let path = std::path::PathBuf::from(model_path);
        let model_path = if path.exists() {
            path
        } else if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let p = dir.join(model_path);
                if p.exists() { p } else { path }
            } else { path }
        } else {
            path
        };

        eprintln!("[YOLO-ONNX] Loading model: {}", model_path.display());

        let mut builder = Session::builder()
            .map_err(|e| format!("Session builder 失败: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| format!("优化级别设置失败: {}", e))?
            .with_intra_threads(4)
            .map_err(|e| format!("线程设置失败: {}", e))?;

        #[cfg(feature = "cuda")]
        {
            match builder.with_execution_providers([CUDAExecutionProvider::default().build()]) {
                Ok(b) => builder = b,
                Err(e) => eprintln!("[YOLO-ONNX] CUDA 初始化失败，回退到 CPU: {}", e),
            }
        }

        let session = builder
            .commit_from_file(&model_path)
            .map_err(|e| format!("模型加载失败: {}", e))?;

        eprintln!(
            "[YOLO-ONNX] Model loaded in {}ms",
            start.elapsed().as_millis()
        );

        Ok(Self {
            session,
            conf_threshold: 0.25,
            nms_threshold: 0.45,
            input_size: 640,
            input_buffer: Vec::with_capacity(3 * 640 * 640),
        })
    }

    pub fn set_conf_threshold(&mut self, conf: f32) {
        self.conf_threshold = conf.clamp(0.01, 1.0);
    }

    /// 对截屏图像进行推理
    pub fn infer(&mut self, image: &image::DynamicImage) -> Result<Vec<OnnxDetection>, String> {
        let preprocess_start = std::time::Instant::now();
        let (input_shape, input_data) = self.preprocess(image)?;
        let preprocess_ms = preprocess_start.elapsed().as_secs_f64() * 1000.0;

        let infer_start = std::time::Instant::now();
        let input_tensor = TensorRef::<f32>::from_array_view((input_shape, &*input_data))
            .map_err(|e| format!("构建输入 tensor 失败: {}", e))?;
        let outputs = self.session
            .run(ort::inputs!["images" => input_tensor])
            .map_err(|e| format!("推理失败: {}", e))?;
        let infer_ms = infer_start.elapsed().as_secs_f64() * 1000.0;

        let post_start = std::time::Instant::now();
        let (_shape, output_data) = outputs["output0"]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("提取输出失败: {}", e))?;
        let output_vec = output_data.to_vec();
        drop(outputs); // 释放对 self.session 的借用
        let detections = Self::postprocess(
            &output_vec, image.width(), image.height(),
            self.conf_threshold, self.nms_threshold, self.input_size,
        )?;
        let post_ms = post_start.elapsed().as_secs_f64() * 1000.0;

        // 只在预处理或后处理耗时异常时打印，避免高帧率下的 IO 开销
        if preprocess_ms > 5.0 || post_ms > 5.0 {
            eprintln!(
                "[YOLO-ONNX] pre={:.1}ms infer={:.1}ms post={:.1}ms dets={}",
                preprocess_ms, infer_ms, post_ms, detections.len()
            );
        }

        Ok(detections)
    }

    /// 直接从 BGRA 原始数据进行推理（零拷贝 fast path，供 scrap 使用）
    pub fn infer_from_bgra(
        &mut self,
        bgra: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Vec<OnnxDetection>, String> {
        let preprocess_start = std::time::Instant::now();
        let input_shape = self.preprocess_fast_bgra(bgra, width, height);
        let preprocess_ms = preprocess_start.elapsed().as_secs_f64() * 1000.0;

        let infer_start = std::time::Instant::now();
        let input_tensor = TensorRef::<f32>::from_array_view((input_shape, &*self.input_buffer))
            .map_err(|e| format!("构建输入 tensor 失败: {}", e))?;
        let outputs = self.session
            .run(ort::inputs!["images" => input_tensor])
            .map_err(|e| format!("推理失败: {}", e))?;
        let infer_ms = infer_start.elapsed().as_secs_f64() * 1000.0;

        let post_start = std::time::Instant::now();
        let (_shape, output_data) = outputs["output0"]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("提取输出失败: {}", e))?;
        let detections = Self::postprocess(
            output_data, width, height,
            self.conf_threshold, self.nms_threshold, self.input_size,
        )?;
        drop(outputs);
        let post_ms = post_start.elapsed().as_secs_f64() * 1000.0;

        eprintln!(
            "[YOLO-ONNX] pre={:.1}ms infer={:.1}ms post={:.1}ms dets={}",
            preprocess_ms,
            infer_ms,
            post_ms,
            detections.len()
        );

        Ok(detections)
    }

    /// Fast preprocess: BGRA → CHW [1,3,640,640]，直接从 BGRA 采样，跳过中间 RGBA 转换
    /// 使用预分配的 input_buffer，避免每帧重新分配内存
    fn preprocess_fast_bgra(&mut self, bgra: &[u8], src_w: u32, src_h: u32) -> [usize; 4] {
        let dst_size = self.input_size as u32;
        self.input_buffer.clear();
        let scale_x = src_w as f32 / dst_size as f32;
        let scale_y = src_h as f32 / dst_size as f32;
        for c in 0..3 {
            // BGRA: B=0, G=1, R=2; 目标 CHW 顺序: R=0, G=1, B=2
            let src_c = 2 - c;
            for dy in 0..dst_size {
                for dx in 0..dst_size {
                    let sx = (dx as f32 * scale_x) as u32;
                    let sy = (dy as f32 * scale_y) as u32;
                    let src_idx = ((sy * src_w + sx) * 4 + src_c) as usize;
                    self.input_buffer.push(bgra[src_idx] as f32 / 255.0);
                }
            }
        }
        [1, 3, self.input_size, self.input_size]
    }

    fn preprocess(&self, image: &image::DynamicImage) -> Result<([usize; 4], Vec<f32>), String> {
        let t1 = std::time::Instant::now();
        let rgba = image.to_rgba8();
        let t1_ms = t1.elapsed().as_secs_f64() * 1000.0;

        let t2 = std::time::Instant::now();
        let dst_size = self.input_size as u32;
        let resized = image::imageops::resize(
            &rgba,
            dst_size,
            dst_size,
            image::imageops::FilterType::Nearest, // Nearest is fastest
        );
        let t2_ms = t2.elapsed().as_secs_f64() * 1000.0;

        let t3 = std::time::Instant::now();
        // Convert RGBA → RGB CHW normalized [1, 3, 640, 640]
        let mut tensor_data: Vec<f32> = Vec::with_capacity(3 * self.input_size * self.input_size);
        for c in 0..3 {
            for y in 0..dst_size {
                for x in 0..dst_size {
                    let idx = ((y * dst_size + x) * 4 + c) as usize;
                    let val = resized.get(idx).copied().unwrap_or(0) as f32 / 255.0;
                    tensor_data.push(val);
                }
            }
        }
        let t3_ms = t3.elapsed().as_secs_f64() * 1000.0;
        eprintln!("[YOLO-ONNX-pre] to_rgba8={:.1}ms resize={:.1}ms convert={:.1}ms img={}x{}", 
            t1_ms, t2_ms, t3_ms, image.width(), image.height());

        Ok(([1, 3, self.input_size, self.input_size], tensor_data))
    }

    /// 后处理：从 ONNX 输出 [1,84,8400] 中提取检测框
    /// 作为关联函数，避免在 outputs 存活期间 borrow self
    fn postprocess(
        data: &[f32],
        orig_w: u32,
        orig_h: u32,
        conf_threshold: f32,
        nms_threshold: f32,
        input_size: usize,
    ) -> Result<Vec<OnnxDetection>, String> {
        let num_anchors = 8400;
        let num_features = 84;
        let expected_len = num_features * num_anchors;
        if data.len() < expected_len {
            return Err(format!(
                "输出数据长度不足: {} < {}",
                data.len(),
                expected_len
            ));
        }

        let scale_x = orig_w as f32 / input_size as f32;
        let scale_y = orig_h as f32 / input_size as f32;

        // Collect candidates
        let mut candidates: Vec<(f32, f32, f32, f32, f32, usize)> = Vec::with_capacity(200);
        for i in 0..num_anchors {
            let cx = data[0 * num_anchors + i];
            let cy = data[1 * num_anchors + i];
            let w = data[2 * num_anchors + i];
            let h = data[3 * num_anchors + i];

            let mut max_conf = 0.0f32;
            let mut max_class = 0usize;
            for c in 0..NUM_CLASSES {
                let conf = data[(4 + c) * num_anchors + i];
                if conf > max_conf {
                    max_conf = conf;
                    max_class = c;
                }
            }

            if max_conf >= conf_threshold {
                let x1 = (cx - w / 2.0).max(0.0);
                let y1 = (cy - h / 2.0).max(0.0);
                let x2 = (cx + w / 2.0).min(input_size as f32);
                let y2 = (cy + h / 2.0).min(input_size as f32);
                candidates.push((x1, y1, x2, y2, max_conf, max_class));
            }
        }

        // NMS
        candidates.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));
        let mut suppressed = vec![false; candidates.len()];
        let mut detections = Vec::with_capacity(candidates.len().min(100));

        for i in 0..candidates.len() {
            if suppressed[i] {
                continue;
            }
            let (x1, y1, x2, y2, conf, cls) = candidates[i];
            detections.push(OnnxDetection {
                class_id: cls,
                confidence: conf,
                x1: x1 * scale_x,
                y1: y1 * scale_y,
                x2: x2 * scale_x,
                y2: y2 * scale_y,
            });

            for j in (i + 1)..candidates.len() {
                if suppressed[j] || candidates[j].5 != cls {
                    continue;
                }
                let iou = compute_iou(
                    candidates[i].0,
                    candidates[i].1,
                    candidates[i].2,
                    candidates[i].3,
                    candidates[j].0,
                    candidates[j].1,
                    candidates[j].2,
                    candidates[j].3,
                );
                if iou > nms_threshold {
                    suppressed[j] = true;
                }
            }
        }

        Ok(detections)
    }
}

fn compute_iou(x1: f32, y1: f32, x2: f32, y2: f32, x1b: f32, y1b: f32, x2b: f32, y2b: f32) -> f32 {
    let ix1 = x1.max(x1b);
    let iy1 = y1.max(y1b);
    let ix2 = x2.min(x2b);
    let iy2 = y2.min(y2b);
    let inter = (ix2 - ix1).max(0.0) * (iy2 - iy1).max(0.0);
    let area_a = (x2 - x1) * (y2 - y1);
    let area_b = (x2b - x1b) * (y2b - y1b);
    let union = area_a + area_b - inter;
    if union <= 0.0 {
        0.0
    } else {
        inter / union
    }
}
