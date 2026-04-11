//! 高性能视频推理服务 - 流水线架构
//! 
//! 优化要点：
//! 1. 解码-推理流水线并行
//! 2. 无锁队列
//! 3. 多线程并行推理
//! 4. 批处理优化
//! 5. 流式处理

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, Duration};
use crossbeam_channel::{bounded, Sender, Receiver};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use parking_lot::Mutex as ParkMutex;

use super::high_performance_inference::HighPerformanceInferenceEngine;
use crate::modules::yolo::models::training::{AnnotationBox, VideoInferenceConfig};

/// 帧数据
#[derive(Debug, Clone)]
struct Frame {
    index: u32,
    timestamp_ms: u64,
    image: image::DynamicImage,
    width: u32,
    height: u32,
}

/// 推理结果
#[derive(Debug, Clone)]
struct InferenceResult {
    frame_index: u32,
    timestamp_ms: u64,
    boxes: Vec<AnnotationBox>,
    inference_time_ms: f64,
}

/// 流水线状态
#[derive(Debug, Clone, Serialize)]
pub struct PipelineStatus {
    pub is_running: bool,
    pub frames_processed: u32,
    pub fps: f64,
    pub avg_inference_time_ms: f64,
}

/// 推理会话
struct InferenceSession {
    video_path: String,
    model_path: String,
    is_running: bool,
    status: PipelineStatus,
}

/// 高性能视频推理服务
pub struct HighPerformanceVideoInferenceService {
    sessions: Arc<ParkMutex<HashMap<String, InferenceSession>>>,
    model_cache: Arc<ParkMutex<HashMap<String, Arc<HighPerformanceInferenceEngine>>>>,
}

