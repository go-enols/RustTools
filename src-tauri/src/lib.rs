use std::sync::{Arc, Mutex};

mod agent_commands;
mod commands;

// ============================================================================
// AppState
// ============================================================================

pub struct AppState {
    pub current_project: Mutex<Option<rusttools_app::models::ProjectConfig>>,
    pub trainer_service: Arc<rusttools_app::services::trainer::TrainerService>,
    pub yolo_engine: Mutex<Option<rusttools_app::services::yolo_onnx::YoloOnnxEngine>>,
    pub capture: Arc<Mutex<CaptureState>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_project: Mutex::new(None),
            trainer_service: Arc::new(rusttools_app::services::trainer::TrainerService::new()),
            yolo_engine: Mutex::new(None),
            capture: Arc::new(Mutex::new(CaptureState::default())),
        }
    }
}

#[derive(Default, serde::Serialize, Clone)]
pub struct OnnxDetection {
    pub class_id: usize,
    pub confidence: f32,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

#[derive(Default, serde::Serialize, Clone)]
pub struct CaptureState {
    pub running: bool,
    pub fps: f32,
    pub last_frame_base64: Option<String>,
    pub detections: Vec<OnnxDetection>,
}

// ============================================================================
// Application Entry
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new()
            .level(log::LevelFilter::Warn)
            .build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .manage(agent_commands::AgentState::default())
        .invoke_handler(tauri::generate_handler![
            // ── Project / Environment / Dialog ──
            commands::project::get_env_status,
            commands::project::refresh_env_status,
            commands::project::generate_env_report,
            commands::project::refresh_env_report,
            commands::project::install_python_env,
            commands::project::get_device_info,
            commands::project::pick_folder,
            commands::project::pick_file,
            commands::project::create_project,
            commands::project::open_project,
            commands::project::get_current_project,
            commands::project::update_project_classes,
            commands::project::scan_project,
            // ── YOLO / Training / Inference / Capture ──
            commands::yolo::start_training,
            commands::yolo::stop_training,
            commands::yolo::get_training_status,
            commands::yolo::list_training_logs,
            commands::yolo::list_training_results,
            commands::yolo::load_model,
            commands::yolo::run_inference_image,
            commands::yolo::unload_model,
            commands::yolo::auto_annotate_image,
            commands::yolo::list_models,
            commands::yolo::list_images,
            commands::yolo::read_yolo_labels,
            commands::yolo::save_yolo_labels,
            commands::yolo::get_image_dimensions,
            commands::yolo::extract_video_frame,
            commands::yolo::export_pt_to_onnx,
            commands::yolo::check_onnx_for_pt,
            commands::yolo::start_capture,
            commands::yolo::stop_capture,
            commands::yolo::get_capture_state,
            commands::yolo::get_capture_frame,
            // ── AI Agent 命令 ──
            agent_commands::agent_get_models,
            agent_commands::agent_add_model,
            agent_commands::agent_remove_model,
            agent_commands::agent_test_model,
            agent_commands::agent_list_agents,
            agent_commands::agent_create_agent,
            agent_commands::agent_update_agent,
            agent_commands::agent_delete_agent,
            agent_commands::agent_get_mcp_servers,
            agent_commands::agent_add_mcp_server,
            agent_commands::agent_remove_mcp_server,
            agent_commands::agent_send_message,
            agent_commands::agent_cancel_chat,
            agent_commands::agent_load_config,
            agent_commands::agent_save_config,
            agent_commands::agent_translate_skill,
            agent_commands::agent_create_session,
            agent_commands::agent_get_session_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_device_info_returns_valid_struct() {
        let result = commands::project::get_device_info();
        assert!(result.is_ok(), "get_device_info should succeed");
        let info = result.unwrap();
        assert!(!info.os.is_empty(), "OS should not be empty");
        assert!(!info.arch.is_empty(), "Architecture should not be empty");
        assert!(info.cpu.cores > 0, "CPU cores should be > 0");
        let _ = info.gpus.len();
    }

    #[test]
    fn test_generate_env_report_returns_all_fields() {
        let result = commands::project::generate_env_report();
        assert!(result.is_ok(), "generate_env_report should succeed");
        let report = result.unwrap();

        assert!(
            matches!(
                report.system.os,
                rusttools_app::services::env::OsType::Linux
                    | rusttools_app::services::env::OsType::Windows
                    | rusttools_app::services::env::OsType::MacOS
            ),
            "OS should be detected"
        );
        assert!(report.system.cpu_cores > 0, "CPU cores should be > 0");

        let _ = report.cuda.available;
        let _ = report.cuda.gpus.len();
        let _ = report.uv_installed;
        let _ = report.python_installed;
        let _ = report.venv_exists;
        let _ = report.torch_available;
        let _ = report.torch_cuda;
        let _ = report.ort_available;
        let _ = report.ort_cuda;
    }

    #[test]
    fn test_get_env_status_cuda_matches_report() {
        let status = commands::project::get_env_status().unwrap();
        let report = commands::project::generate_env_report().unwrap();

        // CUDA detection should be consistent between both functions
        assert_eq!(
            status.cuda_available, report.cuda.available,
            "CUDA availability should match between status and report"
        );
    }

    #[test]
    fn test_install_python_env_starts_installation() {
        let result = commands::project::install_python_env();
        assert!(result.is_ok(), "install_python_env should return Ok immediately (installation runs in background)");
    }

    #[test]
    fn test_scan_project_nonexistent_path() {
        let scan = commands::project::scan_project("/nonexistent/path".to_string()).unwrap();
        assert_eq!(scan.train_images, 0, "Non-existent path should have 0 train images");
        assert_eq!(scan.val_images, 0, "Non-existent path should have 0 val images");
        assert_eq!(scan.total_annotations, 0, "Non-existent path should have 0 annotations");
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(state.current_project.lock().unwrap().is_none());
        assert!(state.yolo_engine.lock().unwrap().is_none());
        assert!(!state.capture.lock().unwrap().running);
    }
}
