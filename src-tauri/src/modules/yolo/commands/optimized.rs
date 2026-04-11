//! 优化后的Tauri命令 - 提供高性能推理接口
//! 
//! 优化命令：
//! 1. optimized_desktop_capture_* - 优化的桌面捕获
//! 2. optimized_video_inference_* - 优化的视频推理

use crate::modules::yolo::models::training::{AnnotationBox, VideoInferenceConfig};
use crate::modules::yolo::services::desktop_capture_optimized::{
    OptimizedDesktopCaptureService, MonitorInfo as OptMonitorInfo, 
    DesktopCaptureFrame as OptDesktopCaptureFrame, DesktopCaptureStatus as OptDesktopCaptureStatus
};
use crate::modules::yolo::services::video_inference_optimized::{
    OptimizedVideoInferenceService, VideoInfo, FrameAnnotations
};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// 命令响应封装
#[derive(Debug, serde::Serialize)]
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

/// 桌面捕获配置
#[derive(Debug, serde::Deserialize)]
pub struct DesktopCaptureConfig {
    pub model_path: String,
    pub confidence: f32,
    pub device: String,
    pub monitor: u32,
    pub fps_limit: u32,
}

/// 启动优化的桌面捕获
#[tauri::command]
pub async fn optimized_desktop_capture_start(
    app: AppHandle,
    state: State<'_, Arc<OptimizedDesktopCaptureService>>,
    config: DesktopCaptureConfig,
) -> Result<CommandResponse<String>, String> {
    eprintln!("[OptimizedDesktop] Starting capture");
    
    // 验证模型路径
    if config.model_path.is_empty() {
        return Ok(CommandResponse::err("模型路径不能为空".to_string()));
    }
    
    if !std::path::Path::new(&config.model_path).exists() {
        return Ok(CommandResponse::err(format!("模型文件不存在: {}", config.model_path)));
    }
    
    // 创建会话ID
    let session_id = format!("opt_desktop_{}", 
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    
    // 启动捕获
    let state_clone = Arc::clone(&state);
    let session_id_clone = session_id.clone();
    
    tokio::spawn(async move {
        match state_clone.start_capture(
            session_id_clone,
            config.model_path,
            config.confidence,
            config.monitor,
            config.fps_limit,
            app
        ).await {
            Ok(()) => {
                eprintln!("[OptimizedDesktop] Capture started successfully");
            }
            Err(e) => {
                eprintln!("[OptimizedDesktop] Capture failed: {}", e);
            }
        }
    });
    
    Ok(CommandResponse::ok(session_id))
}

/// 停止优化的桌面捕获
#[tauri::command]
pub async fn optimized_desktop_capture_stop(
    state: State<'_, Arc<OptimizedDesktopCaptureService>>,
    session_id: String,
) -> Result<CommandResponse<()>, String> {
    match state.stop_capture(&session_id).await {
        Ok(()) => Ok(CommandResponse::ok(())),
        Err(e) => Ok(CommandResponse::err(e)),
    }
}

/// 获取监视器列表
#[tauri::command]
pub async fn optimized_get_monitors(
    state: State<'_, Arc<OptimizedDesktopCaptureService>>,
) -> Result<CommandResponse<Vec<OptMonitorInfo>>, String> {
    match state.get_monitors() {
        Ok(monitors) => Ok(CommandResponse::ok(monitors)),
        Err(e) => Ok(CommandResponse::err(e)),
    }
}

/// 获取优化桌面捕获状态
#[tauri::command]
pub async fn optimized_desktop_capture_status(
    state: State<'_, Arc<OptimizedDesktopCaptureService>>,
) -> Result<CommandResponse<OptDesktopCaptureStatus>, String> {
    let status = state.get_status().await;
    Ok(CommandResponse::ok(status))
}

/// 加载视频元数据
#[tauri::command]
pub async fn optimized_video_load(
    state: State<'_, Arc<OptimizedVideoInferenceService>>,
    video_path: String,
) -> Result<CommandResponse<VideoInfo>, String> {
    match state.probe_video(&video_path).await {
        Ok(info) => Ok(CommandResponse::ok(info)),
        Err(e) => Ok(CommandResponse::err(e)),
    }
}

/// 启动优化的视频推理
#[tauri::command]
pub async fn optimized_video_inference_start(
    app: AppHandle,
    state: State<'_, Arc<OptimizedVideoInferenceService>>,
    config: VideoInferenceConfig,
) -> Result<CommandResponse<String>, String> {
    let session_id = format!("opt_vid_{}", 
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    
    let session_id_clone = session_id.clone();
    let session_id_clone2 = session_id.clone();
    let state_arc = Arc::clone(&state);
    let session_id_return = session_id.clone();
    
    eprintln!("[OptimizedVideo] Starting inference session: {}", session_id);
    
    tokio::spawn(async move {
        let app_clone = app.clone();
        let app_for_callback = app.clone();
        
        let callback = move |frame_idx: u32, boxes: Vec<AnnotationBox>| {
            let event = serde_json::json!({
                "session_id": session_id_clone,
                "frame": frame_idx,
                "boxes": boxes,
            });
            let _ = app_for_callback.emit("optimized-video-inference-frame", event);
        };
        
        match state_arc.run_inference(&session_id, &config, callback).await {
            Ok(results) => {
                let _ = app_clone.emit("optimized-video-inference-complete", serde_json::json!({
                    "session_id": session_id_clone2,
                    "success": true,
                    "frames": results.len(),
                }));
            }
            Err(e) => {
                eprintln!("[OptimizedVideo] Inference error: {}", e);
                let _ = app_clone.emit("optimized-video-inference-complete", serde_json::json!({
                    "session_id": session_id_clone2,
                    "success": false,
                    "error": e,
                }));
            }
        }
    });
    
    Ok(CommandResponse::ok(session_id_return))
}

/// 停止优化的视频推理
#[tauri::command]
pub async fn optimized_video_inference_stop(
    state: State<'_, Arc<OptimizedVideoInferenceService>>,
    session_id: Option<String>,
) -> Result<CommandResponse<()>, String> {
    if let Some(sid) = session_id {
        state.stop_inference(&sid).await;
    }
    Ok(CommandResponse::ok(()))
}
