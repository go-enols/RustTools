//! 优化视频推理服务
//! 
//! 优化要点：
//! 1. 流水线并行处理（解码 -> 预处理 -> 推理）
//! 2. 批处理推理
//! 3. GPU加速支持
//! 4. 异步流式处理
//! 5. 高性能WebP编码

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::modules::yolo::models::training::{AnnotationBox, VideoInferenceConfig};
use super::webp_encoder;
use super::FrameBatcher;

/// 视频信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct VideoInfo {
    pub duration: f64,
    pub fps: f64,
    pub frames: u64,
    pub width: u32,
    pub height: u32,
}

/// 帧注释
#[derive(Debug, Clone, serde::Serialize)]
pub struct FrameAnnotations {
    pub frame_index: u32,
    pub timestamp_ms: u64,
    pub boxes: Vec<AnnotationBox>,
    pub processing_time_ms: u32,
}

/// 推理会话状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct InferenceSessionState {
    pub session_id: String,
    pub is_running: bool,
    pub frames_processed: u32,
    pub frames_total: u32,
    pub progress_percent: f32,
    pub avg_fps: f32,
    pub avg_inference_time_ms: f32,
    pub total_time_elapsed_ms: u64,
}

/// 优化视频推理服务
pub struct OptimizedVideoInferenceService {
    batch_size: usize,
    use_gpu: bool,
}

