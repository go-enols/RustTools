//! 视频推理服务 - 高性能流水线版本
//! 
//! 架构优化：
//! 1. 流水线并行处理（Producer-Consumer）
//! 2. 动态批量推理
//! 3. 帧缓冲管理
//! 4. 增量结果更新

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::process::Stdio;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tokio::process::Command;
use image::GenericImageView;

use super::optimized_inference::OptimizedInferenceEngine;
use super::optimized_inference::DetectionResult;
use crate::modules::yolo::models::training::{AnnotationBox, VideoInferenceConfig};

/// 帧类型
#[derive(Debug, Clone, serde::Serialize)]
pub enum FrameType {
    Full,      // 完整帧
    Diff,      // 差异帧
    DetectionsOnly, // 仅检测结果
}

/// 优化的帧结构
#[derive(Debug, Clone)]
pub struct StreamingFrame {
    pub index: u32,
    pub timestamp_ms: u64,
    pub image: image::DynamicImage,
    pub detections: Vec<DetectionResult>,
}

/// 流水线配置
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub buffer_size: usize,        // 帧缓冲大小
    pub batch_size: usize,          // 批量大小
    pub batch_timeout_ms: u64,      // 批次超时
    pub num_workers: usize,        // 工作线程数
    pub enable_pipeline: bool,     // 启用流水线
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            buffer_size: 32,
            batch_size: 8,
            batch_timeout_ms: 100,
            num_workers: num_cpus::get(),
            enable_pipeline: true,
        }
    }
}

/// 推理会话状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct InferenceSessionState {
    pub session_id: String,
    pub is_running: bool,
    pub processed_frames: u32,
    pub total_frames: u32,
    pub fps: f32,
    pub avg_inference_time_ms: f64,
}

/// 视频推理服务 - 高性能版本
pub struct StreamingVideoInferenceService {
    sessions: Arc<Mutex<Vec<InferenceSession>>>,
    config: PipelineConfig,
}

struct InferenceSession {
    id: String,
    video_path: String,
    model_path: String,
    is_running: Arc<Mutex<bool>>,
    state: InferenceSessionState,
}

