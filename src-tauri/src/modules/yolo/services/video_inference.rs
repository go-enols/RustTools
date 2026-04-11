//! 视频推理服务 - 纯 Rust 实现
//! 
//! 架构：
//! 1. 使用 ffmpeg 提取帧
//! 2. 使用 Rust 推理引擎进行检测
//! 3. 并行处理多个帧
//! 4. 通过 Tauri 事件流式返回结果

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::process::Command;
use tokio::fs;
use image::GenericImageView;

use super::inference_engine::InferenceEngine;
use crate::modules::yolo::models::training::{AnnotationBox, VideoInferenceConfig};

/// 帧注解
#[derive(Debug, Clone, serde::Serialize)]
pub struct FrameAnnotations {
    pub frame_index: u32,
    pub timestamp_ms: u64,
    pub boxes: Vec<AnnotationBox>,
}

/// 视频推理会话
#[derive(Debug)]
pub struct VideoSession {
    pub video_path: String,
    pub model_path: String,
    pub output_dir: PathBuf,
    pub is_running: Arc<Mutex<bool>>,
}

/// 视频推理服务
pub struct VideoInferenceService {
    sessions: Arc<Mutex<HashMap<String, VideoSession>>>,
}

impl VideoInferenceService {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// 获取视频信息
    pub async fn probe_video(&self, video_path: &str) -> Result<VideoInfo, String> {
        let path = PathBuf::from(video_path);
        if !path.exists() {
            return Err("视频文件不存在".to_string());
        }
        
        // 使用 ffprobe 获取视频信息
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
        
        // 提取视频流信息
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
        
        // 解析帧率
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
    
    /// 运行视频推理
    pub async fn run_inference(
        &self,
        session_id: &str,
        config: &VideoInferenceConfig,
        progress_callback: impl Fn(u32, Vec<AnnotationBox>) + Send + 'static,
    ) -> Result<Vec<FrameAnnotations>, String> {
        // 创建输出目录
        let output_dir = PathBuf::from(&config.output_dir);
        fs::create_dir_all(&output_dir)
            .await
            .map_err(|e| format!("创建输出目录失败: {}", e))?;
        
        // 创建会话
        let session = VideoSession {
            video_path: config.video_path.clone(),
            model_path: config.model_path.clone(),
            output_dir: output_dir.clone(),
            is_running: Arc::new(Mutex::new(true)),
        };
        
        {
            let mut sessions = self.sessions.lock().await;
            sessions.insert(session_id.to_string(), session);
        }
        
        // 提取帧目录
        let frames_dir = output_dir.join("frames");
        fs::create_dir_all(&frames_dir)
            .await
            .map_err(|e| format!("创建帧目录失败: {}", e))?;
        
        // 使用 ffmpeg 提取帧
        let frame_pattern = frames_dir.join("frame_%04d.jpg");
        
        eprintln!("[VideoInference] Extracting frames...");
        
        Command::new("ffmpeg")
            .args([
                "-y",
                "-i", &config.video_path,
                "-vf", &format!("fps={}", config.frame_interval),
                "-q:v", "2",
                frame_pattern.to_str().unwrap_or(""),
            ])
            .output()
            .await
            .map_err(|e| format!("ffmpeg 执行失败: {}", e))?;
        
        eprintln!("[VideoInference] Frames extracted, loading model...");
        
        // 加载推理引擎
        let engine = InferenceEngine::load(&config.model_path)?;
        
        // 获取帧列表
        let mut frames: Vec<PathBuf> = Vec::new();
        let mut entries = fs::read_dir(&frames_dir)
            .await
            .map_err(|e| format!("读取帧目录失败: {}", e))?;
        
        while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jpg") {
                frames.push(path);
            }
        }
        
        // 按文件名排序
        frames.sort();
        
        let total_frames = frames.len();
        eprintln!("[VideoInference] Processing {} frames...", total_frames);
        
        // 并行推理
        let mut results = Vec::with_capacity(total_frames);
        let batch_size = std::cmp::min(num_cpus(), 8); // 最多 8 个并行
        
