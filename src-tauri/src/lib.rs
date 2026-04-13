pub mod core;
pub mod modules;

use core::commands::watcher::WatcherState;
use modules::yolo::services::{
    TrainerService, 
    VideoService, 
    VideoInferenceService,
    DesktopCaptureService,
};
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(WatcherState::default())
        .manage(Arc::new(TrainerService::new()))
        .manage(Arc::new(VideoService::new()))
        .manage(Arc::new(VideoInferenceService::new()))
        .manage(Arc::new(DesktopCaptureService::new()))
        .invoke_handler(tauri::generate_handler![
            // Settings commands
            core::commands::settings_load,
            core::commands::settings_save,
            // Project commands
            core::commands::project_recent_list,
            core::commands::project_save,
            // YOLO commands
            modules::yolo::commands::project_create,
            modules::yolo::commands::project_open,
            modules::yolo::commands::update_classes,
            modules::yolo::commands::load_annotation,
            modules::yolo::commands::save_annotation,
            // Training commands
            modules::yolo::commands::train::training_start,
            modules::yolo::commands::train::yolo_check_model,
            modules::yolo::commands::train::yolo_download_model,
            // File operations
            core::commands::read_text_file,
            core::commands::read_binary_file,
            core::commands::write_text_file,
            core::commands::delete_file,
            core::commands::rename_path,
            core::commands::create_directory,
            core::commands::delete_directory,
            core::commands::list_directory,
            core::commands::copy_file,
            core::commands::path_exists,
            // File watcher
            core::commands::start_watch,
            core::commands::stop_watch,
            // Video commands
            modules::yolo::commands::video::video_load,
            modules::yolo::commands::video::video_inference_start,
            modules::yolo::commands::video::video_inference_stop,
            modules::yolo::commands::video::video_capture_screenshot,
            modules::yolo::commands::video::video_extract_frames,
            modules::yolo::commands::video::video_inference_results,
            modules::yolo::commands::video::rust_video_inference_start,
            modules::yolo::commands::video::rust_video_inference_stop,
            // Device commands
            modules::yolo::commands::device::device_list,
            modules::yolo::commands::device::device_stats,
            modules::yolo::commands::device::device_set_default,
            // Desktop capture commands
            modules::yolo::commands::desktop::desktop_capture_start,
            modules::yolo::commands::desktop::desktop_capture_stop,
            modules::yolo::commands::desktop::get_monitors,
            modules::yolo::commands::desktop::get_desktop_capture_status,
            // Model conversion and compatibility check
            modules::yolo::commands::desktop::detect_model_format_cmd,
            modules::yolo::commands::desktop::get_model_info_cmd,
            modules::yolo::commands::desktop::check_model_compatibility,
            modules::yolo::commands::desktop::get_conversion_instructions_cmd,
            // Model conversion commands
            modules::yolo::commands::model_conversion::get_supported_formats,
            modules::yolo::commands::model_conversion::detect_format,
            modules::yolo::commands::model_conversion::get_model_details,
            modules::yolo::commands::model_conversion::check_compatibility,
            modules::yolo::commands::model_conversion::simplify_onnx_model,
            modules::yolo::commands::model_conversion::optimize_onnx_model,
            modules::yolo::commands::model_conversion::get_conversion_guide,
            modules::yolo::commands::model_conversion::get_conversion_script_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
