use crate::modules::yolo::services::python_env::{
    get_env_status, install_python_deps, InstallProgress, InstallResult, PythonEnvStatus,
};
use serde::{Deserialize, Serialize};

/// Standard command response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> CommandResponse<T> {
    pub fn ok(data: T) -> Self {
        CommandResponse {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: String) -> Self {
        CommandResponse {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// Check the current status of the Python environment
#[tauri::command]
pub fn python_env_status() -> CommandResponse<PythonEnvStatus> {
    CommandResponse::ok(get_env_status())
}

/// Check what Python packages are available (alias for status)
#[tauri::command]
pub fn python_env_check() -> CommandResponse<PythonEnvStatus> {
    CommandResponse::ok(get_env_status())
}

/// Install Python dependencies (torch, ultralytics) - runs asynchronously
/// Calls on_progress callback with InstallProgress during installation
/// Calls on_done callback with InstallResult when complete
#[tauri::command]
pub fn python_env_install(
    on_progress: Option<Box<dyn Fn(InstallProgress) + Send + Sync>>,
    on_done: Option<Box<dyn Fn(InstallResult) + Send + Sync>>,
) {
    install_python_deps(on_progress, on_done);
}
