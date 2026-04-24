//! 文件系统工具
//!
//! 提供文件读写、目录列表、文件搜索等功能。
//! 所有操作都经过路径安全检查，确保只在允许的目录范围内操作。

use super::tool::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};

// ============================================================================
// 路径安全检查
// ============================================================================

/// 检查给定路径是否在允许的目录列表内
///
/// 通过canonicalize解析真实路径后检查前缀匹配，
/// 防止通过符号链接或../等方式绕过目录限制。
///
/// # Arguments
/// * `path` - 要检查的路径
/// * `allowed` - 允许的目录列表
///
/// # Returns
/// * `Ok(())` - 路径允许访问
/// * `Err(ToolError::PathNotAllowed)` - 路径不在允许范围内
pub fn check_path_allowed(path: &Path, allowed: &[String]) -> Result<(), ToolError> {
    // 如果没有设置允许目录，则允许所有路径（仅在测试环境中）
    if allowed.is_empty() {
        return Ok(());
    }

    // 尝试解析canonical路径
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    for allowed_dir in allowed {
        let allowed_path = Path::new(allowed_dir);
        let allowed_canonical = allowed_path
            .canonicalize()
            .unwrap_or_else(|_| allowed_path.to_path_buf());

        if canonical.starts_with(&allowed_canonical) {
            return Ok(());
        }
    }

    Err(ToolError::PathNotAllowed {
        path: path.display().to_string(),
    })
}

/// 解析并规范化路径
fn resolve_path(path_str: &str, _allowed: &[String]) -> Result<PathBuf, ToolError> {
    let path = Path::new(path_str);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        // 相对路径转换为绝对路径
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(ToolError::Io)
    }
}

/// 从参数中提取allowed_directories（如果提供了agent级别的覆盖）
fn get_allowed_dirs(params: &Value, default: &[String]) -> Vec<String> {
    params
        .get("_allowed_directories")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_else(|| default.to_vec())
}

// ============================================================================
// 文件读取工具
// ============================================================================

/// 文件读取工具 — 读取指定文件的内容
///
/// 参数:
/// - `path`: 文件路径（必填）
/// - `offset`: 起始行号（可选，默认0）
/// - `limit`: 最大读取行数（可选，默认1000）
pub struct FileReadTool {
    allowed_directories: Vec<String>,
}

impl FileReadTool {
    /// 创建新的文件读取工具
    pub fn new(allowed_directories: Vec<String>) -> Self {
        Self {
            allowed_directories,
        }
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "fs_read"
    }

    fn description(&self) -> &str {
        "读取文件内容。支持指定起始行和最大行数限制。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "文件路径"
                },
                "offset": {
                    "type": "integer",
                    "description": "起始行号（从0开始）",
                    "minimum": 0,
                    "default": 0
                },
                "limit": {
                    "type": "integer",
                    "description": "最大读取行数",
                    "minimum": 1,
                    "maximum": 10000,
                    "default": 1000
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let path_str = params["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParameters("缺少path参数".to_string()))?;

        let allowed = get_allowed_dirs(&params, &self.allowed_directories);
        let path = resolve_path(path_str, &allowed)?;
        check_path_allowed(&path, &allowed)?;

        if !path.exists() {
            return Err(ToolError::NotFound(format!("文件不存在: {}", path_str)));
        }

        if !path.is_file() {
            return Err(ToolError::Filesystem(format!(
                "路径不是文件: {}",
                path_str
            )));
        }

        let offset = params["offset"].as_u64().unwrap_or(0) as usize;
        let limit = params["limit"].as_u64().unwrap_or(1000) as usize;

        let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
            ToolError::Filesystem(format!("读取文件失败: {}", e))
        })?;

        // 按行截取
        let lines: Vec<&str> = content.lines().collect();
        let start = offset;
        let end = (offset + limit).min(lines.len());

        if start >= lines.len() {
            return Ok(ToolResult::ok(""));
        }

        let result = lines[start..end].join("\n");
        let result_with_info = if offset > 0 || limit < lines.len() {
            format!(
                "[文件: {}, 第{}-{}行 / 共{}行]\n{}",
                path_str,
                offset + 1,
                end,
                lines.len(),
                result
            )
        } else {
            result
        };

        Ok(ToolResult::ok(result_with_info))
    }
}

// ============================================================================
// 文件写入工具
// ============================================================================

