use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static RECENT_PROJECTS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));
const MAX_RECENT_PROJECTS: usize = 10;

fn get_recent_projects_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("rust-tools");
    path.push("recent_projects.json");
    path
}

fn get_projects_dir() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("rust-tools");
    path.push("projects");
    path
}

/// Get list of recent projects
#[tauri::command]
pub fn project_recent_list() -> Result<Vec<String>, String> {
    let path = get_recent_projects_path();
    
    if !path.exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read recent projects: {}", e))?;
    
    let projects: Vec<String> = serde_json::from_str(&content)
        .unwrap_or_else(|_| Vec::new());
    
    Ok(projects)
}

/// Add a project to the recent list
fn add_to_recent_list(project_path: &str) -> Result<(), String> {
    let path = get_recent_projects_path();
    let dir = path.parent().unwrap_or(&PathBuf::from(".")).to_path_buf();
    
    // Create directory if it doesn't exist
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create projects directory: {}", e))?;
    }
    
    let mut projects = project_recent_list().unwrap_or_else(|_| Vec::new());
    
    // Remove if already exists (to move it to front)
    projects.retain(|p| p != project_path);
    
    // Add to front
    projects.insert(0, project_path.to_string());
    
    // Keep only MAX_RECENT_PROJECTS
    projects.truncate(MAX_RECENT_PROJECTS);
    
    let content = serde_json::to_string_pretty(&projects)
        .map_err(|e| format!("Failed to serialize recent projects: {}", e))?;
    
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write recent projects: {}", e))?;
    
    Ok(())
}

/// Save current project state
#[tauri::command]
pub fn project_save(
    project_path: String,
    config: serde_json::Value,
) -> Result<(), String> {
    let path = PathBuf::from(&project_path);
    
    if !path.exists() {
        return Err(format!("Project path does not exist: {}", project_path));
    }
    
    // Save project config
    let config_path = path.join("project.yaml");
    let content = serde_yaml_pretty(&config)?;
    
    fs::write(&config_path, content)
        .map_err(|e| format!("Failed to save project config: {}", e))?;
    
    // Add to recent list
    add_to_recent_list(&project_path)?;
    
    Ok(())
}

fn serde_yaml_pretty(value: &serde_json::Value) -> Result<String, String> {
    match value {
        serde_json::Value::Object(map) => {
            let mut lines: Vec<String> = Vec::new();
            
            for (key, val) in map.iter() {
                match val {
                    serde_json::Value::String(s) => {
                        if s.contains('\n') {
                            lines.push(format!("{}: |", key));
                            for line in s.lines() {
                                lines.push(format!("  {}", line));
                            }
                        } else {
                            lines.push(format!("{}: {}", key, s));
                        }
                    }
                    serde_json::Value::Number(n) => {
                        lines.push(format!("{}: {}", key, n));
                    }
                    serde_json::Value::Bool(b) => {
                        lines.push(format!("{}: {}", key, b));
                    }
                    serde_json::Value::Array(arr) => {
                        lines.push(format!("{}:", key));
                        for item in arr {
                            if let serde_json::Value::String(s) = item {
                                lines.push(format!("  - {}", s));
                            } else {
                                lines.push(format!("  - {}", item));
                            }
                        }
                    }
                    serde_json::Value::Object(nested) => {
                        lines.push(format!("{}:", key));
                        for (nested_key, nested_val) in nested.iter() {
                            if let serde_json::Value::String(s) = nested_val {
                                lines.push(format!("  {}: {}", nested_key, s));
                            } else {
                                lines.push(format!("  {}: {}", nested_key, nested_val));
                            }
                        }
                    }
                    _ => {
                        lines.push(format!("{}: {}", key, val));
                    }
                }
            }
            
            Ok(lines.join("\n"))
        }
        _ => Err("Expected object".to_string()),
    }
}