impl StreamingVideoInferenceService {
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(Vec::new())),
            config,
        }
    }
    
    /// 获取视频信息
    pub async fn probe_video(&self, video_path: &str) -> Result<VideoStreamInfo, String> {
        let path = PathBuf::from(video_path);
        if !path.exists() {
            return Err("视频文件不存在".to_string());
        }
        
        let output = Command::new("ffprobe")
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
                video_path,
            ])
            .output()
            .await
            .map_err(|e| format!("ffprobe 执行失败: {}", e))?;
        
        if !output.status.success() {
            return Err(format!("ffprobe 失败: {}", output.status));
        }
        
        let json_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| format!("JSON 解析失败: {}", e))?;
        
        let video_stream = json["streams"]
            .as_array()
            .and_then(|streams| {
                streams.iter().find(|s| s["codec_type"] == "video")
            })
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
        
        Ok(VideoStreamInfo {
            duration,
            fps,
            frames: total_frames,
            width,
            height,
        })
    }
    
    /// 运行流水线推理
    pub async fn run_pipeline_inference(
        &self,
        session_id: String,
        config: &VideoInferenceConfig,
        progress_callback: impl Fn(u32, Vec<AnnotationBox>) + Send + 'static,
    ) -> Result<Vec<FrameAnnotations>, String> {
        eprintln!("[PipelineInference] Starting session: {}", session_id);
        
        // 创建会话
        let session = InferenceSession {
            id: session_id.clone(),
            video_path: config.video_path.clone(),
            model_path: config.model_path.clone(),
            is_running: Arc::new(Mutex::new(true)),
            state: InferenceSessionState {
                session_id: session_id.clone(),
                is_running: true,
                processed_frames: 0,
                total_frames: 0,
                fps: 0.0,
                avg_inference_time_ms: 0.0,
            },
        };
        
        {
            let mut sessions = self.sessions.lock().await;
            sessions.push(session);
        }
        
        // 创建帧通道
        let (frame_tx, frame_rx) = mpsc::channel::<StreamingFrame>(self.config.buffer_size);
        
        // 启动帧提取器 (Producer)
        let video_path = config.video_path.clone();
        let frame_interval = config.frame_interval;
        let running = Arc::clone(&session.is_running);
        
        let producer_handle = tokio::spawn(async move {
            if let Err(e) = Self::frame_producer(video_path, frame_interval, frame_tx, running).await {
                eprintln!("[PipelineInference] Producer error: {}", e);
            }
        });
        
        // 启动消费者 (Consumer)
        let running = Arc::clone(&session.is_running);
        let results = Arc::new(Mutex::new(Vec::new()));
        let results_clone = Arc::clone(&results);
        
        let consumer_handle = tokio::spawn(async move {
            if let Err(e) = Self::frame_consumer(
                frame_rx,
                &config.model_path,
                config.confidence,
                running,
                results_clone,
                progress_callback,
            ).await {
                eprintln!("[PipelineInference] Consumer error: {}", e);
            }
        });
        
        // 等待完成
        producer_handle.await.map_err(|e| format!("Producer join error: {:?}", e))?;
        consumer_handle.await.map_err(|e| format!("Consumer join error: {:?}", e))?;
        
        // 获取结果
        let final_results = results.lock().await.clone();
        
        // 更新会话状态
        {
            let mut sessions = self.sessions.lock().await;
            if let Some(s) = sessions.iter_mut().find(|s| s.id == session_id) {
                s.state.is_running = false;
            }
        }
        
        eprintln!("[PipelineInference] Session complete: {} frames processed", final_results.len());
        
        Ok(final_results)
    }
    
    /// 帧提取器 - 从视频中提取帧
    async fn frame_producer(
        video_path: String,
        frame_interval: f64,
        frame_tx: mpsc::Sender<StreamingFrame>,
        running: Arc<Mutex<bool>>,
    ) -> Result<(), String> {
        let mut frame_idx = 0u32;
        let mut timestamp_ms = 0u64;
        
        // 使用 ffmpeg 实时提取帧
        let mut child = Command::new("ffmpeg")
            .args([
                "-i", &video_path,
                "-vf", &format!("fps={}", frame_interval),
                "-f", "image2pipe",
                "-vcodec", "mjpeg",
                "-q:v", "2",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start ffmpeg: {}", e))?;
        
        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        
        use tokio::io::AsyncReadExt;
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut jpeg_buffer = Vec::new();
        let mut buffer = [0u8; 8192];
        
        loop {
            // 检查是否应该停止
            if !*running.lock().await {
                child.kill().await.ok();
                break;
            }
            
            // 读取 JPEG 数据
            match reader.read(&mut buffer).await {
                Ok(0) => break, // EOF
                Ok(n) => {
                    jpeg_buffer.extend_from_slice(&buffer[..n]);
                    
                    // 尝试解析 JPEG
                    while let Some(jpeg_end) = Self::find_jpeg_end(&jpeg_buffer) {
                        let jpeg_data = jpeg_buffer.drain(..jpeg_end).collect::<Vec<_>>();
                        
                        if let Ok(img) = image::load_from_memory(&jpeg_data) {
                            let frame = StreamingFrame {
                                index: frame_idx,
                                timestamp_ms,
                                image: img,
                                detections: Vec::new(),
                            };
                            
                            if frame_tx.send(frame).await.is_err() {
                                return Err("Frame channel closed".to_string());
                            }
                            
                            frame_idx += 1;
                            timestamp_ms = (frame_idx as f64 * 1000.0 / frame_interval) as u64;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[PipelineInference] Read error: {}", e);
                    break;
                }
            }
        }
        
        eprintln!("[PipelineInference] Producer finished: {} frames", frame_idx);
        Ok(())
    }
    
    /// 查找 JPEG 数据边界
    fn find_jpeg_end(data: &[u8]) -> Option<usize> {
        // JPEG EOI 标记: 0xFF 0xD9
        for i in (1..data.len()).rev() {
            if data[i] == 0xD9 && data[i-1] == 0xFF {
                return Some(i + 1);
            }
        }
        None
    }
    
    /// 帧消费者 - 执行推理
    async fn frame_consumer(
        mut frame_rx: mpsc::Receiver<StreamingFrame>,
        model_path: &str,
        confidence: f32,
        running: Arc<Mutex<bool>>,
        results: Arc<Mutex<Vec<FrameAnnotations>>>,
        progress_callback: impl Fn(u32, Vec<AnnotationBox>) + Send + 'static,
    ) -> Result<(), String> {
        // 加载推理引擎
        let engine = OptimizedInferenceEngine::load(model_path)?;
        eprintln!("[PipelineInference] Engine loaded, starting inference");
        
        // 批处理缓冲
        let mut batch: Vec<StreamingFrame> = Vec::with_capacity(8);
        let batch_timeout = Duration::from_millis(100);
        
        // 性能统计
        let mut total_time = Duration::ZERO;
        let mut frame_count = 0u32;
        let start_time = Instant::now();
        
        loop {
            // 尝试获取帧或超时
            let frame = tokio::select! {
                Some(f) = frame_rx.recv() => {
                    batch.push(f);
                    
                    // 如果达到批次大小，立即处理
                    if batch.len() >= 8 {
                        Some(batch.drain(..).collect::<Vec<_>>())
                    } else {
                        None
                    }
                }
                _ = sleep(batch_timeout) => {
                    // 超时，处理当前批次
                    if !batch.is_empty() {
                        Some(batch.drain(..).collect::<Vec<_>>())
                    } else {
                        None
                    }
                }
            };
            
            // 处理批次
            if let Some(frames) = frame {
                let batch_start = Instant::now();
                
                // 批量推理
                let images: Vec<_> = frames.iter().map(|f| &f.image).collect();
                let detection_results = engine.batch_detect(&images, confidence);
                
                let batch_time = batch_start.elapsed();
                total_time += batch_time;
                frame_count += frames.len() as u32;
                
                // 处理结果
                for (i, frame) in frames.into_iter().enumerate() {
                    let boxes: Vec<AnnotationBox> = detection_results[i]
                        .boxes
                        .iter()
                        .enumerate()
                        .map(|(j, det)| AnnotationBox {
                            id: format!("{}_{}", frame.index, j),
                            class_id: det.class_id,
                            class_name: det.class_name.clone(),
                            confidence: det.confidence,
                            x: det.x,
                            y: det.y,
                            width: det.width,
                            height: det.height,
                        })
                        .collect();
                    
                    let annotations = FrameAnnotations {
                        frame_index: frame.index,
                        timestamp_ms: frame.timestamp_ms,
                        boxes: boxes.clone(),
                    };
                    
                    // 保存结果
                    results.lock().await.push(annotations.clone());
                    
                    // 回调
                    progress_callback(frame.index, boxes);
                    
                    // 打印进度
                    if frame_count % 100 == 0 {
                        let elapsed = start_time.elapsed().as_secs_f64();
                        let fps = frame_count as f64 / elapsed;
                        let avg_time = total_time.as_secs_f64() / frame_count as f64 * 1000.0;
                        eprintln!("[PipelineInference] Progress: {} frames ({:.1} FPS, {:.1}ms avg)",
                            frame_count, fps, avg_time);
                    }
                }
            }
            
            // 检查是否应该停止
            if !*running.lock().await && batch.is_empty() {
                break;
            }
            
            // 检查通道是否关闭
            if frame_rx.is_closed() && batch.is_empty() {
                break;
            }
        }
        
        eprintln!("[PipelineInference] Consumer finished: {} frames in {:.2}s",
            frame_count, total_time.as_secs_f64());
        
        Ok(())
    }
    
    /// 停止推理
    pub async fn stop_inference(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
            *session.is_running.lock().await = false;
        }
    }
    
    /// 获取会话状态
    pub async fn get_session_state(&self, session_id: &str) -> Option<InferenceSessionState> {
        let sessions = self.sessions.lock().await;
        sessions.iter()
            .find(|s| s.id == session_id)
            .map(|s| s.state.clone())
    }
}

/// 视频流信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct VideoStreamInfo {
    pub duration: f64,
    pub fps: f64,
    pub frames: u64,
    pub width: u32,
    pub height: u32,
}

/// 帧注解
#[derive(Debug, Clone, serde::Serialize)]
pub struct FrameAnnotations {
    pub frame_index: u32,
    pub timestamp_ms: u64,
    pub boxes: Vec<AnnotationBox>,
}