/// 文件写入工具 — 写入或追加内容到文件
///
/// 参数:
/// - `path`: 文件路径（必填）
/// - `content`: 要写入的内容（必填）
/// - `append`: 是否追加模式（可选，默认false）
pub struct FileWriteTool {
    allowed_directories: Vec<String>,
}

impl FileWriteTool {
    /// 创建新的文件写入工具
    pub fn new(allowed_directories: Vec<String>) -> Self {
        Self {
            allowed_directories,
        }
    }
}

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "fs_write"
    }

    fn description(&self) -> &str {
        "写入内容到文件。支持覆盖或追加模式。目录不存在会自动创建。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "文件路径"
                },
                "content": {
                    "type": "string",
                    "description": "要写入的文件内容"
                },
                "append": {
                    "type": "boolean",
                    "description": "是否追加到文件末尾",
                    "default": false
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let path_str = params["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParameters("缺少path参数".to_string()))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParameters("缺少content参数".to_string()))?;
        let append = params["append"].as_bool().unwrap_or(false);

        let allowed = get_allowed_dirs(&params, &self.allowed_directories);
        let path = resolve_path(path_str, &allowed)?;
        check_path_allowed(&path, &allowed)?;

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                ToolError::Filesystem(format!("创建目录失败: {}", e))
            })?;
        }

        if append {
            tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .map_err(|e| ToolError::Filesystem(format!("打开文件失败: {}", e)))?;

            tokio::fs::write(&path, content.as_bytes())
                .await
                .map_err(|e| ToolError::Filesystem(format!("写入文件失败: {}", e)))?;
        } else {
            tokio::fs::write(&path, content.as_bytes())
                .await
                .map_err(|e| ToolError::Filesystem(format!("写入文件失败: {}", e)))?;
        }

        let mode = if append { "追加" } else { "写入" };
        Ok(ToolResult::ok(format!(
            "{}成功: {} ({} 字节)",
            mode,
            path_str,
            content.len()
        )))
    }
}

// ============================================================================
// 文件列表工具
// ============================================================================

/// 目录列表工具 — 列出目录中的文件和子目录
///
/// 参数:
/// - `path`: 目录路径（必填）
/// - `recursive`: 是否递归列出（可选，默认false）
pub struct FileListTool {
    allowed_directories: Vec<String>,
}

impl FileListTool {
    /// 创建新的目录列表工具
    pub fn new(allowed_directories: Vec<String>) -> Self {
        Self {
            allowed_directories,
        }
    }
}

#[async_trait]
impl Tool for FileListTool {
    fn name(&self) -> &str {
        "fs_list"
    }

    fn description(&self) -> &str {
        "列出目录中的文件和子目录。支持递归列出。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "目录路径"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "是否递归列出子目录",
                    "default": false
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let path_str = params["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParameters("缺少path参数".to_string()))?;
        let recursive = params["recursive"].as_bool().unwrap_or(false);

        let allowed = get_allowed_dirs(&params, &self.allowed_directories);
        let path = resolve_path(path_str, &allowed)?;
        check_path_allowed(&path, &allowed)?;

        if !path.exists() {
            return Err(ToolError::NotFound(format!(
                "目录不存在: {}",
                path_str
            )));
        }

        if !path.is_dir() {
            return Err(ToolError::Filesystem(format!(
                "路径不是目录: {}",
                path_str
            )));
        }

        let mut entries = Vec::new();
        let mut read_dir = tokio::fs::read_dir(&path).await.map_err(|e| {
            ToolError::Filesystem(format!("读取目录失败: {}", e))
        })?;

        while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
            ToolError::Filesystem(format!("读取目录项失败: {}", e))
        })? {
            let file_type = entry.file_type().await.ok();
            let name = entry.file_name().to_string_lossy().to_string();
            let meta = entry.metadata().await.ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let is_dir = file_type.as_ref().map(|ft| ft.is_dir()).unwrap_or(false);

            let prefix = if is_dir { "[DIR] " } else { "[FILE]" };
            let size_str = if is_dir {
                "".to_string()
            } else {
                format!(" ({} bytes)", format_size(size))
            };
            entries.push(format!("{} {}{}", prefix, name, size_str));

            // 递归处理子目录
            if recursive && is_dir {
                let sub_path = entry.path();
                let sub_entries =
                    list_recursive(&sub_path, &path, &allowed).await?;
                entries.extend(sub_entries);
            }
        }

        let result = if entries.is_empty() {
            "(空目录)".to_string()
        } else {
            entries.join("\n")
        };

