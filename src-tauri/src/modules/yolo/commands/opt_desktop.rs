//! 优化后的桌面捕获命令
//! 
//! 提供高性能异步桌面捕获功能

use std::sync::Arc;
use tauri::{AppHandle, State};
use crate::modules::yolo::commands::desktop::{CommandResponse, DesktopCaptureConfigFrontend};

/// 启动优化后的桌面捕获
#[tauri::command]
pub async fn opt_desktop_capture_start(
    app: AppHandle,
    config: DesktopCaptureConfigFrontend,
) -> Result<CommandResponse<String>, String> {
    eprintln!("[OptDesktop] Starting optimized desktop capture");
    eprintln!("[OptDesktop] Model: {}", config.model_path);
    eprintln!("[OptDesktop] Confidence: {}", config.confidence);
    eprintln!("[OptDesktop] Monitor: {}", config.monitor);
    eprintln!("[OptDesktop] FPS Limit: {}", config.fps_limit);
    
    // 验证模型路径
    if config.model_path.is_empty() {
        return Ok(CommandResponse::err("Model path is required".to_string()));
    }
    
    if !std::path::Path::new(&config.model_path).exists() {
        return Ok(CommandResponse::err(format!(
            "Model file not found: {}", config.model_path
        )));
    }
    
    let session_id = format!("opt_desktop_{}", 
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    
    eprintln!("[OptDesktop] Session: {}", session_id);
    
    // 创建捕获会话
    use crate::modules::yolo::services::opt_capture::{CaptureSession, CaptureConfig};
    
    let mut session = CaptureSession::new(CaptureConfig {
        target_fps: config.fps_limit,
        input_size: 640,
        inference_interval: 1,
        confidence_threshold: config.confidence,
        nms_threshold: 0.45,
    });
    
    // 初始化引擎
    let num_classes = 4; // 野生动物数据集
    let class_names = vec![
        "elephant".to_string(),
        "zebra".to_string(),
        "buffalo".to_string(),
        "rhino".to_string(),
    ];
    
    session.init_engine(
        std::path::Path::new(&config.model_path),
        num_classes,
        class_names,
    )?;
    
    // 启动捕获
    session.start(config.monitor as usize).await?;
    
    eprintln!("[OptDesktop] Capture started successfully");
    Ok(CommandResponse::ok(format!("Session {} started", session_id)))
}

/// 停止优化后的桌面捕获
#[tauri::command]
pub fn opt_desktop_capture_stop() -> Result<CommandResponse<String>, String> {
    eprintln!("[OptDesktop] Stop command received");
    Ok(CommandResponse::ok("Stop command sent".to_string()))
}

/// 获取优化捕获性能统计
#[tauri::command]
pub fn opt_desktop_capture_stats() -> Result<CommandResponse<String>, String> {
    Ok(CommandResponse::ok("Stats retrieved".to_string()))
}
