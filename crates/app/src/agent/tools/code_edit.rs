//! 代码编辑工具
//!
//! 提供精确的代码替换功能，在文件中查找old_string并替换为new_string。
//! 支持整文件替换。所有操作经过路径安全检查。

use super::tool::{Tool, ToolError, ToolResult};
use super::filesystem::check_path_allowed;
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

/// 代码编辑工具 — 在文件中精确替换代码片段
///
/// 参数:
/// - `path`: 文件路径（必填）
/// - `old_string`: 要替换的旧字符串（可选，为空则插入到文件开头）
/// - `new_string`: 新字符串（必填）
pub struct CodeEditTool {
    allowed_directories: Vec<String>,
}

impl CodeEditTool {
    /// 创建新的代码编辑工具
    pub fn new(allowed_directories: Vec<String>) -> Self {
        Self {
            allowed_directories,
        }
    }

    /// 执行替换操作
    fn do_replace(content: &str, old: &str, new: &str) -> Result<String, ToolError> {
        if old.is_empty() {
            // old_string为空，插入到文件开头
            Ok(format!("{new}{content}"))
        } else {
            let count = content.matches(old).count();
            match count {
                0 => Err(ToolError::NotFound(format!(
                    "未在文件中找到匹配内容: '{}...'",
                    &old[..old.len().min(30)]
                ))),
                1 => Ok(content.replace(old, new)),
                _ => Err(ToolError::Other(format!(
                    "找到 {} 处匹配，请提供更精确的匹配内容",
                    count
                ))),
            }
        }
    }
}

#[async_trait]
impl Tool for CodeEditTool {
    fn name(&self) -> &str {
        "code_edit"
    }

    fn description(&self) -> &str {
        "在文件中精确替换代码。old_string必须唯一匹配文件中的内容。如果old_string为空，则插入到文件开头。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "文件路径"
                },
                "old_string": {
                    "type": "string",
                    "description": "要替换的旧字符串（必须唯一匹配）"
                },
                "new_string": {
                    "type": "string",
                    "description": "新字符串"
                }
            },
            "required": ["path", "new_string"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let path_str = params["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParameters("缺少path参数".to_string()))?;
        let old_string = params["old_string"].as_str().unwrap_or("");
        let new_string = params["new_string"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParameters("缺少new_string参数".to_string()))?;

        let path = Path::new(path_str);
        check_path_allowed(path, &self.allowed_directories)?;

        if !path.exists() {
            return Err(ToolError::NotFound(format!("文件不存在: {}", path_str)));
        }

        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            ToolError::Filesystem(format!("读取文件失败: {}", e))
        })?;

        let new_content = Self::do_replace(&content, old_string, new_string)?;

        tokio::fs::write(path, new_content.as_bytes())
            .await
            .map_err(|e| ToolError::Filesystem(format!("写入文件失败: {}", e)))?;

        let action = if old_string.is_empty() {
            "插入"
        } else {
            "替换"
        };

        Ok(ToolResult::ok(format!(
            "{}成功: {} (旧内容: {} 字节, 新内容: {} 字节)",
            action,
            path_str,
            content.len(),
            new_content.len()
        )))
    }
}

// ============================================================================
// 整文件替换工具
// ============================================================================

/// 整文件替换工具 — 完全替换文件内容
///
/// 参数:
/// - `path`: 文件路径（必填）
/// - `content`: 新的文件内容（必填）
pub struct CodeReplaceTool {
    allowed_directories: Vec<String>,
}

impl CodeReplaceTool {
    /// 创建新的整文件替换工具
    pub fn new(allowed_directories: Vec<String>) -> Self {
        Self {
            allowed_directories,
        }
    }
}

#[async_trait]
impl Tool for CodeReplaceTool {
    fn name(&self) -> &str {
        "code_replace"
    }

