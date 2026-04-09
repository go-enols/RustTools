use crate::modules::yolo::services::device::{self, DeviceInfo, DeviceStats};
use serde::{Deserialize, Serialize};

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

/// List available compute devices (GPU/CPU)
#[tauri::command]
pub fn device_list() -> CommandResponse<Vec<DeviceInfo>> {
    let devices = device::list_devices();
    CommandResponse::ok(devices)
}

/// Get device utilization statistics
#[tauri::command]
pub fn device_stats(device_id: i32) -> CommandResponse<DeviceStats> {
    // Only GPU devices have detailed stats
    if device_id < 0 {
        return CommandResponse::err("Invalid device ID".to_string());
    }

    match device::get_gpu_stats(device_id) {
        Some(stats) => CommandResponse::ok(stats),
        None => CommandResponse::ok(DeviceStats {
            gpu_util: 0.0,
            memory_util: 0.0,
            temperature: 0.0,
        }),
    }
}

/// Set the default training device (persisted in settings)
#[tauri::command]
pub fn device_set_default(device_id: i32) -> CommandResponse<()> {
    // For now, just acknowledge - actual persistence is handled by settings system
    CommandResponse::ok(())
}
