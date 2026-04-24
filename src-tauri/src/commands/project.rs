//! Project / Environment / Dialog 命令模块

use crate::AppState;
use tauri::State;

// ============================================================================
// Environment Commands
// ============================================================================

#[tauri::command]
pub fn get_env_status() -> Result<rusttools_app::services::python_env::PythonEnvStatus, String> {
    Ok(rusttools_app::services::python_env::get_env_status())
}

#[tauri::command]
pub fn refresh_env_status() -> Result<rusttools_app::services::python_env::PythonEnvStatus, String> {
    Ok(rusttools_app::services::python_env::refresh_env_status())
}

#[tauri::command]
pub fn generate_env_report() -> Result<rusttools_app::services::env::EnvReport, String> {
    Ok(rusttools_app::services::env::generate_env_report())
}

#[tauri::command]
pub fn refresh_env_report() -> Result<rusttools_app::services::env::EnvReport, String> {
    Ok(rusttools_app::services::env::refresh_env_report())
}

#[tauri::command]
pub fn install_python_env() -> Result<(), String> {
    rusttools_app::services::python_env::install_python_deps(None, None);
    Ok(())
}

#[derive(serde::Serialize)]
pub struct CpuInfo {
    model: String,
    cores: usize,
    threads: usize,
}

#[derive(serde::Serialize)]
pub struct MemoryInfo {
    total_mb: u64,
    used_mb: u64,
}

#[derive(serde::Serialize)]
pub struct GpuInfo {
    name: String,
    memory_mb: u64,
    cuda_available: bool,
}

#[derive(serde::Serialize)]
pub struct DeviceInfo {
    cpu: CpuInfo,
    memory: MemoryInfo,
    gpus: Vec<GpuInfo>,
    os: String,
    arch: String,
}

#[tauri::command]
pub fn get_device_info() -> Result<DeviceInfo, String> {
    use sysinfo::{System, RefreshKind, CpuRefreshKind, MemoryRefreshKind};

    let cuda = rusttools_app::services::env::detect_cuda();

    let gpus = cuda.gpus.into_iter().map(|gpu| GpuInfo {
        name: gpu.name,
        memory_mb: gpu.memory_mb,
        cuda_available: cuda.available,
    }).collect();

    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );
    sys.refresh_all();

    let cpu_model = sys.cpus().first()
        .map(|cpu| cpu.brand().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let physical_cores = sys.physical_core_count().unwrap_or(1);
    let logical_threads = sys.cpus().len();

    let total_mb = sys.total_memory();
    let used_mb = sys.used_memory();

    Ok(DeviceInfo {
        cpu: CpuInfo {
            model: cpu_model,
            cores: physical_cores,
            threads: logical_threads,
        },
        memory: MemoryInfo {
            total_mb,
            used_mb,
        },
        gpus,
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    })
}

// ============================================================================
// Dialog Commands
// ============================================================================

#[tauri::command]
pub async fn pick_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let file_path = app.dialog().file().blocking_pick_folder();
    Ok(file_path.map(|p| p.to_string()))
}

#[tauri::command]
pub async fn pick_file(app: tauri::AppHandle, extensions: Vec<String>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let mut builder = app.dialog().file();
    if !extensions.is_empty() {
        builder = builder.add_filter("files", &extensions.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    }
    let file_path = builder.blocking_pick_file();
    Ok(file_path.map(|p| p.to_string()))
}

// ============================================================================
// Project Commands
// ============================================================================

#[tauri::command]
pub fn create_project(
    config: rusttools_app::models::ProjectConfig,
    state: State<AppState>,
) -> Result<rusttools_app::models::ProjectResponse, String> {
    let response = rusttools_app::services::project::create_project(config.clone());
    if response.success {
        *state.current_project.lock().unwrap() = Some(config);
    }
    Ok(response)
}

#[tauri::command]
pub fn open_project(
    path: String,
    state: State<AppState>,
) -> Result<rusttools_app::models::ProjectResponse, String> {
    let response = rusttools_app::services::project::open_project(path);
    if let Some(ref config) = response.data {
        *state.current_project.lock().unwrap() = Some(config.clone());
    }
    Ok(response)
}

#[tauri::command]
pub fn get_current_project(
    state: State<AppState>,
) -> Result<Option<rusttools_app::models::ProjectConfig>, String> {
    Ok(state.current_project.lock().unwrap().clone())
}

#[tauri::command]
pub fn update_project_classes(
    path: String,
    classes: Vec<String>,
) -> Result<(), String> {
    rusttools_app::services::project::update_classes(path, classes)
        .map_err(|e| e)
}

#[tauri::command]
pub fn scan_project(path: String) -> Result<rusttools_app::models::ProjectScanResult, String> {
    Ok(rusttools_app::services::project::scan_project(&path))
}
