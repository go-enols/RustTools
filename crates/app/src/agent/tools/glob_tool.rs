//! 文件 glob 搜索工具
//!
//! 使用 glob 模式查找文件和目录，支持递归搜索。
//! 比 fs_list 更强大的文件发现能力。

use super::tool::{Tool, ToolError, ToolResult};
use super::filesystem::check_path_allowed;
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Glob 文件搜索工具
///
/// 参数:
/// - `pattern`: glob 模式（必填，如 "src/**/*.rs"）
/// - `directory`: 搜索目录（可选，默认当前目录）
/// - `include_dirs`: 是否包含目录（可选，默认 true）
pub struct GlobTool {
    allowed_directories: Vec<String>,
}

impl GlobTool {
    pub fn new(allowed_directories: Vec<String>) -> Self {
        Self {
            allowed_directories,
        }
    }

    /// 验证 glob 模式安全性
    fn validate_pattern(pattern: &str) -> Result<(), ToolError> {
        // 拒绝可能导致性能问题的模式
        if pattern.starts_with("**/") && pattern.ends_with("/**") {
            return Ok(()); // 这是合法的递归模式
        }
        
        // 拒绝纯 ** 模式（会递归搜索所有文件，可能极大）
        if pattern.trim() == "**" || pattern.trim() == "**/*" {
            return Err(ToolError::InvalidParameters(
                "模式 '**' 或 '**/*' 范围太大，请使用更具体的模式如 'src/**/*.rs'".to_string()
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "使用 glob 模式查找文件和目录。支持标准 glob 语法如 *、?、** 等。\
         示例: 'src/**/*.rs' 查找 src 目录下所有 .rs 文件，\
         '*.config.*' 查找所有 config 文件。\
         注意: 不要以 '**' 开头，应指定起始目录如 'src/**'。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "glob 模式，如 'src/**/*.rs', '*.toml', 'test_*.py'"
                },
                "directory": {
                    "type": "string",
                    "description": "搜索起始目录，默认为当前工作目录",
                    "default": "."
                },
                "include_dirs": {
                    "type": "boolean",
                    "description": "是否在结果中包含目录",
                    "default": true
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let pattern = params["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParameters("缺少 pattern 参数".to_string()))?;
        
        let directory = params["directory"].as_str().unwrap_or(".");
        let include_dirs = params["include_dirs"].as_bool().unwrap_or(true);

        if pattern.is_empty() {
            return Err(ToolError::InvalidParameters("glob 模式不能为空".to_string()));
        }

        Self::validate_pattern(pattern)?;

        // 构建完整路径模式
        let base_path = Path::new(directory);
        check_path_allowed(base_path, &self.allowed_directories)?;

        let full_pattern = if Path::new(pattern).is_absolute() {
            pattern.to_string()
        } else {
            PathBuf::from(directory).join(pattern)
                .to_string_lossy()
                .to_string()
        };

        let mut entries = Vec::new();
        let mut dir_count = 0;
        let mut file_count = 0;

        match glob::glob(&full_pattern) {
            Ok(paths) => {
                for entry in paths {
                    match entry {
                        Ok(path) => {
                            // 路径安全检查
                            if check_path_allowed(&path, &self.allowed_directories).is_err() {
                                continue;
                            }

                            let is_dir = path.is_dir();
                            
                            if is_dir && !include_dirs {
                                continue;
                            }

                            let path_str = path.to_string_lossy().to_string();
                            let prefix = if is_dir { "[DIR] " } else { "[FILE]" };
                            
                            if is_dir {
                                dir_count += 1;
                                entries.push(format!("{} {}", prefix, path_str));
                            } else {
                                file_count += 1;
                                let size = std::fs::metadata(&path)
                                    .map(|m| m.len())
                                    .unwrap_or(0);
                                entries.push(format!("{} {} ({} bytes)", prefix, path_str, size));
                            }
                        }
                        Err(e) => {
                            return Err(ToolError::Filesystem(format!("glob 遍历错误: {}", e)));
                        }
                    }
                }
            }
            Err(e) => {
                return Err(ToolError::InvalidParameters(format!("无效的 glob 模式: {}", e)));
            }
        }

        entries.sort();

        let mut lines = Vec::new();
        lines.push(format!("模式: {} (目录: {})", pattern, directory));
        lines.push(format!("结果: {} 个文件, {} 个目录", file_count, dir_count));
        lines.push(String::new());
        
        if entries.is_empty() {
            lines.push("(无匹配结果)".to_string());
        } else {
            for entry in entries {
                lines.push(entry);
            }
        }

        Ok(ToolResult::ok(lines.join("\n")))
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_glob_tool_basic() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();

        std::fs::write(temp.path().join("a.rs"), "").unwrap();
        std::fs::write(temp.path().join("b.rs"), "").unwrap();
        std::fs::write(temp.path().join("c.toml"), "").unwrap();

        let tool = GlobTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "directory": path.clone(),
                "pattern": "*.rs"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("a.rs"));
        assert!(result.content.contains("b.rs"));
        assert!(!result.content.contains("c.toml"));
    }

    #[tokio::test]
    async fn test_glob_tool_recursive() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();

        std::fs::create_dir(temp.path().join("src")).unwrap();
        std::fs::create_dir(temp.path().join("src").join("sub")).unwrap();
        std::fs::write(temp.path().join("src").join("main.rs"), "").unwrap();
        std::fs::write(temp.path().join("src").join("sub").join("lib.rs"), "").unwrap();

        let tool = GlobTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "directory": path.clone(),
                "pattern": "src/**/*.rs"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("main.rs"));
        assert!(result.content.contains("lib.rs"));
    }

    #[tokio::test]
    async fn test_glob_tool_rejects_dangerous_pattern() {
        let tool = GlobTool::new(vec![]);
        let result = tool
            .execute(json!({
                "pattern": "**"
            }))
            .await;

        assert!(result.is_err());
    }
}
