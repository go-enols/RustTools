//! Desktop Capture Commands - Rust Implementation
//! 
//! This module provides Tauri commands for desktop capture functionality,
//! implemented in pure Rust for better performance and reliability.

use std::sync::Arc;
use tauri::{AppHandle, State};
use crate::modules::yolo::services::desktop_capture::{
    DesktopCaptureService, MonitorInfo
};

/// Response wrapper for commands
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

/// Desktop capture configuration from frontend
#[derive(Debug, serde::Deserialize)]
pub struct DesktopCaptureConfigFrontend {
    pub model_path: String,
    pub confidence: f32,
    pub device: String,
    pub monitor: u32,
    pub fps_limit: u32,
}

/// Start desktop capture inference
#[tauri::command]
pub async fn desktop_capture_start(
    app: AppHandle,
    state: State<'_, Arc<DesktopCaptureService>>,
    config: DesktopCaptureConfigFrontend,
) -> Result<CommandResponse<String>, String> {
    eprintln!("[Desktop] Received start command");
    eprintln!("[Desktop] Model: {}", config.model_path);
    eprintln!("[Desktop] Confidence: {}", config.confidence);
    eprintln!("[Desktop] Device: {}", config.device);
    eprintln!("[Desktop] Monitor: {}", config.monitor);
    eprintln!("[Desktop] FPS Limit: {}", config.fps_limit);
    
    // Validate model path
    if config.model_path.is_empty() {
        return Ok(CommandResponse::err("Model path is required".to_string()));
    }
    
    // Check if model file exists
    if !std::path::Path::new(&config.model_path).exists() {
        return Ok(CommandResponse::err(format!(
            "Model file not found: {}. Please ensure the model file exists and the path is correct.",
            config.model_path
        )));
    }
    
    // Create session ID
    let session_id = format!("desktop_{}", 
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    
    eprintln!("[Desktop] Created session: {}", session_id);
    
    match state
        .start_capture(
            session_id.clone(),
            config.model_path.clone(),
            config.confidence,
            config.monitor,
            config.fps_limit,
            app,
        )
        .await
    {
        Ok(()) => {
            eprintln!("[Desktop] Capture started successfully");
        }
        Err(e) => {
            eprintln!("[Desktop] Capture failed: {}", e);
            return Ok(CommandResponse::err(e));
        }
    }
    
    Ok(CommandResponse::ok(session_id))
}

/// Stop desktop capture inference
#[tauri::command]
pub async fn desktop_capture_stop(
    state: State<'_, Arc<DesktopCaptureService>>,
    session_id: String,
) -> Result<CommandResponse<()>, String> {
    eprintln!("[Desktop] Received stop command for session: {}", session_id);
    
    match state.stop_capture(&session_id).await {
        Ok(()) => {
            eprintln!("[Desktop] Capture stopped successfully");
            Ok(CommandResponse::ok(()))
        }
        Err(e) => {
            eprintln!("[Desktop] Failed to stop capture: {}", e);
            Ok(CommandResponse::err(e))
        }
    }
}

/// Get list of available monitors
#[tauri::command]
pub async fn get_monitors(
    state: State<'_, Arc<DesktopCaptureService>>,
) -> Result<CommandResponse<Vec<MonitorInfo>>, String> {
    eprintln!("[Desktop] Getting monitor list");
    
    match state.get_monitors() {
        Ok(monitors) => {
            eprintln!("[Desktop] Found {} monitors", monitors.len());
            for (i, monitor) in monitors.iter().enumerate() {
                eprintln!("[Desktop] Monitor {}: {} ({}x{})", 
                    i, monitor.name, monitor.width, monitor.height);
            }
            Ok(CommandResponse::ok(monitors))
        }
        Err(e) => {
            eprintln!("[Desktop] Failed to get monitors: {}", e);
            Ok(CommandResponse::err(e))
        }
    }
}

/// Get desktop capture service state
#[tauri::command]
pub async fn get_desktop_capture_status(
    state: State<'_, Arc<DesktopCaptureService>>,
) -> Result<CommandResponse<DesktopCaptureStatusResponse>, String> {
    let status = state.get_status().await;
    Ok(CommandResponse::ok(DesktopCaptureStatusResponse {
        active_sessions: status.active_sessions,
        total_sessions: status.total_sessions,
    }))
}

#[derive(Debug, serde::Serialize)]
pub struct DesktopCaptureStatusResponse {
    pub active_sessions: Vec<String>,
    pub total_sessions: usize,
}

/// Detect model format
#[tauri::command]
pub fn detect_model_format_cmd(path: String) -> Result<CommandResponse<String>, String> {
    use crate::modules::yolo::services::model_converter::detect_model_format;
    
    let format = detect_model_format(&path);
    let format_str = match format {
        crate::modules::yolo::services::model_converter::ModelFormat::PyTorch => "PyTorch (.pt)",
        crate::modules::yolo::services::model_converter::ModelFormat::ONNX => "ONNX (.onnx)",
        crate::modules::yolo::services::model_converter::ModelFormat::Safetensors => "Safetensors (.safetensors)",
        crate::modules::yolo::services::model_converter::ModelFormat::CandleModel => "Candle Model (.bin)",
        crate::modules::yolo::services::model_converter::ModelFormat::Unknown => "Unknown",
    };
    
    Ok(CommandResponse::ok(format_str.to_string()))
}

/// Get detailed model information
#[tauri::command]
pub fn get_model_info_cmd(path: String) -> Result<CommandResponse<String>, String> {
    use crate::modules::yolo::services::model_converter::get_model_info;
    
    let info = get_model_info(&path);
    Ok(CommandResponse::ok(info.details))
}

/// Check if model is compatible with the system
#[tauri::command]
pub fn check_model_compatibility(path: String) -> Result<CommandResponse<ModelCompatibilityResponse>, String> {
    use crate::modules::yolo::services::model_converter::{is_model_compatible, ModelFormat};
    
    let result = is_model_compatible(&path);
    
    // 转换格式为字符串
    let format_str = match result.format {
        ModelFormat::PyTorch => "PyTorch (.pt)".to_string(),
        ModelFormat::ONNX => "ONNX (.onnx)".to_string(),
        ModelFormat::Safetensors => "Safetensors".to_string(),
        ModelFormat::CandleModel => "Candle Model".to_string(),
        ModelFormat::Unknown => "Unknown".to_string(),
    };
    
    Ok(CommandResponse::ok(ModelCompatibilityResponse {
        is_compatible: result.is_compatible,
        message: result.message,
        conversion_hint: result.conversion_hint,
        format: format_str,
    }))
}

#[derive(Debug, serde::Serialize)]
pub struct ModelCompatibilityResponse {
    pub is_compatible: bool,
    pub format: String,
    pub message: String,
    pub conversion_hint: Option<String>,
}

/// Get model conversion instructions
#[tauri::command]
pub fn get_conversion_instructions_cmd(path: String) -> Result<CommandResponse<String>, String> {
    use crate::modules::yolo::services::model_converter::{detect_model_format, format_conversion_instructions};
    
    let format = detect_model_format(&path);
    let instructions = format_conversion_instructions(&format);
    
    Ok(CommandResponse::ok(instructions))
}

