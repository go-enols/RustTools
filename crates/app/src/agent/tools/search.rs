//! 文件搜索工具
//!
//! 在指定路径下搜索文件内容，使用walkdir遍历目录。
//! 支持glob模式过滤文件类型，返回匹配的文件路径、行号和内容摘要。

use super::tool::{Tool, ToolError, ToolResult};
use super::filesystem::check_path_allowed;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

/// 搜索结果条目
#[derive(Debug, Clone)]
struct SearchMatch {
    /// 文件路径
    file_path: String,
    /// 行号
    line_number: usize,
    /// 行内容
    line_content: String,
}

/// 文件搜索工具 — 在文件内容中搜索匹配指定模式的行
///
/// 参数:
/// - `path`: 搜索路径（必填）
/// - `pattern`: 搜索模式（必填，支持简单字符串匹配）
/// - `glob`: 文件过滤模式（可选，如"*.rs"）
pub struct SearchTool {
    allowed_directories: Vec<String>,
}

impl SearchTool {
    /// 创建新的文件搜索工具
    pub fn new(allowed_directories: Vec<String>) -> Self {
        Self {
            allowed_directories,
        }
    }

    /// 检查文件名是否匹配glob模式
    fn matches_glob(file_name: &str, pattern: &str) -> bool {
        // 简化实现：支持 *.ext 和 name.* 模式
        if pattern.starts_with("*.") {
            let ext = &pattern[2..];
            file_name.ends_with(ext)
        } else if pattern.ends_with(".*") {
            let prefix = &pattern[..pattern.len() - 2];
            let stem = file_name.rsplit_once('.').map(|(s, _)| s).unwrap_or(file_name);
            stem == prefix
        } else if pattern.contains('*') {
            // 简单通配符匹配
            let parts: Vec<&str> = pattern.split('*').collect();
            let mut remaining = file_name;
            for (i, part) in parts.iter().enumerate() {
                if part.is_empty() {
                    continue;
                }
                if let Some(pos) = remaining.find(part) {
                    if i == 0 && pos != 0 {
                        return false;
                    }
                    remaining = &remaining[pos + part.len()..];
                } else {
                    return false;
                }
            }
            true
        } else {
            file_name == pattern
        }
    }