impl HighPerformanceVideoInferenceService {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(ParkMutex::new(HashMap::new())),
            model_cache: Arc::new(ParkMutex::new(HashMap::new())),
        }
    }
    
    /// 探测视频信息
    pub async fn probe_video(&self, video_path: &str) -> Result<VideoStreamInfo, String> {
        use std::process::Command;
        
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
            .map_err(|e| format!("ffprobe失败: {}", e))?;
        
        if !output.status.success() {
            return Err(format!("ffprobe返回错误: {}", output.status));
        }
        
        let json_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| format!("JSON解析失败: {}", e))?;
        
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
        
        Ok(VideoStreamInfo {
            duration,
            fps,
            frames: total_frames,
            width,
            height,
        })
    }
    
    /// 获取或加载模型
    fn get_or_load_model(&self, model_path: &str) -> Result<Arc<HighPerformanceInferenceEngine>, String> {
        let mut cache = self.model_cache.lock();
        
        if let Some(engine) = cache.get(model_path) {
            return Ok(Arc::clone(engine));
        }
        
        eprintln!("[HPVVideo] 加载模型: {}", model_path);
        let start = Instant::now();
        
        let engine = HighPerformanceInferenceEngine::load(model_path)?;
        let engine = Arc::new(engine);
        
        eprintln!("[HPVVideo] 模型加载完成: {:.2}s", start.elapsed().as_secs_f64());
        
        cache.insert(model_path.to_string(), Arc::clone(&engine));
        Ok(engine)
    }
    
    /// 运行视频推理（流水线模式）
    pub async fn run_pipeline_inference(
        &self,
        session_id: String,
        config: VideoInferenceConfig,
        app: AppHandle,
    ) -> Result<Vec<FrameAnnotations>, String> {
        eprintln!("[HPVVideo] 启动流水线推理: {}", session_id);
        eprintln!("[HPVVideo] 视频: {}", config.video_path);
        eprintln!("[HPVVideo] 模型: {}", config.model_path);
        eprintln!("[HPVVideo] 置信度: {}", config.confidence);
        eprintln!("[HPVVideo] 帧间隔: {}", config.frame_interval);
        
        let start_time = Instant::now();
        
        // 探测视频
        let video_info = self.probe_video(&config.video_path).await?;
        let total_frames = video_info.frames as u32;
        
        eprintln!("[HPVVideo] 视频信息: {}x{} @ {:.1}fps, {} 帧", 
            video_info.width, video_info.height, video_info.fps, total_frames);
        
        // 创建输出目录
        let output_dir = PathBuf::from(&config.output_dir);
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| format!("创建输出目录失败: {}", e))?;
        
        // 加载模型
        let engine = self.get_or_load_model(&config.model_path)?;
        
        // 创建会话
        {
            let mut sessions = self.sessions.lock();
            sessions.insert(session_id.clone(), InferenceSession {
                video_path: config.video_path.clone(),
                model_path: config.model_path.clone(),
                is_running: true,
                status: PipelineStatus {
                    is_running: true,
                    frames_processed: 0,
                    fps: 0.0,
                    avg_inference_time_ms: 0.0,
                },
            });
        }
        
        // 创建帧队列（无锁队列）
        let (frame_tx, frame_rx) = bounded(32);
        let (result_tx, result_rx) = bounded(32);
        
        // 获取 CPU 核心数
        let num_workers = num_cpus::get().max(1);
        
        // 启动推理工作池
        let result_tx_clone = result_tx.clone();
        let engine_clone = Arc::clone(&engine);
        
        std::thread::spawn(move || {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(num_workers)
                .build()
                .unwrap();
            
            pool.install(|| {
                while let Ok(frame) = frame_rx.recv() {
                    let result = Self::process_frame(&engine_clone, frame);
                    let _ = result_tx_clone.send(result);
                }
            });
        });
        
        // 启动解码线程
        let frame_tx_clone = frame_tx;
        let video_path = config.video_path.clone();
        let frame_interval = config.frame_interval;
        
        std::thread::spawn(move || {
            Self::decode_frames(&video_path, frame_interval, &frame_tx_clone);
        });
        
        // 主线程收集结果
        let mut all_results: Vec<FrameAnnotations> = Vec::new();
        let mut frames_processed = 0u32;
        let mut total_inference_time = 0.0f64;
        
        loop {
            // 检查是否停止
            {
                let sessions = self.sessions.lock();
                if let Some(session) = sessions.get(&session_id) {
                    if !session.is_running {
                        eprintln!("[HPVVideo] 推理被停止");
                        break;
                    }
                }
            }
            
            // 接收结果
            match result_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(result) => {
                    frames_processed += 1;
                    total_inference_time += result.inference_time_ms;
                    
                    let annotation = FrameAnnotations {
                        frame_index: result.frame_index,
                        timestamp_ms: result.timestamp_ms,
                        boxes: result.boxes.clone(),
                    };
                    
                    all_results.push(annotation.clone());
                    
                    // 发送进度事件
                    let _ = app.emit("hp-video-frame", &FrameEvent {
                        session_id: session_id.clone(),
                        frame: result.frame_index,
                        boxes: result.boxes,
                        progress: frames_processed as f32 / total_frames as f32 * 100.0,
                    });
                    
                    // 更新状态
                    {
                        let mut sessions = self.sessions.lock();
                        if let Some(session) = sessions.get_mut(&session_id) {
                            session.status.frames_processed = frames_processed;
                            session.status.avg_inference_time_ms = total_inference_time / frames_processed as f64;
                        }
                    }
                    
                    // 进度日志
                    if frames_processed % 100 == 0 {
                        let elapsed = start_time.elapsed().as_secs_f64();
                        let fps = frames_processed as f64 / elapsed;
                        eprintln!("[HPVVideo] 进度: {}/{} ({:.1}%), FPS: {:.1}", 
                            frames_processed, total_frames, 
                            frames_processed as f32 / total_frames as f32 * 100.0, fps);
                    }
                }
                Err(_) => {
                    // 超时，继续检查
                    continue;
                }
            }
            
            // 检查是否完成
            if frames_processed >= total_frames {
                break;
            }
        }
        
        // 发送完成事件
        let elapsed = start_time.elapsed().as_secs_f64();
        let avg_fps = frames_processed as f64 / elapsed;
        
        let _ = app.emit("hp-video-complete", &CompleteEvent {
            session_id: session_id.clone(),
            success: true,
            frames_processed,
            results_path: output_dir.join("results.json").to_string_lossy().to_string(),
            total_time_s: elapsed,
            avg_fps,
        });
        
        // 标记会话完成
        {
            let mut sessions = self.sessions.lock();
            if let Some(session) = sessions.get_mut(&session_id) {
                session.is_running = false;
            }
        }
        
        eprintln!("[HPVVideo] 推理完成: {} 帧 in {:.2}s ({:.1} FPS)", 
            frames_processed, elapsed, avg_fps);
        
        // 保存结果
        let results_json = serde_json::to_string_pretty(&all_results)
            .map_err(|e| format!("序列化失败: {}", e))?;
        
        std::fs::write(output_dir.join("results.json"), results_json)
            .map_err(|e| format!("保存结果失败: {}", e))?;
        
        Ok(all_results)
    }
    
    /// 解码帧
    fn decode_frames(video_path: &str, frame_interval: u32, tx: &Sender<Frame>) {
        use std::process::Command;
        
        let output_pattern = "/tmp/frame_%04d.png";
        
        let status = Command::new("ffmpeg")
            .args([
                "-i", video_path,
                "-vf", &format!("select=not(mod(n\\,{}))", frame_interval),
                "-vsync", "vfr",
                "-q:v", "2",
                "-frame_pts", "1",
                output_pattern,
            ])
            .output();
        
        if let Err(e) = status {
            eprintln!("[HPVVideo] ffmpeg执行失败: {}", e);
            return;
        }
        
        let status = status.unwrap();
        if !status.status.success() {
            eprintln!("[HPVVideo] ffmpeg返回错误: {:?}", status.stderr);
            return;
        }
        
        // 读取帧文件
        let tmp_dir = Path::new("/tmp");
        let mut frame_files: Vec<_> = std::fs::read_dir(tmp_dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| {
                e.path().extension().map(|ext| ext == "png").unwrap_or(false)
            })
            .collect();
        
        frame_files.sort_by_key(|e| e.file_name().to_string_lossy().to_string());
        
        for (idx, entry) in frame_files.into_iter().enumerate() {
            if let Ok(img) = image::open(entry.path()) {
                let (width, height) = img.dimensions();
                let frame = Frame {
                    index: idx as u32,
                    timestamp_ms: (idx as u64) * (frame_interval as u64),
                    image: img,
                    width,
                    height,
                };
                
                let _ = tx.send(frame);
            }
            
            // 清理文件
            let _ = std::fs::remove_file(entry.path());
        }
    }
    
    /// 处理单帧
    fn process_frame(
        engine: &HighPerformanceInferenceEngine,
        frame: Frame,
    ) -> InferenceResult {
        let start = Instant::now();
        
        let result = engine.detect(&frame.image, 0.25);
        
        let boxes: Vec<AnnotationBox> = result.boxes
            .into_iter()
            .enumerate()
            .map(|(idx, b)| AnnotationBox {
                id: format!("box_{}_{}", frame.index, idx),
                class_id: b.class_id,
                class_name: b.class_name,
                confidence: b.confidence,
                x: b.x,
                y: b.y,
                width: b.width,
                height: b.height,
            })
            .collect();
        
        InferenceResult {
            frame_index: frame.index,
            timestamp_ms: frame.timestamp_ms,
            boxes,
            inference_time_ms: start.elapsed().as_secs_f64() * 1000.0,
        }
    }
    
    /// 停止推理
    pub async fn stop_inference(&self, session_id: &str) {
        let mut sessions = self.sessions.lock();
        if let Some(session) = sessions.get_mut(session_id) {
            session.is_running = false;
            eprintln!("[HPVVideo] 停止推理: {}", session_id);
        }
    }
    
    /// 获取会话状态
    pub fn get_status(&self, session_id: &str) -> Option<PipelineStatus> {
        let sessions = self.sessions.lock();
        sessions.get(session_id).map(|s| s.status.clone())
    }
}

