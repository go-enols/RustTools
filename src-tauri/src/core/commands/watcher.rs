use notify::{
    Config, Event, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

// Shared state for watchers
pub struct WatcherState {
    watchers: Mutex<HashMap<String, RecommendedWatcher>>,
}

impl Default for WatcherState {
    fn default() -> Self {
        Self {
            watchers: Mutex::new(HashMap::new()),
        }
    }
}

// File change event sent to frontend
#[derive(Clone, serde::Serialize)]
pub struct FileChangeEvent {
    pub path: String,
    pub kind: String,
    pub parent: String,
}

/// Start watching a directory for file changes
#[tauri::command]
pub async fn start_watch(
    app: AppHandle,
    state: State<'_, WatcherState>,
    path: String,
) -> Result<(), String> {
    println!("[start_watch] Starting watch on: {:?}", path);

    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("Path does not exist: {:?}", path));
    }

    let app_handle = app.clone();

    let watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| {
            if let Ok(event) = result {
                let kind = match event.kind {
                    notify::EventKind::Create(_) => Some("create"),
                    notify::EventKind::Modify(_) => Some("modify"),
                    notify::EventKind::Remove(_) => Some("remove"),
                    _ => None,
                };

                if let Some(kind) = kind {
                    for event_path in event.paths {
                        // Get parent directory for incremental updates
                        let parent = event_path.parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default();

                        let change = FileChangeEvent {
                            path: event_path.to_string_lossy().to_string(),
                            kind: kind.to_string(),
                            parent,
                        };

                        println!("[watcher] File {}: {:?}", kind, change.path);

                        if let Err(e) = app_handle.emit("file-change", &change) {
                            println!("[watcher] Failed to emit event: {:?}", e);
                        }
                    }
                }
            }
        },
        Config::default(),
    )
    .map_err(|e| format!("Failed to create watcher: {}", e))?;

    let mut watchers = state.watchers.lock().map_err(|e| e.to_string())?;

    // Stop existing watcher for this path if any
    watchers.remove(&path);

    // Add and start watching
    let mut new_watcher = watcher;
    new_watcher
        .watch(&path_buf, RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch path: {}", e))?;

    watchers.insert(path, new_watcher);

    println!("[start_watch] Watch started successfully");
    Ok(())
}

/// Stop watching a directory
#[tauri::command]
pub async fn stop_watch(state: State<'_, WatcherState>, path: String) -> Result<(), String> {
    println!("[stop_watch] Stopping watch on: {:?}", path);

    let mut watchers = state.watchers.lock().map_err(|e| e.to_string())?;
    watchers.remove(&path);

    println!("[stop_watch] Watch stopped successfully");
    Ok(())
}
