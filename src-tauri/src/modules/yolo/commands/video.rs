use crate::modules::yolo::models::training::{AnnotationBox, VideoInferenceConfig};
use crate::modules::yolo::services::video::VideoInfo;
use crate::modules::yolo::services::{VideoService, VideoInferenceService};
use crate::modules::yolo::services::video_inference::VideoInfo as RustVideoInfo;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> CommandResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    pub fn err(msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg),
        }
    }
}

/// Load video and return metadata (uses existing VideoService)
#[tauri::command]
pub async fn video_load(
    state: State<'_, Arc<VideoService>>,
    video_path: String,
) -> Result<CommandResponse<VideoInfo>, String> {
    match state.probe_video(&video_path).await {
        Ok(info) => Ok(CommandResponse::ok(info)),
        Err(e) => Ok(CommandResponse::err(e)),
    }
}

/// Start video inference using Python (existing implementation)
#[tauri::command]
pub async fn video_inference_start(
    app: AppHandle,
    state: State<'_, Arc<VideoService>>,
    config: VideoInferenceConfig,
) -> Result<CommandResponse<String>, String> {
    let session_id = format!("vid_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis());

    let session_id_clone = session_id.clone();
    let session_id_clone2 = session_id.clone();
    let state_arc = Arc::clone(&state);
    let session_id_return = session_id.clone();

    // Spawn inference in background
    tokio::spawn(async move {
        let app_clone = app.clone();
        let app_clone_for_callback = app.clone();
        let callback = move |frame_idx: u32, boxes: Vec<AnnotationBox>| {
            let event = serde_json::json!({
                "session_id": session_id_clone,
                "frame": frame_idx,
                "boxes": boxes,
            });
            let _ = app_clone_for_callback.emit("video-inference-frame", event);
        };

        match state_arc.run_inference(&session_id, &config, callback).await {
            Ok(results) => {
                let _ = app_clone.emit("video-inference-complete", serde_json::json!({
                    "session_id": session_id_clone2,
                    "success": true,
                    "frames": results.len(),
                }));
            }
            Err(e) => {
                let _ = app_clone.emit("video-inference-complete", serde_json::json!({
                    "session_id": session_id_clone2,
                    "success": false,
                    "error": e,
                }));
            }
        }
    });

    Ok(CommandResponse::ok(session_id_return))
}

/// Start video inference using pure Rust (NEW - optimized)
#[tauri::command]
pub async fn rust_video_inference_start(
    app: AppHandle,
    state: State<'_, Arc<VideoInferenceService>>,
    config: VideoInferenceConfig,
) -> Result<CommandResponse<String>, String> {
    let session_id = format!("rust_vid_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis());

    let session_id_clone = session_id.clone();
    let session_id_clone2 = session_id.clone();
    let state_arc = Arc::clone(&state);
    let session_id_return = session_id.clone();

    eprintln!("[RustVideoInference] Starting session: {}", session_id);

    // Spawn inference in background
    tokio::spawn(async move {
        let app_clone = app.clone();
        let app_clone_for_callback = app.clone();
        let callback = move |frame_idx: u32, boxes: Vec<AnnotationBox>| {
            let event = serde_json::json!({
                "session_id": session_id_clone,
                "frame": frame_idx,
                "boxes": boxes,
            });
            let _ = app_clone_for_callback.emit("rust-video-inference-frame", event);
        };

        match state_arc.run_inference(&session_id, &config, callback).await {
            Ok(results) => {
                let _ = app_clone.emit("rust-video-inference-complete", serde_json::json!({
                    "session_id": session_id_clone2,
                    "success": true,
                    "frames": results.len(),
                    "results_path": format!("{}/inference_results.json", config.output_dir),
                }));
            }
            Err(e) => {
                eprintln!("[RustVideoInference] Error: {}", e);
                let _ = app_clone.emit("rust-video-inference-complete", serde_json::json!({
                    "session_id": session_id_clone2,
                    "success": false,
                    "error": e,
                }));
            }
        }
    });

    Ok(CommandResponse::ok(session_id_return))
}

/// Stop Rust video inference
#[tauri::command]
pub async fn rust_video_inference_stop(
    state: State<'_, Arc<VideoInferenceService>>,
    session_id: Option<String>,
) -> Result<CommandResponse<()>, String> {
    if let Some(sid) = session_id {
        state.stop_inference(&sid).await;
    }
    Ok(CommandResponse::ok(()))
}

/// Stop video inference (Python)
#[tauri::command]
pub async fn video_inference_stop(
    state: State<'_, Arc<VideoService>>,
    session_id: Option<String>,
) -> Result<CommandResponse<()>, String> {
    if let Some(sid) = session_id {
        state.stop_inference(&sid).await;
    }
    Ok(CommandResponse::ok(()))
}

/// Capture screenshot from video at given timestamp
#[tauri::command]
pub async fn video_capture_screenshot(
    state: State<'_, Arc<VideoService>>,
    video_path: String,
    timestamp_ms: u64,
) -> Result<CommandResponse<String>, String> {
    let output_path = format!("/tmp/screenshot_{}.jpg", timestamp_ms);
    match state.capture_screenshot(&video_path, timestamp_ms, &output_path).await {
        Ok(path) => Ok(CommandResponse::ok(path)),
        Err(e) => Ok(CommandResponse::err(e)),
    }
}

/// Extract frames from video at given interval
#[tauri::command]
pub async fn video_extract_frames(
    state: State<'_, Arc<VideoService>>,
    video_path: String,
    interval_ms: u32,
) -> Result<CommandResponse<Vec<String>>, String> {
    let output_dir = format!("/tmp/frames_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis());
    match state.extract_frames(&video_path, interval_ms, &output_dir).await {
        Ok(frames) => Ok(CommandResponse::ok(frames)),
        Err(e) => Ok(CommandResponse::err(e)),
    }
}

/// Get inference results (placeholder - results are streamed)
#[tauri::command]
pub async fn video_inference_results(
    _state: State<'_, Arc<VideoService>>,
    _inference_id: String,
) -> Result<CommandResponse<serde_json::Value>, String> {
    // Results are delivered via events, this is a placeholder for future file-based retrieval
    Ok(CommandResponse::ok(serde_json::json!({
        "message": "Results delivered via events"
    })))
}
