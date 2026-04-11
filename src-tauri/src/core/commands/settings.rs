use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static SETTINGS: Lazy<Mutex<Option<HashMap<String, serde_json::Value>>>> = Lazy::new(|| Mutex::new(None));

fn get_settings_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("rust-tools");
    path.push("settings.json");
    path
}

fn get_settings_dir() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("rust-tools");
    path
}

/// Load application settings from disk
#[tauri::command]
pub fn settings_load() -> Result<HashMap<String, serde_json::Value>, String> {
    let path = get_settings_path();
    
    if !path.exists() {
        // Return empty settings if file doesn't exist
        let mut empty_settings = HashMap::new();
        // Add default settings
        empty_settings.insert("theme".to_string(), serde_json::json!("light"));
        empty_settings.insert("language".to_string(), serde_json::json!("zh-CN"));
        empty_settings.insert("autoCheckEnv".to_string(), serde_json::json!(true));
        return Ok(empty_settings);
    }
    
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read settings: {}", e))?;
    
    let settings: HashMap<String, serde_json::Value> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings: {}", e))?;
    
    Ok(settings)
}

/// Save application settings to disk
#[tauri::command]
pub fn settings_save(settings: HashMap<String, serde_json::Value>) -> Result<(), String> {
    let path = get_settings_path();
    let dir = get_settings_dir();
    
    // Create directory if it doesn't exist
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create settings directory: {}", e))?;
    }
    
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write settings: {}", e))?;
    
    // Update cached settings
    if let Ok(mut cache) = SETTINGS.lock() {
        *cache = Some(settings);
    }
    
    Ok(())
}