        for (batch_idx, batch) in frames.chunks(batch_size).enumerate() {
            // 检查是否应该停止
            {
                let sessions = self.sessions.lock().await;
                if let Some(session) = sessions.get(session_id) {
                    if !*session.is_running.lock().await {
                        eprintln!("[VideoInference] Stopped by user");
                        break;
                    }
                }
            }
            
            // 处理当前批次
            let mut batch_results = Vec::new();
            
            for (i, frame_path) in batch.iter().enumerate() {
                let frame_idx = batch_idx * batch_size + i;
                
                // 加载图像
                match image::open(frame_path) {
                    Ok(img) => {
                        // 运行检测
                        let detections = engine.detect(&img, config.confidence);
                        
                        // 转换格式
                        let boxes: Vec<AnnotationBox> = detections.into_iter().enumerate()
                            .map(|(j, det)| AnnotationBox {
                                id: format!("{}_{}_{}", session_id, frame_idx, j),
                                class_id: det.class_id,
                                class_name: det.class_name,
                                confidence: det.confidence,
                                x: det.x,
                                y: det.y,
                                width: det.width,
                                height: det.height,
                            })
                            .collect();
                        
                        // 计算时间戳
                        let timestamp_ms = (frame_idx as f64 * 1000.0 / config.frame_interval as f64) as u64;
                        
                        batch_results.push(FrameAnnotations {
                            frame_index: frame_idx as u32,
                            timestamp_ms,
                            boxes: boxes.clone(),
                        });
                        
                        // 回调进度
                        progress_callback(frame_idx as u32, boxes);
                    }
                    Err(e) => {
                        eprintln!("[VideoInference] Failed to load frame {}: {}", frame_path.display(), e);
                    }
                }
            }
            
            results.extend(batch_results);
            
            // 打印进度
            let progress = (batch_idx + 1) * batch_size;
            if progress % 100 == 0 || progress >= total_frames {
                eprintln!("[VideoInference] Progress: {}/{} ({:.1}%)", 
                    std::cmp::min(progress, total_frames), 
                    total_frames,
                    (std::cmp::min(progress, total_frames) as f64 / total_frames as f64) * 100.0
                );
            }
        }
        
        // 更新会话状态
        {
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(session_id) {
                *session.is_running.lock().await = false;
            }
        }
        
        // 保存结果到 JSON 文件
        let results_path = output_dir.join("inference_results.json");
        let json = serde_json::to_string_pretty(&results)
            .map_err(|e| format!("结果序列化失败: {}", e))?;
        fs::write(&results_path, json).await.ok();
        
        eprintln!("[VideoInference] Complete! Results saved to {}", results_path.display());
        
        Ok(results)
    }
    
    /// 停止推理
    pub async fn stop_inference(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            *session.is_running.lock().await = false;
        }
    }
    
    /// 捕获单帧截图
    pub async fn capture_screenshot(&self, video_path: &str, timestamp_ms: u64, output_path: &str) -> Result<String, String> {
        let path = PathBuf::from(video_path);
        if !path.exists() {
            return Err("视频文件不存在".to_string());
        }
        
        let timestamp_sec = timestamp_ms as f64 / 1000.0;
        
        Command::new("ffmpeg")
            .args([
                "-y",
                "-ss", &format!("{:.3}", timestamp_sec),
                "-i", video_path,
                "-vframes", "1",
                "-q:v", "2",
                output_path,
            ])
            .output()
            .await
            .map_err(|e| format!("ffmpeg 截图失败: {}", e))?;
        
        if !PathBuf::from(output_path).exists() {
            return Err("截图文件未创建".to_string());
        }
        
        Ok(output_path.to_string())
    }
    
    /// 提取帧
    pub async fn extract_frames(&self, video_path: &str, interval_ms: u32, output_dir: &str) -> Result<Vec<String>, String> {
        let path = PathBuf::from(video_path);
        if !path.exists() {
            return Err("视频文件不存在".to_string());
        }
        
        fs::create_dir_all(output_dir)
            .await
            .map_err(|e| format!("创建输出目录失败: {}", e))?;
        
        // 获取视频信息
        let info = self.probe_video(video_path).await?;
        let interval_sec = interval_ms as f64 / 1000.0;
        
        // 使用 ffmpeg 提取帧
        let output_pattern = PathBuf::from(output_dir).join("frame_%04d.jpg");
        
        Command::new("ffmpeg")
            .args([
                "-y",
                "-i", video_path,
                "-vf", &format!("fps={:.3}", 1.0 / interval_sec),
                "-q:v", "2",
                output_pattern.to_str().unwrap_or(""),
            ])
            .output()
            .await
            .map_err(|e| format!("ffmpeg 帧提取失败: {}", e))?;
        
        // 列出提取的帧
        let mut frames = Vec::new();
        let mut entries = fs::read_dir(output_dir)
            .await
            .map_err(|e| format!("读取输出目录失败: {}", e))?;
        
        while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();
            if path.extension().map(|e| e == "jpg" || e == "jpeg").unwrap_or(false) {
                frames.push(path.to_string_lossy().to_string());
            }
        }
        
        frames.sort();
        
        Ok(frames)
    }
}

impl Default for VideoInferenceService {
    fn default() -> Self {
        Self::new()
    }
}

/// 视频信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct VideoInfo {
    pub duration: f64,
    pub fps: f64,
    pub frames: u64,
    pub width: u32,
    pub height: u32,
}

/// 获取 CPU 核心数
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