    /// 在单个文件中搜索
    fn search_in_file(
        path: &Path,
        pattern: &str,
        max_results: usize,
    ) -> Result<Vec<SearchMatch>, ToolError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            ToolError::Filesystem(format!("读取文件失败 {}: {}", path.display(), e))
        })?;

        let mut matches = Vec::new();
        for (line_idx, line) in content.lines().enumerate() {
            if line.contains(pattern) {
                matches.push(SearchMatch {
                    file_path: path.to_string_lossy().to_string(),
                    line_number: line_idx + 1,
                    line_content: line.trim().to_string(),
                });
                if matches.len() >= max_results {
                    break;
                }
            }
        }

        Ok(matches)
    }
}

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str {
        "fs_search"
    }

    fn description(&self) -> &str {
        "在指定路径下搜索文件内容。返回匹配的文件路径、行号和内容摘要。支持按文件类型过滤。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "搜索路径"
                },
                "pattern": {
                    "type": "string",
                    "description": "搜索模式（简单字符串匹配）"
                },
                "glob": {
                    "type": "string",
                    "description": "文件过滤模式（如 *.rs, *.toml）",
                    "default": "*"
                }
            },
            "required": ["path", "pattern"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let path_str = params["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParameters("缺少path参数".to_string()))?;
        let pattern = params["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParameters("缺少pattern参数".to_string()))?;
        let glob = params["glob"].as_str().unwrap_or("*");

        let search_path = Path::new(path_str);
        check_path_allowed(search_path, &self.allowed_directories)?;

        if !search_path.exists() {
            return Err(ToolError::NotFound(format!(
                "搜索路径不存在: {}",
                path_str
            )));
        }

        if pattern.is_empty() {
            return Err(ToolError::InvalidParameters(
                "搜索模式不能为空".to_string(),
            ));
        }

        let max_results_per_file = 100;
        let max_total_results = 500;
        let mut all_matches = Vec::new();

        // 使用walkdir遍历
        for entry in WalkDir::new(search_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if all_matches.len() >= max_total_results {
                break;
            }

            let file_path = entry.path();

            // 跳过目录
            if !file_path.is_file() {
                continue;
            }

            // glob过滤
            if glob != "*" {
                if let Some(name) = file_path.file_name().and_then(|n| n.to_str()) {
                    if !Self::matches_glob(name, glob) {
                        continue;
                    }
                }
            }

            // 尝试作为文本文件搜索
            let remaining = max_total_results - all_matches.len();
            let file_max = max_results_per_file.min(remaining);
            match Self::search_in_file(file_path, pattern, file_max) {
                Ok(mut matches) => all_matches.append(&mut matches),
                Err(_) => {
                    // 跳过无法读取的文件（可能是二进制文件）
                    continue;
                }
            }
        }

        // 组装结果
        if all_matches.is_empty() {
            return Ok(ToolResult::ok(format!(
                "未找到匹配 '{}' 的内容",
                pattern
            )));
        }

        // 按文件分组
        let mut by_file: HashMap<String, Vec<SearchMatch>> = HashMap::new();
        for m in all_matches {
            by_file
                .entry(m.file_path.clone())
                .or_default()
                .push(m);
        }

        let mut lines = Vec::new();
        lines.push(format!(
            "找到 {} 个匹配（共 {} 个文件）:",
            by_file.values().map(|v| v.len()).sum::<usize>(),
            by_file.len()
        ));
        lines.push(String::new());

        for (file_path, matches) in by_file.iter().take(20) {
            lines.push(format!("📄 {}", file_path));
            for m in matches.iter().take(10) {
                let truncated = if m.line_content.len() > 120 {
                    format!("{}...", &m.line_content[..120])
                } else {
                    m.line_content.clone()
                };
                lines.push(format!("  L{:4}: {}", m.line_number, truncated));
            }
            if matches.len() > 10 {
                lines.push(format!("  ... 还有 {} 个匹配", matches.len() - 10));
            }
            lines.push(String::new());
        }

        if by_file.len() > 20 {
            lines.push(format!("... 还有 {} 个文件", by_file.len() - 20));
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
    async fn test_search_tool_basic() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();

        // 创建测试文件
        tokio::fs::write(temp.path().join("a.rs"), "fn main() {\n    println!(\"hello\");\n}\n")
            .await
            .unwrap();
        tokio::fs::write(temp.path().join("b.rs"), "fn helper() {\n    let x = 42;\n}\n")
            .await
            .unwrap();

        let tool = SearchTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": path.clone(),
                "pattern": "fn "
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("a.rs"));
        assert!(result.content.contains("b.rs"));
        assert!(result.content.contains("fn main"));
        assert!(result.content.contains("fn helper"));
    }

    #[tokio::test]
    async fn test_search_tool_with_glob() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();

        tokio::fs::write(temp.path().join("code.rs"), "fn code() {}\n")
            .await
            .unwrap();
        tokio::fs::write(temp.path().join("readme.md"), "# fn markdown\n")
            .await
            .unwrap();

        let tool = SearchTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": path.clone(),
                "pattern": "fn ",
                "glob": "*.rs"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("code.rs"));
        assert!(!result.content.contains("readme.md"));
    }

    #[tokio::test]
    async fn test_search_tool_no_match() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();

        tokio::fs::write(temp.path().join("file.txt"), "hello world\n")
            .await
            .unwrap();

        let tool = SearchTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": path.clone(),
                "pattern": "nonexistent_pattern"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("未找到匹配"));
    }

    #[tokio::test]
    async fn test_search_tool_not_allowed() {
        let tool = SearchTool::new(vec!["/some/other/dir".to_string()]);
        let result = tool
            .execute(json!({
                "path": "/etc",
                "pattern": "test"
            }))
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_matches_glob() {
        assert!(SearchTool::matches_glob("test.rs", "*.rs"));
        assert!(!SearchTool::matches_glob("test.rs", "*.toml"));
        assert!(SearchTool::matches_glob("lib.rs", "*.rs"));
        assert!(SearchTool::matches_glob("test.rs", "test.*"));
        assert!(!SearchTool::matches_glob("other.rs", "test.*"));
        assert!(SearchTool::matches_glob("test.rs", "*"));
    }

    #[test]
    fn test_search_in_file() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("test.txt");
        std::fs::write(&file, "line one\nline two\nline three\n").unwrap();

        let results = SearchTool::search_in_file(&file, "two", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line_number, 2);
        assert_eq!(results[0].line_content, "line two");
    }

    #[test]
    fn test_search_in_file_max_results() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("test.txt");
        std::fs::write(&file, "test\ntest\ntest\ntest\n").unwrap();

        let results = SearchTool::search_in_file(&file, "test", 2).unwrap();
        assert_eq!(results.len(), 2);
    }
}
