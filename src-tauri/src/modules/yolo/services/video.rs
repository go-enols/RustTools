use crate::modules::yolo::models::training::{AnnotationBox, VideoInferenceConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

/// Global video inference state
pub struct VideoService {
    pub sessions: Mutex<HashMap<String, VideoSession>>,
}

#[derive(Debug, Clone)]
pub struct VideoSession {
    pub video_path: String,
    pub model_path: String,
    pub confidence: f32,
    pub output_dir: PathBuf,
    pub is_running: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoInfo {
    pub duration: f64,
    pub fps: f64,
    pub frames: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotResult {
    pub screenshot_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrameAnnotations {
    pub frame_index: u32,
    pub timestamp_ms: u64,
    pub boxes: Vec<AnnotationBox>,
}

impl Default for VideoService {
    fn default() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

impl VideoService {
    pub fn new() -> Self {
        Self::default()
    }

    /// Probe video file to get metadata using ffprobe
    pub async fn probe_video(&self, video_path: &str) -> Result<VideoInfo, String> {
        let path = PathBuf::from(video_path);
        if !path.exists() {
            return Err("Video file not found".to_string());
        }

        // Use ffprobe to get video metadata
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
            .map_err(|e| format!("ffprobe failed: {}", e))?;

        if !output.status.success() {
            return Err(format!("ffprobe failed with status: {}", output.status));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed to parse ffprobe output: {}", e))?;

        // Extract video stream info
        let video_stream = json["streams"]
            .as_array()
            .and_then(|streams| streams.iter().find(|s| s["codec_type"] == "video"))
            .ok_or("No video stream found")?;

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

    /// Run YOLO inference on a video file using Python sidecar approach
    pub async fn run_inference(
        &self,
        session_id: &str,
        config: &VideoInferenceConfig,
        progress_callback: impl Fn(u32, Vec<AnnotationBox>) + Send + 'static,
    ) -> Result<Vec<FrameAnnotations>, String> {
        let mut sessions = self.sessions.lock().await;

        // Create output directory
        let output_dir = PathBuf::from(&config.output_dir);
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;

        // Store session
        let session = VideoSession {
            video_path: config.video_path.clone(),
            model_path: config.model_path.clone(),
            confidence: config.confidence,
            output_dir: output_dir.clone(),
            is_running: true,
        };
        sessions.insert(session_id.to_string(), session);

        drop(sessions);

        // Build Python inference script arguments
        let infer_script = Self::get_infer_script_path()?;
        let output_json = output_dir.join("inference_results.json");

        let python_args = vec![
            config.video_path.clone(),
            config.model_path.clone(),
            "--conf".to_string(),
            config.confidence.to_string(),
            "--iou".to_string(),
            config.iou_threshold.to_string(),
            "--device".to_string(),
            config.device.clone(),
            "--output-json".to_string(),
            output_json.to_str().unwrap_or("").to_string(),
            "--frame-interval".to_string(),
            config.frame_interval.to_string(),
        ];

        // Run Python inference
        let python_cmd = if cfg!(windows) { "python" } else { "python3" };
        let mut child = Command::new(python_cmd)
            .arg(&infer_script)
            .args(&python_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to start inference: {}", e))?;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let mut reader = BufReader::new(stdout).lines();

        let mut all_results: Vec<FrameAnnotations> = Vec::new();
        let mut _current_frame: Option<FrameAnnotations> = None;

        // Parse stdout for progress updates
        loop {
            let line = match timeout(Duration::from_secs(60), reader.next_line()).await {
                Ok(Ok(l)) => l,
                Ok(Err(_)) => break,
                Err(_) => {
                    // Timeout - check if process is still running
                    if let Ok(exit_status) = child.try_wait() {
                        if exit_status.is_some() { break; }
                    }
                    continue;
                }
            };

            if let Some(line) = line {
                // Parse JSON progress lines from Python script
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
                    let event_type = event["type"].as_str().unwrap_or("");
                    match event_type {
                        "progress" => {
                            let frame_idx = event["frame"].as_u64().unwrap_or(0) as u32;
                            let timestamp_ms = event["timestamp_ms"].as_u64().unwrap_or(0);
                            let boxes: Vec<AnnotationBox> = event["boxes"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter().filter_map(|b| {
                                        let x1 = b["x1"].as_f64().unwrap_or(0.0) as f32;
                                        let y1 = b["y1"].as_f64().unwrap_or(0.0) as f32;
                                        let x2 = b["x2"].as_f64().unwrap_or(0.0) as f32;
                                        let y2 = b["y2"].as_f64().unwrap_or(0.0) as f32;
                                        Some(AnnotationBox {
                                            id: format!("box_{}_{}", frame_idx, b["class_id"].as_u64().unwrap_or(0)),
                                            class_id: b["class_id"].as_u64().unwrap_or(0) as usize,
                                            class_name: b["class_name"].as_str().unwrap_or("").to_string(),
                                            confidence: b["confidence"].as_f64().unwrap_or(0.0) as f32,
                                            x: x1,
                                            y: y1,
                                            width: x2 - x1,
                                            height: y2 - y1,
                                        })
                                    }).collect()
                                })
                                .unwrap_or_default();

                            _current_frame = Some(FrameAnnotations {
                                frame_index: frame_idx,
                                timestamp_ms,
                                boxes: boxes.clone(),
                            });

                            progress_callback(frame_idx, boxes);
                        }
                        "complete" => {
                            // Inference finished
                            break;
                        }
                        _ => {}
                    }
                }
            }

            // Check if process ended
            if let Ok(Some(_)) = child.try_wait() {
                break;
            }
        }

        // Wait for process to finish
        let status = child.wait().await
            .map_err(|e| format!("Process wait failed: {}", e))?;

        // Try to load results from JSON file if produced
        if output_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&output_json) {
                if let Ok(results) = serde_json::from_str::<Vec<FrameAnnotations>>(&content) {
                    all_results = results;
                }
            }
        }

        if !status.success() && all_results.is_empty() {
            return Err(format!("Inference exited with non-zero status: {:?}", status));
        }

        // Update session
        let mut sessions = self.sessions.lock().await;
        if let Some(sess) = sessions.get_mut(session_id) {
            sess.is_running = false;
        }

        Ok(all_results)
    }

    /// Capture a single frame screenshot using ffmpeg
    pub async fn capture_screenshot(&self, video_path: &str, timestamp_ms: u64, output_path: &str) -> Result<String, String> {
        let path = PathBuf::from(video_path);
        if !path.exists() {
            return Err("Video file not found".to_string());
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
            .map_err(|e| format!("ffmpeg screenshot failed: {}", e))?;

        if !PathBuf::from(output_path).exists() {
            return Err("Screenshot file was not created".to_string());
        }

        Ok(output_path.to_string())
    }

    /// Extract frames at regular intervals using ffmpeg
    pub async fn extract_frames(&self, video_path: &str, interval_ms: u32, output_dir: &str) -> Result<Vec<String>, String> {
        let path = PathBuf::from(video_path);
        if !path.exists() {
            return Err("Video file not found".to_string());
        }

        std::fs::create_dir_all(output_dir)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;

        // Get video duration
        let info = self.probe_video(video_path).await?;
        let interval_sec = interval_ms as f64 / 1000.0;
        let _num_frames = (info.duration / interval_sec).ceil() as u32;

        // Use ffmpeg to extract frames
        let output_pattern = PathBuf::from(output_dir).join("frame_%04d.jpg");
        let output_str = output_pattern.to_str().unwrap_or("");

        Command::new("ffmpeg")
            .args([
                "-y",
                "-i", video_path,
                "-vf", &format!("fps={:.3}", 1.0 / interval_sec),
                "-q:v", "2",
                output_str,
            ])
            .output()
            .await
            .map_err(|e| format!("ffmpeg frame extraction failed: {}", e))?;

        // List extracted frames
        let mut frames: Vec<String> = Vec::new();
        let entries = std::fs::read_dir(output_dir)
            .map_err(|e| format!("Failed to read output directory: {}", e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "jpg" || e == "jpeg").unwrap_or(false) {
                frames.push(path.to_string_lossy().to_string());
            }
        }
        frames.sort();

        Ok(frames)
    }

    /// Get the Python inference script path
    fn get_infer_script_path() -> Result<String, String> {
        let search_paths = [
            PathBuf::from("src-tauri/scripts/video_infer.py"),
            PathBuf::from("scripts/video_infer.py"),
        ];

        search_paths
            .iter()
            .find(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .ok_or_else(|| "video_infer.py not found".to_string())
    }

    /// Stop running inference
    pub async fn stop_inference(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.is_running = false;
        }
    }
}
