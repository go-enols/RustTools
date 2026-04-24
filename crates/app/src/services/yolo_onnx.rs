//! YOLOv11 ONNX 推理引擎（纯 Rust，tract-onnx）
//!
//! 使用 `tract-onnx` 纯 Rust ONNX 推理引擎，无 C++ FFI 依赖。
//! 输入: yolo11n.onnx（ultralytics 导出，输入 [1,3,640,640]，输出 [1,84,8400]）

use tract_onnx::prelude::*;

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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    plan: TypedSimplePlan<TypedModel>,
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

        let model = tract_onnx::onnx()
            .model_for_path(&model_path)
            .map_err(|e| format!("加载 ONNX 模型失败: {}", e))?
            .into_optimized()
            .map_err(|e| format!("优化模型失败: {}", e))?;

        let plan = SimplePlan::new(model)
            .map_err(|e| format!("创建执行计划失败: {}", e))?;

        eprintln!(
            "[YOLO-ONNX] Model loaded in {}ms (tract-onnx)",
            start.elapsed().as_millis()
        );

        Ok(Self {
            plan,
            conf_threshold: 0.25,
            nms_threshold: 0.45,
            input_size: 640,
            input_buffer: Vec::with_capacity(3 * 640 * 640),
        })
    }

    pub fn set_conf_threshold(&mut self, conf: f32) {
        self.conf_threshold = conf.clamp(0.01, 1.0);
    }

    /// 对 RGBA 截屏数据进行零拷贝推理
    pub fn infer_from_rgba(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Vec<OnnxDetection>, String> {
        let preprocess_start = std::time::Instant::now();
        let input_shape = self.preprocess_fast_rgba(rgba, width, height);
        let preprocess_ms = preprocess_start.elapsed().as_secs_f64() * 1000.0;

        let infer_start = std::time::Instant::now();
        let input = Tensor::from_shape(&input_shape, &self.input_buffer)
            .map_err(|e| format!("创建输入张量失败: {}", e))?;
        let result = self.plan.run(tvec!(input.into()))
            .map_err(|e| format!("推理失败: {}", e))?;
        let infer_ms = infer_start.elapsed().as_secs_f64() * 1000.0;

        let post_start = std::time::Instant::now();
        let output = &result[0];
        let data = output.to_array_view::<f32>()
            .map_err(|e| format!("读取输出失败: {}", e))?;
        let data_slice = data.as_slice()
            .ok_or("输出数据非连续布局")?;
        let detections = Self::postprocess(
            data_slice, width, height,
            self.conf_threshold, self.nms_threshold, self.input_size,
        )?;
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

    /// Fast preprocess: RGBA → CHW [1,3,640,640]，手写循环，零中间分配
    fn preprocess_fast_rgba(&mut self, rgba: &[u8], src_w: u32, src_h: u32) -> [usize; 4] {
        let dst_size = self.input_size as u32;
        let total = 3 * self.input_size * self.input_size;
        self.input_buffer.resize(total, 0.0);

        let scale_x = src_w as f32 / dst_size as f32;
        let scale_y = src_h as f32 / dst_size as f32;

        // 预计算采样位置，消除每像素浮点运算
        let sx_tab: Vec<u32> = (0..dst_size)
            .map(|dx| (dx as f32 * scale_x).min(src_w as f32 - 1.0) as u32)
            .collect();
        let sy_tab: Vec<u32> = (0..dst_size)
            .map(|dy| (dy as f32 * scale_y).min(src_h as f32 - 1.0) as u32)
            .collect();

        for c in 0..3 {
            let src_c = c as u32; // RGBA: R=0, G=1, B=2
            let dst_offset = c * self.input_size * self.input_size;
            for dy in 0..dst_size as usize {
                let sy = sy_tab[dy];
                let src_row_offset = sy * src_w * 4;
                let dst_row_offset = dst_offset + dy * self.input_size;
                for dx in 0..dst_size as usize {
                    let sx = sx_tab[dx];
                    let src_idx = (src_row_offset + sx * 4 + src_c) as usize;
                    self.input_buffer[dst_row_offset + dx] = rgba[src_idx] as f32 * (1.0 / 255.0);
                }
            }
        }
        [1, 3, self.input_size, self.input_size]
    }

    /// 对图片文件进行推理
    pub fn infer(&mut self, image: &image::DynamicImage) -> Result<Vec<OnnxDetection>, String> {
        let preprocess_start = std::time::Instant::now();
        let (input_shape, input_data) = self.preprocess(image)?;
        let preprocess_ms = preprocess_start.elapsed().as_secs_f64() * 1000.0;

        let infer_start = std::time::Instant::now();
        let input = Tensor::from_shape(&input_shape, &input_data)
            .map_err(|e| format!("创建输入张量失败: {}", e))?;
        let result = self.plan.run(tvec!(input.into()))
            .map_err(|e| format!("推理失败: {}", e))?;
        let infer_ms = infer_start.elapsed().as_secs_f64() * 1000.0;

        let post_start = std::time::Instant::now();
        let output = &result[0];
        let data = output.to_array_view::<f32>()
            .map_err(|e| format!("读取输出失败: {}", e))?;
        let data_vec: Vec<f32> = data.iter().cloned().collect();
        let detections = Self::postprocess(
            &data_vec, image.width(), image.height(),
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
        let input = Tensor::from_shape(&input_shape, &self.input_buffer)
            .map_err(|e| format!("创建输入张量失败: {}", e))?;
        let result = self.plan.run(tvec!(input.into()))
            .map_err(|e| format!("推理失败: {}", e))?;
        let infer_ms = infer_start.elapsed().as_secs_f64() * 1000.0;

        let post_start = std::time::Instant::now();
        let output = &result[0];
        let data = output.to_array_view::<f32>()
            .map_err(|e| format!("读取输出失败: {}", e))?;
        let data_slice = data.as_slice()
            .ok_or("输出数据非连续布局")?;
        let detections = Self::postprocess(
            data_slice, width, height,
            self.conf_threshold, self.nms_threshold, self.input_size,
        )?;
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

    /// Fast preprocess: BGRA → CHW [1,3,640,640]，手写循环，直接从 BGRA 采样
    fn preprocess_fast_bgra(&mut self, bgra: &[u8], src_w: u32, src_h: u32) -> [usize; 4] {
        let dst_size = self.input_size as u32;
        let total = 3 * self.input_size * self.input_size;
        self.input_buffer.resize(total, 0.0);

        let scale_x = src_w as f32 / dst_size as f32;
        let scale_y = src_h as f32 / dst_size as f32;

        // 预计算采样位置，消除每像素浮点运算
        let sx_tab: Vec<u32> = (0..dst_size)
            .map(|dx| (dx as f32 * scale_x).min(src_w as f32 - 1.0) as u32)
            .collect();
        let sy_tab: Vec<u32> = (0..dst_size)
            .map(|dy| (dy as f32 * scale_y).min(src_h as f32 - 1.0) as u32)
            .collect();

        for c in 0..3 {
            let src_c = (2 - c) as u32; // BGRA: B=0, G=1, R=2 → CHW: R=0, G=1, B=2
            let dst_offset = c * self.input_size * self.input_size;
            for dy in 0..dst_size as usize {
                let sy = sy_tab[dy];
                let src_row_offset = sy * src_w * 4;
                let dst_row_offset = dst_offset + dy * self.input_size;
                for dx in 0..dst_size as usize {
                    let sx = sx_tab[dx];
                    let src_idx = (src_row_offset + sx * 4 + src_c) as usize;
                    self.input_buffer[dst_row_offset + dx] = bgra[src_idx] as f32 * (1.0 / 255.0);
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

    /// 后处理：自动检测 ONNX 输出格式并解析
    /// 支持两种格式：
    /// - 传统 YOLO [1, 84, 8400]: 84 = 4 bbox + 80 classes, 8400 anchors
    /// - 端到端 [1, N, 6]: N 个检测结果，每个 = [x1, y1, x2, y2, conf, class]
    fn postprocess(
        data: &[f32],
        orig_w: u32,
        orig_h: u32,
        conf_threshold: f32,
        nms_threshold: f32,
        input_size: usize,
    ) -> Result<Vec<OnnxDetection>, String> {
        const LEGACY_LEN: usize = 84 * 8400; // 705600

        match data.len() {
            LEGACY_LEN => Self::postprocess_legacy(
                data, orig_w, orig_h, conf_threshold, nms_threshold, input_size,
            ),
            len if len >= 6 && len % 6 == 0 => {
                let num_dets = len / 6;
                // 启发式判断 [N,6] 还是 [6,N] 布局：
                // 扫描前50个检测框，统计两种格式下 conf(0-1) 和 class(0-100) 同时合理的数量
                let valid_interleaved = (0..num_dets.min(50))
                    .filter(|&i| {
                        let conf = data.get(i * 6 + 4).copied().unwrap_or(-1.0);
                        let class = data.get(i * 6 + 5).copied().unwrap_or(-1.0);
                        conf >= 0.0 && conf <= 1.0 && class >= 0.0 && class <= 100.0
                    })
                    .count();
                let valid_non_interleaved = (0..num_dets.min(50))
                    .filter(|&i| {
                        let conf = data.get(4 * num_dets + i).copied().unwrap_or(-1.0);
                        let class = data.get(5 * num_dets + i).copied().unwrap_or(-1.0);
                        conf >= 0.0 && conf <= 1.0 && class >= 0.0 && class <= 100.0
                    })
                    .count();
                let interleaved = valid_interleaved >= valid_non_interleaved;
                eprintln!("[YOLO-ONNX] format: interleaved={}, valid_i={}, valid_ni={}",
                         interleaved, valid_interleaved, valid_non_interleaved);
                Self::postprocess_end2end(
                    data, num_dets, interleaved, orig_w, orig_h, conf_threshold, input_size,
                )
            }
            _ => Err(format!(
                "不支持的 ONNX 输出格式: 数据长度 {}。期望 {} (YOLOv8 传统 [1,84,8400]) 或 6 的倍数 (端到端 [1,N,6])",
                data.len(), LEGACY_LEN
            )),
        }
    }

    /// 传统 YOLO 后处理：[1, 84, 8400]
    fn postprocess_legacy(
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

    /// 端到端后处理：[1, N, 6] 或 [1, 6, N]
    /// 每个检测 = [x1, y1, x2, y2, conf, class]，通常已内置 NMS
    fn postprocess_end2end(
        data: &[f32],
        num_dets: usize,
        interleaved: bool, // true: [N,6], false: [6,N]
        orig_w: u32,
        orig_h: u32,
        conf_threshold: f32,
        input_size: usize,
    ) -> Result<Vec<OnnxDetection>, String> {
        let scale_x = orig_w as f32 / input_size as f32;
        let scale_y = orig_h as f32 / input_size as f32;

        // 判断坐标范围：采样所有检测的 x2 最大值
        // 端到端模型的坐标可能是 0-1 归一化或 0-input_size 模型空间
        let max_x2 = if interleaved {
            (0..num_dets)
                .map(|i| data.get(i * 6 + 2).copied().unwrap_or(0.0))
                .fold(0.0f32, f32::max)
        } else {
            (0..num_dets)
                .map(|i| data.get(2 * num_dets + i).copied().unwrap_or(0.0))
                .fold(0.0f32, f32::max)
        };
        // 判断坐标是否归一化：归一化坐标最大≈1.0，模型空间坐标通常在 1~640 范围。
        // 使用阈值 2.0 更安全，避免恰好 x2=1.0 的边界情况。
        let coords_normalized = max_x2 < 2.0;

        // 调试：打印坐标范围判断结果和前几个有效检测
        if let Some(first_valid) = (0..num_dets.min(20)).find(|&i| {
            let conf = if interleaved { data[i * 6 + 4] } else { data[4 * num_dets + i] };
            conf >= conf_threshold
        }) {
            let (x1, y1, x2, y2, conf, _cls) = if interleaved {
                let b = first_valid * 6;
                (data[b], data[b+1], data[b+2], data[b+3], data[b+4], data[b+5])
            } else {
                (data[0*num_dets+first_valid], data[1*num_dets+first_valid],
                 data[2*num_dets+first_valid], data[3*num_dets+first_valid],
                 data[4*num_dets+first_valid], data[5*num_dets+first_valid])
            };
            eprintln!("[YOLO-ONNX] max_x2={:.2} normalized={} | first_valid det[{}]: x1={:.2} y1={:.2} x2={:.2} y2={:.2} conf={:.3}",
                     max_x2, coords_normalized, first_valid, x1, y1, x2, y2, conf);
        }

        let mut detections = Vec::with_capacity(num_dets.min(100));

        for i in 0..num_dets {
            let (x1, y1, x2, y2, conf, class_id) = if interleaved {
                let base = i * 6;
                if base + 5 >= data.len() { break; }
                (
                    data[base + 0],
                    data[base + 1],
                    data[base + 2],
                    data[base + 3],
                    data[base + 4],
                    data[base + 5] as usize,
                )
            } else {
                if 5 * num_dets + i >= data.len() { break; }
                (
                    data[0 * num_dets + i],
                    data[1 * num_dets + i],
                    data[2 * num_dets + i],
                    data[3 * num_dets + i],
                    data[4 * num_dets + i],
                    data[5 * num_dets + i] as usize,
                )
            };

            if conf < conf_threshold {
                continue;
            }

            let (x1, y1, x2, y2) = if coords_normalized {
                (
                    x1 * input_size as f32,
                    y1 * input_size as f32,
                    x2 * input_size as f32,
                    y2 * input_size as f32,
                )
            } else {
                (x1, y1, x2, y2)
            };

            detections.push(OnnxDetection {
                class_id,
                confidence: conf,
                x1: x1 * scale_x,
                y1: y1 * scale_y,
                x2: x2 * scale_x,
                y2: y2 * scale_y,
            });
        }

        // 端到端格式通常已做 NMS，但做一遍也不影响
        detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        detections.truncate(100);

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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coco_classes_length_is_80() {
        assert_eq!(COCO_CLASSES.len(), 80, "COCO_CLASSES should have 80 classes");
        assert_eq!(NUM_CLASSES, 80, "NUM_CLASSES should be 80");
    }

    #[test]
    fn test_coco_classes_first_and_last() {
        assert_eq!(COCO_CLASSES[0], "person", "First class should be person");
        assert_eq!(COCO_CLASSES[79], "toothbrush", "Last class should be toothbrush");
    }

    #[test]
    fn test_compute_iou_identical_boxes() {
        let iou = compute_iou(0.0, 0.0, 10.0, 10.0, 0.0, 0.0, 10.0, 10.0);
        assert!((iou - 1.0).abs() < 1e-5, "Identical boxes should have IoU = 1.0");
    }

    #[test]
    fn test_compute_iou_no_overlap() {
        let iou = compute_iou(0.0, 0.0, 10.0, 10.0, 20.0, 20.0, 30.0, 30.0);
        assert!((iou - 0.0).abs() < 1e-5, "Non-overlapping boxes should have IoU = 0.0");
    }

    #[test]
    fn test_compute_iou_partial_overlap() {
        // Box A: (0,0) to (10,10), area = 100
        // Box B: (5,5) to (15,15), area = 100
        // Intersection: (5,5) to (10,10), area = 25
        // Union: 100 + 100 - 25 = 175
        // IoU: 25 / 175 = 0.142857...
        let iou = compute_iou(0.0, 0.0, 10.0, 10.0, 5.0, 5.0, 15.0, 15.0);
        let expected = 25.0 / 175.0;
        assert!(
            (iou - expected).abs() < 1e-5,
            "Partial overlap IoU should be ~{:.5}, got {:.5}",
            expected,
            iou
        );
    }

    #[test]
    fn test_onnx_detection_serialize() {
        let det = OnnxDetection {
            class_id: 5,
            confidence: 0.95,
            x1: 10.0,
            y1: 20.0,
            x2: 100.0,
            y2: 200.0,
        };
        let json = serde_json::to_string(&det).unwrap();
        assert!(json.contains("\"class_id\":5"), "Serialized JSON should contain class_id");
        assert!(json.contains("\"confidence\":0.95"), "Serialized JSON should contain confidence");

        let deserialized: OnnxDetection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.class_id, 5);
        assert!((deserialized.confidence - 0.95).abs() < 1e-5);
        assert_eq!(deserialized.x1, 10.0);
    }

    #[test]
    fn test_postprocess_empty_data() {
        // Empty data should return an error due to length check
        let result = YoloOnnxEngine::postprocess(
            &[],
            640,
            640,
            0.25,
            0.45,
            640,
        );
        assert!(result.is_err(), "Empty data should return error");
    }

    #[test]
    fn test_postprocess_no_detections() {
        // Create mock output data with all confidences below threshold
        let num_anchors = 8400;
        let num_features = 84;
        let mut data = vec![0.0f32; num_features * num_anchors];

        // Set one candidate with very low confidence
        data[0 * num_anchors + 0] = 320.0; // cx
        data[1 * num_anchors + 0] = 320.0; // cy
        data[2 * num_anchors + 0] = 100.0; // w
        data[3 * num_anchors + 0] = 100.0; // h
        // class confidences all near 0
        for c in 0..NUM_CLASSES {
            data[(4 + c) * num_anchors + 0] = 0.01;
        }

        let result = YoloOnnxEngine::postprocess(
            &data,
            640,
            640,
            0.25, // high threshold
            0.45,
            640,
        );
        assert!(result.is_ok(), "Postprocess should succeed");
        let detections = result.unwrap();
        assert_eq!(detections.len(), 0, "No detections should pass high threshold");
    }

    #[test]
    fn test_postprocess_single_detection() {
        let num_anchors = 8400;
        let num_features = 84;
        let mut data = vec![0.0f32; num_features * num_anchors];

        // Set one strong detection at anchor 0
        data[0 * num_anchors + 0] = 320.0; // cx
        data[1 * num_anchors + 0] = 320.0; // cy
        data[2 * num_anchors + 0] = 100.0; // w
        data[3 * num_anchors + 0] = 100.0; // h
        // class 0 (person) with high confidence
        data[4 * num_anchors + 0] = 0.9;
        for c in 1..NUM_CLASSES {
            data[(4 + c) * num_anchors + 0] = 0.01;
        }

        let result = YoloOnnxEngine::postprocess(
            &data,
            640,
            640,
            0.25,
            0.45,
            640,
        );
        assert!(result.is_ok(), "Postprocess should succeed");
        let detections = result.unwrap();
        assert_eq!(detections.len(), 1, "Should detect 1 object");
        assert_eq!(detections[0].class_id, 0, "Should be class 0 (person)");
        assert!((detections[0].confidence - 0.9).abs() < 1e-5, "Confidence should be 0.9");
    }

    #[test]
    fn test_postprocess_nms_removes_duplicates() {
        let num_anchors = 8400;
        let num_features = 84;
        let mut data = vec![0.0f32; num_features * num_anchors];

        // Set two overlapping detections of the same class
        // Anchor 0
        data[0 * num_anchors + 0] = 320.0;
        data[1 * num_anchors + 0] = 320.0;
        data[2 * num_anchors + 0] = 100.0;
        data[3 * num_anchors + 0] = 100.0;
        data[4 * num_anchors + 0] = 0.9;
        for c in 1..NUM_CLASSES {
            data[(4 + c) * num_anchors + 0] = 0.01;
        }

        // Anchor 1 - slightly offset but heavily overlapping
        data[0 * num_anchors + 1] = 325.0;
        data[1 * num_anchors + 1] = 325.0;
        data[2 * num_anchors + 1] = 100.0;
        data[3 * num_anchors + 1] = 100.0;
        data[4 * num_anchors + 1] = 0.85;
        for c in 1..NUM_CLASSES {
            data[(4 + c) * num_anchors + 1] = 0.01;
        }

        let result = YoloOnnxEngine::postprocess(
            &data,
            640,
            640,
            0.25,
            0.45, // NMS threshold
            640,
        );
        assert!(result.is_ok(), "Postprocess should succeed");
        let detections = result.unwrap();
        // NMS should suppress the lower-confidence duplicate
        assert_eq!(detections.len(), 1, "NMS should remove duplicate detection");
        assert!(
            (detections[0].confidence - 0.9).abs() < 1e-5,
            "Higher confidence detection should be kept"
        );
    }
}