    fn description(&self) -> &str {
        "完全替换文件内容。旧内容将被全部覆盖，谨慎使用。"
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
                    "description": "新的完整文件内容"
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

        let path = Path::new(path_str);
        check_path_allowed(path, &self.allowed_directories)?;

        if !path.exists() {
            return Err(ToolError::NotFound(format!("文件不存在: {}", path_str)));
        }

        if !path.is_file() {
            return Err(ToolError::Filesystem(format!(
                "路径不是文件: {}",
                path_str
            )));
        }

        // 备份原文件（简单实现：读取原内容但不保存备份文件）
        let old_content = tokio::fs::read_to_string(path).await.map_err(|e| {
            ToolError::Filesystem(format!("读取原文件失败: {}", e))
        })?;

        tokio::fs::write(path, content.as_bytes())
            .await
            .map_err(|e| ToolError::Filesystem(format!("写入文件失败: {}", e)))?;

        Ok(ToolResult::ok(format!(
            "整文件替换成功: {} (旧: {} 字节, 新: {} 字节)",
            path_str,
            old_content.len(),
            content.len()
        )))
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
    async fn test_code_edit_replace() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();
        let file = temp.path().join("test.rs");
        tokio::fs::write(&file, "fn old_func() {}\nfn other() {}\n")
            .await
            .unwrap();

        let tool = CodeEditTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": file.to_string_lossy().to_string(),
                "old_string": "fn old_func() {}",
                "new_string": "fn new_func() -> i32 { 42 }"
            }))
            .await
            .unwrap();

        assert!(result.success);

        let content = tokio::fs::read_to_string(&file).await.unwrap();
        assert!(content.contains("fn new_func() -> i32 { 42 }"));
        assert!(content.contains("fn other() {}"));
    }

    #[tokio::test]
    async fn test_code_edit_insert_at_beginning() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();
        let file = temp.path().join("test.rs");
        tokio::fs::write(&file, "existing content\n").await.unwrap();

        let tool = CodeEditTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": file.to_string_lossy().to_string(),
                "old_string": "",
                "new_string": "// header comment\n"
            }))
            .await
            .unwrap();

        assert!(result.success);

        let content = tokio::fs::read_to_string(&file).await.unwrap();
        assert!(content.starts_with("// header comment\n"));
    }

    #[tokio::test]
    async fn test_code_edit_not_found() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();
        let file = temp.path().join("test.rs");
        tokio::fs::write(&file, "some content\n").await.unwrap();

        let tool = CodeEditTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": file.to_string_lossy().to_string(),
                "old_string": "nonexistent text",
                "new_string": "replacement"
            }))
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("未找到"));
    }

    #[tokio::test]
    async fn test_code_edit_multiple_matches() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();
        let file = temp.path().join("test.rs");
        tokio::fs::write(&file, "hello\nhello\nhello\n")
            .await
            .unwrap();

        let tool = CodeEditTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": file.to_string_lossy().to_string(),
                "old_string": "hello",
                "new_string": "world"
            }))
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("找到 3 处匹配"));
    }

    #[tokio::test]
    async fn test_code_replace_whole_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();
        let file = temp.path().join("test.rs");
        tokio::fs::write(&file, "old content here\nmore old\n")
            .await
            .unwrap();

        let tool = CodeReplaceTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": file.to_string_lossy().to_string(),
                "content": "completely new content\n"
            }))
            .await
            .unwrap();

        assert!(result.success);

        let content = tokio::fs::read_to_string(&file).await.unwrap();
        assert_eq!(content, "completely new content\n");
    }

    #[test]
    fn test_do_replace_unique_match() {
        let content = "fn a() {}\nfn b() {}\n";
        let result = CodeEditTool::do_replace(content, "fn a() {}", "fn new_a() {}").unwrap();
        assert_eq!(result, "fn new_a() {}\nfn b() {}\n");
    }

    #[test]
    fn test_do_replace_empty_old() {
        let content = "existing\n";
        let result = CodeEditTool::do_replace(content, "", "prefix\n").unwrap();
        assert_eq!(result, "prefix\nexisting\n");
    }

    #[test]
    fn test_do_replace_no_match() {
        let content = "hello world";
        let result = CodeEditTool::do_replace(content, "nonexistent", "replacement");
        assert!(result.is_err());
    }

    #[test]
    fn test_do_replace_multiple_matches() {
        let content = "aaa aaa aaa";
        let result = CodeEditTool::do_replace(content, "aaa", "bbb");
        assert!(result.is_err());
    }
}