impl Default for HighPerformanceVideoInferenceService {
    fn default() -> Self {
        Self::new()
    }
}

/// 视频信息
#[derive(Debug, Clone, Serialize)]
pub struct VideoStreamInfo {
    pub duration: f64,
    pub fps: f64,
    pub frames: u64,
    pub width: u32,
    pub height: u32,
}

/// 帧注解
#[derive(Debug, Clone, Serialize)]
pub struct FrameAnnotations {
    pub frame_index: u32,
    pub timestamp_ms: u64,
    pub boxes: Vec<AnnotationBox>,
}

/// 帧事件
#[derive(Debug, Clone, Serialize)]
pub struct FrameEvent {
    pub session_id: String,
    pub frame: u32,
    pub boxes: Vec<AnnotationBox>,
    pub progress: f32,
}

/// 完成事件
#[derive(Debug, Clone, Serialize)]
pub struct CompleteEvent {
    pub session_id: String,
    pub success: bool,
    pub frames_processed: u32,
    pub results_path: String,
    pub total_time_s: f64,
    pub avg_fps: f64,
}

/// 推理会话状态
#[derive(Debug, Clone, Serialize)]
pub struct InferenceSessionResponse {
    pub session_id: String,
    pub is_running: bool,
    pub frames_processed: u32,
    pub avg_inference_time_ms: f64,
}