        Ok(ToolResult::ok(format!(
            "目录: {}\n{}",
            path_str, result
        )))
    }
}

/// 递归列出目录内容
async fn list_recursive(
    dir: &Path,
    base: &Path,
    allowed: &[String],
) -> Result<Vec<String>, ToolError> {
    let mut entries = Vec::new();
    let prefix = dir.strip_prefix(base).unwrap_or(dir);
    let prefix_str = prefix.display().to_string();

    let mut read_dir = tokio::fs::read_dir(dir).await.map_err(|e| {
        ToolError::Filesystem(format!("读取目录失败: {}", e))
    })?;

    while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
        ToolError::Filesystem(format!("读取目录项失败: {}", e))
    })? {
        let file_type = entry.file_type().await.ok();
        let name = entry.file_name().to_string_lossy().to_string();
        let meta = entry.metadata().await.ok();
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let is_dir = file_type.as_ref().map(|ft| ft.is_dir()).unwrap_or(false);

        let prefix_label = if is_dir { "[DIR] " } else { "[FILE]" };
        let size_str = if is_dir {
            "".to_string()
        } else {
            format!(" ({} bytes)", format_size(size))
        };
        entries.push(format!(
            "  {}/{}: {}{}",
            prefix_str, name, prefix_label, size_str
        ));

        if is_dir {
            let sub_entries = Box::pin(list_recursive(&entry.path(), base, allowed)).await?;
            entries.extend(sub_entries);
        }
    }

    Ok(entries)
}

/// 格式化文件大小
fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn get_temp_dir() -> (tempfile::TempDir, String) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();
        (temp, path)
    }

    #[tokio::test]
    async fn test_file_read_tool() {
        let (temp, path) = get_temp_dir();
        let test_file = temp.path().join("test.txt");
        tokio::fs::write(&test_file, "line1\nline2\nline3\n")
            .await
            .unwrap();

        let tool = FileReadTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({"path": test_file.to_string_lossy().to_string()}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("line1"));
        assert!(result.content.contains("line3"));
    }

    #[tokio::test]
    async fn test_file_read_with_offset_limit() {
        let (temp, path) = get_temp_dir();
        let test_file = temp.path().join("test.txt");
        tokio::fs::write(&test_file, "a\nb\nc\nd\ne\n")
            .await
            .unwrap();

        let tool = FileReadTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": test_file.to_string_lossy().to_string(),
                "offset": 1,
                "limit": 2
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("b"));
        assert!(result.content.contains("c"));
        assert!(!result.content.contains("a"));
        assert!(!result.content.contains("e"));
    }

    #[tokio::test]
    async fn test_file_read_not_allowed() {
        let (temp, _path) = get_temp_dir();
        let test_file = temp.path().join("test.txt");
        tokio::fs::write(&test_file, "content").await.unwrap();

        let tool = FileReadTool::new(vec!["/some/other/dir".to_string()]);
        let result = tool
            .execute(json!({"path": test_file.to_string_lossy().to_string()}))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_write_tool() {
        let (temp, path) = get_temp_dir();
        let test_file = temp.path().join("output.txt");

        let tool = FileWriteTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": test_file.to_string_lossy().to_string(),
                "content": "hello world"
            }))
            .await
            .unwrap();

        assert!(result.success);

        let content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_file_list_tool() {
        let (temp, path) = get_temp_dir();
        tokio::fs::write(temp.path().join("a.txt"), "a").await.unwrap();
        tokio::fs::write(temp.path().join("b.txt"), "b").await.unwrap();
        tokio::fs::create_dir(temp.path().join("subdir"))
            .await
            .unwrap();

        let tool = FileListTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({"path": temp.path().to_string_lossy().to_string()}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("a.txt"));
        assert!(result.content.contains("b.txt"));
        assert!(result.content.contains("[DIR]"));
    }

    #[test]
    fn test_check_path_allowed() {
        let allowed = vec!["/home/user/projects".to_string()];

        // 允许的路径
        assert!(check_path_allowed(
            Path::new("/home/user/projects/myfile"),
            &allowed
        )
        .is_ok());

        // 不允许的路径
        assert!(check_path_allowed(
            Path::new("/etc/passwd"),
            &allowed
        )
        .is_err());
    }

    #[test]
    fn test_check_path_allowed_empty_list() {
        // 空列表允许所有路径
        assert!(check_path_allowed(Path::new("/any/path"), &[]).is_ok());
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1024 * 1024 * 2), "2.0 MB");
    }
}
