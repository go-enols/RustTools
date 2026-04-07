pub mod core;
pub mod modules;

use core::commands::watcher::WatcherState;
use modules::yolo::services::TrainerService;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(WatcherState::default())
        .manage(Arc::new(TrainerService::new()))
        .invoke_handler(tauri::generate_handler![
            modules::yolo::commands::project_create,
            modules::yolo::commands::project_open,
            modules::yolo::commands::update_classes,
            modules::yolo::commands::load_annotation,
            modules::yolo::commands::save_annotation,
            // Training commands
            modules::yolo::commands::train::training_start,
            modules::yolo::commands::train::training_stop,
            modules::yolo::commands::train::training_pause,
            modules::yolo::commands::train::training_resume,
            modules::yolo::commands::train::yolo_check_model,
            modules::yolo::commands::train::yolo_download_model,
            // Env commands
            modules::yolo::commands::env::check_python_env,
            modules::yolo::commands::env::install_python_deps,
            modules::yolo::commands::env::get_install_instructions,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
