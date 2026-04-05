use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

/// File or directory info
#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
}

/// Generic API response
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
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

fn system_time_to_string(time: SystemTime) -> String {
    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    format!("{}", secs)
}

/// Read text file content
#[tauri::command]
pub async fn read_text_file(path: String) -> Result<ApiResponse<String>, String> {
    println!("[read_text_file] Received path: {:?}", path);
    match fs::read_to_string(&path) {
        Ok(content) => Ok(ApiResponse::ok(content)),
        Err(e) => Ok(ApiResponse::err(format!("读取文件失败: {} (path: {:?})", e, path))),
    }
}

/// Read binary file as base64
#[tauri::command]
pub async fn read_binary_file(path: String) -> Result<ApiResponse<String>, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    println!("[read_binary_file] Received path: {:?}", path);

    match fs::read(&path) {
        Ok(bytes) => {
            let encoded = STANDARD.encode(&bytes);
            Ok(ApiResponse::ok(encoded))
        }
        Err(e) => Ok(ApiResponse::err(format!("读取文件失败: {} (path: {:?})", e, path))),
    }
}

/// Write text to file
#[tauri::command]
pub async fn write_text_file(path: String, content: String) -> Result<ApiResponse<()>, String> {
    match fs::write(&path, &content) {
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Ok(ApiResponse::err(format!("写入文件失败: {}", e))),
    }
}

/// Delete a file
#[tauri::command]
pub async fn delete_file(path: String) -> Result<ApiResponse<()>, String> {
    match fs::remove_file(&path) {
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Ok(ApiResponse::err(format!("删除文件失败: {}", e))),
    }
}

/// Rename a file or directory
#[tauri::command]
pub async fn rename_path(old_path: String, new_path: String) -> Result<ApiResponse<()>, String> {
    match fs::rename(&old_path, &new_path) {
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Ok(ApiResponse::err(format!("重命名失败: {}", e))),
    }
}

/// Create a directory
#[tauri::command]
pub async fn create_directory(path: String) -> Result<ApiResponse<()>, String> {
    match fs::create_dir_all(&path) {
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Ok(ApiResponse::err(format!("创建目录失败: {}", e))),
    }
}

/// Delete a directory
#[tauri::command]
pub async fn delete_directory(path: String) -> Result<ApiResponse<()>, String> {
    match fs::remove_dir_all(&path) {
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Ok(ApiResponse::err(format!("删除目录失败: {}", e))),
    }
}

/// List directory contents
#[tauri::command]
pub async fn list_directory(path: String) -> Result<ApiResponse<Vec<FileInfo>>, String> {
    println!("[list_directory] Received path: {:?}", path);
    let entries = match fs::read_dir(&path) {
        Ok(entries) => entries,
        Err(e) => return Ok(ApiResponse::err(format!("读取目录失败: {}", e))),
    };

    let mut files = Vec::new();
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let name = entry.file_name().to_string_lossy().to_string();
            let file_path = path.to_string_lossy().to_string();
            let is_dir = metadata.is_dir();
            let size = if is_dir { 0 } else { metadata.len() };
            let modified = metadata
                .modified()
                .map(system_time_to_string)
                .unwrap_or_default();

            files.push(FileInfo {
                name,
                path: file_path,
                is_dir,
                size,
                modified,
            });
        }
    }

    // Sort: directories first, then files, both alphabetically
    files.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(ApiResponse::ok(files))
}

/// Copy a file
#[tauri::command]
pub async fn copy_file(source: String, dest: String) -> Result<ApiResponse<()>, String> {
    match fs::copy(&source, &dest) {
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Ok(ApiResponse::err(format!("复制文件失败: {}", e))),
    }
}

/// Check if path exists
#[tauri::command]
pub async fn path_exists(path: String) -> Result<ApiResponse<bool>, String> {
    Ok(ApiResponse::ok(PathBuf::from(&path).exists()))
}