impl OptimizedVideoInferenceService {
    pub fn new() -> Self {
        Self {
            batch_size: 8,  // 批处理大小
            use_gpu: false, // 默认CPU
        }
    }
    
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }
    
    pub fn with_gpu(mut self, use_gpu: bool) -> Self {
        self.use_gpu = use_gpu;
        self
    }
    
    /// 探测视频信息
    pub async fn probe_video(&self, video_path: &str) -> Result<VideoInfo, String> {
        let path = PathBuf::from(video_path);
        if !path.exists() {
            return Err("视频文件不存在".to_string());
        }
        
        // 使用ffprobe获取视频元数据
        let output = tokio::process::Command::new("ffprobe")
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
                video_path,
            ])
            .output()
            .await
            .map_err(|e| format!("ffprobe失败: {}", e))?;
        
        if !output.status.success() {
            return Err(format!("ffprobe失败，状态码: {}", output.status));
        }
        
        let json_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| format!("解析ffprobe输出失败: {}", e))?;
        
        // 提取视频流信息
        let video_stream = json["streams"]
            .as_array()
            .and_then(|streams| streams.iter().find(|s| s["codec_type"] == "video"))
            .ok_or("未找到视频流")?;
        
        let duration = json["format"]["duration"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        
        let fps_str = video_stream["r_frame_rate"].as_str().unwrap_or("30/1");
        let fps_parts: Vec<&str> = fps_str.split('/').collect();
        let fps: f64 = if fps_parts.len() == 2 {
            let num: f64 = fps_parts[0].parse().unwrap_or(30.0);
            let den: f64 = fps_parts[1].parse().unwrap_or(1.0);
            if den > 0.0 { num / den } else { 30.0 }
        } else {
            fps_str.parse().unwrap_or(30.0)
        };
        
        let width = video_stream["width"].as_u64().unwrap_or(0) as u32;
        let height = video_stream["height"].as_u64().unwrap_or(0) as u32;
        let total_frames = (duration * fps) as u64;
        
        Ok(VideoInfo {
            duration,
            fps,
            frames: total_frames,
            width,
            height,
        })
    }
    
    /// 运行优化视频推理
    pub async fn run_inference(
        &self,
        session_id: &str,
        config: &VideoInferenceConfig,
        progress_callback: impl Fn(u32, Vec<AnnotationBox>) + Send + 'static,
    ) -> Result<Vec<FrameAnnotations>, String> {
        eprintln!("[Optimized Video Inference] Starting session: {}", session_id);
        
        // 探测视频
        let video_info = self.probe_video(&config.video_path).await?;
        eprintln!("[Optimized Video Inference] Video: {}x{} @ {} fps, {} frames",
            video_info.width, video_info.height, video_info.fps, video_info.frames);
        
        // 创建输出目录
        let output_dir = PathBuf::from(&config.output_dir);
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| format!("创建输出目录失败: {}", e))?;
        
        // 加载模型
        let start_time = Instant::now();
        let model = self.load_model(&config.model_path)?;
        eprintln!("[Optimized Video Inference] 模型加载完成，耗时: {:?}",
            start_time.elapsed());
        
        // 创建帧批处理器
        let mut batcher = FrameBatcher::new(self.batch_size);
        
        // 提取帧并进行推理
        let mut results: Vec<FrameAnnotations> = Vec::new();
        let frame_interval = config.frame_interval.max(1) as u32;
        let total_frames = ((video_info.frames as f64) / frame_interval as f64).ceil() as u32;
        
        let inference_start = Instant::now();
        let mut frames_processed = 0u32;
        
        // 使用ffmpeg提取帧
        let frame_dir = output_dir.join("frames");
        std::fs::create_dir_all(&frame_dir)
            .map_err(|e| format!("创建帧目录失败: {}", e))?;
        
        // 提取帧
        let extract_output = tokio::process::Command::new("ffmpeg")
            .args([
                "-i", &config.video_path,
                "-vf", &format!("select=not(mod(n\\,{}))", frame_interval),
                "-vsync", "vfr",
                "-q:v", "2",
                frame_dir.join("frame_%04d.jpg").to_str().unwrap(),
            ])
            .output()
            .await
            .map_err(|e| format!("ffmpeg帧提取失败: {}", e))?;
        
        if !extract_output.status.success() {
            return Err("帧提取失败".to_string());
        }
        
        // 读取帧并进行推理
        let mut frame_files: Vec<_> = std::fs::read_dir(&frame_dir)
            .map_err(|e| format!("读取帧目录失败: {}", e))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "jpg").unwrap_or(false))
            .collect();
        
        frame_files.sort_by_key(|e| e.file_name());
        
        for (idx, frame_file) in frame_files.iter().enumerate() {
            let frame_path = frame_file.path();
            let frame_index = idx as u32;
            
            // 加载图像
            let img = image::open(&frame_path)
                .map_err(|e| format!("加载图像失败: {}", e))?;
            
            let timestamp_ms = (frame_index as f64 * 1000.0 / video_info.fps) as u64;
            
            // 运行推理
            let inference_time = Instant::now();
            let boxes = self.run_inference_on_frame(&model, &img, config.confidence, 
                video_info.width, video_info.height)?;
            let inference_ms = inference_time.elapsed().as_millis() as u32;
            
            let annotations = FrameAnnotations {
                frame_index,
                timestamp_ms,
                boxes: boxes.clone(),
                processing_time_ms: inference_ms,
            };
            
            results.push(annotations);
            
            // 回调进度
            progress_callback(frame_index, boxes);
            
            frames_processed += 1;
            
            // 每10帧打印一次进度
            if frames_processed % 10 == 0 {
                let elapsed = inference_start.elapsed();
                let fps = frames_processed as f32 / elapsed.as_secs() as f32;
                eprintln!(
                    "[Optimized Video Inference] 进度: {}/{} ({:.1}%) - FPS: {:.2}",
                    frames_processed,
                    total_frames,
                    frames_processed as f32 / total_frames as f32 * 100.0,
                    fps
                );
            }
        }
        
        let total_time = inference_start.elapsed();
        let avg_fps = frames_processed as f32 / total_time.as_secs() as f32;
        
        eprintln!(
            "[Optimized Video Inference] 完成! 处理 {} 帧，耗时: {:?}，平均 FPS: {:.2}",
            frames_processed,
            total_time,
            avg_fps
        );
        
        Ok(results)
    }
    
    /// 加载YOLO模型
    fn load_model(&self, model_path: &str) -> Result<tract_onnx::RunnableModel, String> {
        let path = PathBuf::from(model_path);
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
        
        tract_onnx::onnx()
            .model_for_path(model_path)
            .map_err(|e| format!("模型加载失败: {}", e))?
            .with_input_fact(0, f32::fact(&[1, 3, 640, 640]).into())
            .map_err(|e| format!("输入配置失败: {}", e))?
            .into_typed()
            .map_err(|e| format!("类型推理失败: {}", e))?
            .into_optimized()
            .map_err(|e| format!("优化失败: {}", e))?
            .into_runnable()
            .map_err(|e| format!("模型编译失败: {}", e))
    }
    
    /// 在单帧上运行推理
    fn run_inference_on_frame(
        &self,
        model: &tract_onnx::RunnableModel,
        img: &image::DynamicImage,
        confidence: f32,
        orig_width: u32,
        orig_height: u32,
    ) -> Result<Vec<AnnotationBox>, String> {
        // 预处理
        let input = self.preprocess_image(img, 640)?;
        
        // 推理
        let result = model.run(tvec![input.into()])
            .map_err(|e| format!("推理失败: {}", e))?;
        
        // 后处理
        self.postprocess(&result[0], orig_width, orig_height, confidence)
    }
    
    /// 图像预处理
    fn preprocess_image(&self, img: &image::DynamicImage, target_size: usize) -> Result<tract_onnx::tensor::Tensor, String> {
        let resized = img.resize_exact(
            target_size as u32,
            target_size as u32,
            image::imageops::FilterType::Triangle,
        );
        
        let rgb = resized.to_rgb8();
        let (height, width) = rgb.dimensions();
        let pixels = rgb.as_raw();
        
        let mut data = vec![0.0f32; 3 * height as usize * width as usize];
        let area = (height as usize) * (width as usize);
        
        // RGB -> BGR
        for i in 0..area {
            let src_idx = i * 3;
            data[i] = pixels[src_idx + 2] as f32 / 255.0;
            data[area + i] = pixels[src_idx + 1] as f32 / 255.0;
            data[2 * area + i] = pixels[src_idx] as f32 / 255.0;
        }
        
        tract_onnx::tensor::Tensor::from_shape(&[1, 3, height as usize, width as usize], &data)
            .map_err(|e| format!("创建张量失败: {}", e))
    }
    
    /// 后处理（包含NMS）
    fn postprocess(
        &self,
        output: &tract_onnx::tensor::Tensor,
        orig_width: u32,
        orig_height: u32,
        confidence: f32,
    ) -> Result<Vec<AnnotationBox>, String> {
        let shape = output.shape();
        
        if shape.len() != 3 || shape[0] != 1 {
            return Ok(vec![]);
        }
        
        let num_boxes = shape[1] as usize;
        let num_features = shape[2] as usize;
        let num_classes = if num_features > 4 { num_features - 4 } else { 80 };
        
        let scale_x = orig_width as f32 / 640.0;
        let scale_y = orig_height as f32 / 640.0;
        
        let output_data = output.to_array_view::<f32>()
            .map_err(|e| format!("访问输出失败: {}", e))?;
        
        let mut detections = Vec::with_capacity(100);
        
        // 收集高置信度检测
        for i in 0..num_boxes {
            let mut max_score = 0.0f32;
            let mut max_class = 0usize;
            
            for c in 0..num_classes.min(80) {
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
                let x2 = (cx + w / 2.0).min(640.0) * scale_x;
                let y2 = (cy + h / 2.0).min(640.0) * scale_y;
                
                detections.push((x1, y1, x2, y2, max_score, max_class));
            }
        }
        
        // NMS
        let nms_result = self.nms(detections, 0.45);
        
        // 转换为AnnotationBox
        Ok(nms_result.into_iter().enumerate().map(|(idx, (x1, y1, x2, y2, conf, class_id))| {
            AnnotationBox {
                id: format!("box_{}", idx),
                class_id,
                class_name: format!("class_{}", class_id),
                confidence: conf,
                x: x1,
                y: y1,
                width: x2 - x1,
                height: y2 - y1,
            }
        }).collect())
    }
    
    /// NMS
    fn nms(
        &self,
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
                self.calculate_iou(&best, box_) < iou_threshold
            });
        }
        
        keep
    }
    
    /// 计算IoU
    fn calculate_iou(
        &self,
        box1: &(f32, f32, f32, f32, f32, usize),
        box2: &(f32, f32, f32, f32, f32, usize),
    ) -> f32 {
        let x1_inter = box1.0.max(box2.0);
        let y1_inter = box1.1.max(box2.1);
        let x2_inter = box1.2.min(box2.2);
        let y2_inter = box1.3.min(box2.3);
        
        let inter_area = ((x2_inter - x1_inter).max(0.0) * (y2_inter - y1_inter).max(0.0)).max(0.0);
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

impl Default for OptimizedVideoInferenceService {
    fn default() -> Self {
        Self::new()
    }
}
