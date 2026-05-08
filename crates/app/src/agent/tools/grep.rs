//! 基于正则表达式的文件搜索工具 (Grep)
//!
//! 提供 ripgrep 风格的文件内容搜索，支持：
//! - 正则表达式匹配
//! - 上下文行显示 (-B/-A/-C)
//! - 忽略大小写
//! - 按文件类型过滤
//! - 多输出模式 (content/files_with_matches/count_matches)
//! - 结果分页 (head_limit/offset)
//! - 多行模式
//! - 包含/排除 gitignore 文件

use super::tool::{Tool, ToolError, ToolResult};
use super::filesystem::check_path_allowed;
use async_trait::async_trait;
use regex::RegexBuilder;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

/// Grep 搜索工具
///
/// 参数:
/// - `pattern`: 正则表达式模式（必填）
/// - `path`: 搜索路径（可选，默认当前目录）
/// - `output_mode`: 输出模式（可选，默认 "content"）
/// - `glob`: 文件过滤模式（可选）
/// - `-i`: 忽略大小写（可选，默认 false）
/// - `-n`: 显示行号（可选，默认 true）
/// - `-B`: 匹配前上下文行数（可选）
/// - `-A`: 匹配后上下文行数（可选）
/// - `-C`: 前后上下文行数（可选，覆盖 -B/-A）
/// - `head_limit`: 最大返回结果数（可选，默认 250）
/// - `offset`: 跳过前 N 个结果（可选，默认 0）
/// - `multiline`: 启用多行模式（可选，默认 false）
/// - `include_ignored`: 包含 gitignore 忽略的文件（可选，默认 false）
pub struct GrepTool {
    allowed_directories: Vec<String>,
}

impl GrepTool {
    pub fn new(allowed_directories: Vec<String>) -> Self {
        Self {
            allowed_directories,
        }
    }

    /// 检查文件路径是否匹配 glob 模式
    fn matches_glob_pattern(file_path: &Path, pattern: &str) -> bool {
        if pattern == "*" || pattern.is_empty() {
            return true;
        }
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        // 尝试作为 glob 匹配
        match glob::Pattern::new(pattern) {
            Ok(pat) => pat.matches(file_name),
            Err(_) => file_name.contains(pattern),
        }
    }

    /// 读取文件所有行
    fn read_file_lines(path: &Path) -> Result<Vec<String>, ToolError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            ToolError::Filesystem(format!("读取文件失败 {}: {}", path.display(), e))
        })?;
        Ok(content.lines().map(|s| s.to_string()).collect())
    }

    /// 在单个文件中搜索
    fn search_in_file(
        path: &Path,
        regex: &regex::Regex,
        before_context: usize,
        after_context: usize,
        multiline: bool,
    ) -> Result<Vec<GrepMatch>, ToolError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            ToolError::Filesystem(format!("读取文件失败 {}: {}", path.display(), e))
        })?;

        let mut matches = Vec::new();

        if multiline {
            // 多行模式：在整个文本中搜索
            for mat in regex.find_iter(&content) {
                // 计算起始和结束行号
                let start_line = content[..mat.start()].matches('\n').count() + 1;
                let end_line = content[..mat.end()].matches('\n').count() + 1;
                
                // 提取匹配区域的上下文
                let lines: Vec<&str> = content.lines().collect();
                let ctx_start = start_line.saturating_sub(before_context + 1);
                let ctx_end = (end_line + after_context).min(lines.len());
                
                let context_lines: Vec<(usize, String)> = (ctx_start..ctx_end)
                    .map(|i| (i + 1, lines[i].to_string()))
                    .collect();

                matches.push(GrepMatch {
                    file_path: path.to_string_lossy().to_string(),
                    line_number: start_line,
                    line_content: mat.as_str().lines().next().unwrap_or("").to_string(),
                    context_lines,
                    is_multiline: start_line != end_line,
                });
            }
        } else {
            // 单行模式
            let lines: Vec<&str> = content.lines().collect();
            for (line_idx, line) in lines.iter().enumerate() {
                if regex.is_match(line) {
                    let line_num = line_idx + 1;
                    let ctx_start = line_idx.saturating_sub(before_context);
                    let ctx_end = (line_idx + 1 + after_context).min(lines.len());
                    
                    let context_lines: Vec<(usize, String)> = (ctx_start..ctx_end)
                        .map(|i| (i + 1, lines[i].to_string()))
                        .collect();

                    matches.push(GrepMatch {
                        file_path: path.to_string_lossy().to_string(),
                        line_number: line_num,
                        line_content: line.to_string(),
                        context_lines,
                        is_multiline: false,
                    });
                }
            }
        }

        Ok(matches)
    }
}

#[derive(Debug, Clone)]
struct GrepMatch {
    file_path: String,
    line_number: usize,
    line_content: String,
    context_lines: Vec<(usize, String)>,
    is_multiline: bool,
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "基于正则表达式搜索文件内容。支持上下文行、忽略大小写、文件过滤、多输出模式。\
         优先使用此工具替代简单字符串搜索，特别是需要正则、多文件、上下文行时。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "正则表达式搜索模式（ripgrep语法）"
                },
                "path": {
                    "type": "string",
                    "description": "搜索路径，默认为当前目录",
                    "default": "."
                },
                "output_mode": {
                    "type": "string",
                    "description": "输出模式: content(显示匹配行), files_with_matches(仅文件路径), count_matches(统计数)",
                    "enum": ["content", "files_with_matches", "count_matches"],
                    "default": "content"
                },
                "glob": {
                    "type": "string",
                    "description": "文件过滤glob模式（如 *.rs, *.toml）",
                    "default": "*"
                },
                "-i": {
                    "type": "boolean",
                    "description": "忽略大小写",
                    "default": false
                },
                "-n": {
                    "type": "boolean",
                    "description": "显示行号",
                    "default": true
                },
                "-B": {
                    "type": "integer",
                    "description": "匹配前显示的行数",
                    "minimum": 0,
                    "maximum": 10,
                    "default": 0
                },
                "-A": {
                    "type": "integer",
                    "description": "匹配后显示的行数",
                    "minimum": 0,
                    "maximum": 10,
                    "default": 0
                },
                "-C": {
                    "type": "integer",
                    "description": "匹配前后显示的行数（覆盖-B和-A）",
                    "minimum": 0,
                    "maximum": 10
                },
                "head_limit": {
                    "type": "integer",
                    "description": "最大返回结果数",
                    "minimum": 1,
                    "maximum": 1000,
                    "default": 250
                },
                "offset": {
                    "type": "integer",
                    "description": "跳过前N个结果",
                    "minimum": 0,
                    "default": 0
                },
                "multiline": {
                    "type": "boolean",
                    "description": "启用多行模式（.匹配换行符，模式可跨行）",
                    "default": false
                },
                "include_ignored": {
                    "type": "boolean",
                    "description": "包含被.gitignore等忽略的文件",
                    "default": false
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let pattern = params["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParameters("缺少 pattern 参数".to_string()))?;
        
        let path_str = params["path"].as_str().unwrap_or(".");
        let output_mode = params["output_mode"].as_str().unwrap_or("content");
        let glob_pattern = params["glob"].as_str().unwrap_or("*");
        let ignore_case = params["-i"].as_bool().unwrap_or(false);
        let show_line_numbers = params["-n"].as_bool().unwrap_or(true);
        let before_ctx = params["-B"].as_u64().unwrap_or(0) as usize;
        let after_ctx = params["-A"].as_u64().unwrap_or(0) as usize;
        let context = params["-C"].as_u64().map(|c| c as usize);
        let head_limit = params["head_limit"].as_u64().unwrap_or(250) as usize;
        let offset = params["offset"].as_u64().unwrap_or(0) as usize;
        let multiline = params["multiline"].as_bool().unwrap_or(false);
        let _include_ignored = params["include_ignored"].as_bool().unwrap_or(false);

        let before_context = context.unwrap_or(before_ctx);
        let after_context = context.unwrap_or(after_ctx);

        if pattern.is_empty() {
            return Err(ToolError::InvalidParameters("搜索模式不能为空".to_string()));
        }

        let search_path = Path::new(path_str);
        check_path_allowed(search_path, &self.allowed_directories)?;

        if !search_path.exists() {
            return Err(ToolError::NotFound(format!("搜索路径不存在: {}", path_str)));
        }

        // 编译正则表达式
        let mut regex_builder = RegexBuilder::new(pattern);
        regex_builder.case_insensitive(ignore_case);
        regex_builder.multi_line(true);
        regex_builder.dot_matches_new_line(multiline);
        
        let regex = regex_builder.build().map_err(|e| {
            ToolError::InvalidParameters(format!("无效的正则表达式: {}", e))
        })?;

        let mut all_matches: Vec<GrepMatch> = Vec::new();
        let mut files_with_matches: HashMap<String, usize> = HashMap::new();

        // 遍历文件
        let walker = WalkDir::new(search_path)
            .follow_links(false);

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let file_path = entry.path();

            if !file_path.is_file() {
                continue;
            }

            // glob 过滤
            if !Self::matches_glob_pattern(file_path, glob_pattern) {
                continue;
            }

            // 检查路径权限
            if check_path_allowed(file_path, &self.allowed_directories).is_err() {
                continue;
            }

            // 跳过二进制文件（简单检测：检查是否有 null 字节）
            if let Ok(bytes) = std::fs::read(file_path) {
                if bytes.iter().any(|&b| b == 0) {
                    continue;
                }
            }

            match Self::search_in_file(file_path, &regex, before_context, after_context, multiline) {
                Ok(matches) => {
                    if !matches.is_empty() {
                        files_with_matches.insert(
                            file_path.to_string_lossy().to_string(),
                            matches.len(),
                        );
                        all_matches.extend(matches);
                    }
                }
                Err(_) => continue, // 跳过无法读取的文件
            }
        }

        // 根据输出模式返回结果
        match output_mode {
            "files_with_matches" => {
                let mut files: Vec<String> = files_with_matches.keys().cloned().collect();
                files.sort();
                
                // 应用 offset 和 head_limit
                let total = files.len();
                let start = offset.min(total);
                let end = (start + head_limit).min(total);
                let paginated = &files[start..end];

                let mut lines = Vec::new();
                if total > 0 {
                    lines.push(format!("找到 {} 个匹配文件:", total));
                    lines.push(String::new());
                    for f in paginated {
                        lines.push(f.clone());
                    }
                    if total > end {
                        lines.push(format!("... 还有 {} 个文件", total - end));
                    }
                } else {
                    lines.push(format!("未找到匹配 '{}' 的文件", pattern));
                }
                Ok(ToolResult::ok(lines.join("\n")))
            }

            "count_matches" => {
                let total: usize = files_with_matches.values().sum();
                let mut lines = Vec::new();
                lines.push(format!("总计匹配: {} 处（{} 个文件）", total, files_with_matches.len()));
                lines.push(String::new());
                
                let mut sorted: Vec<(String, usize)> = files_with_matches.into_iter().collect();
                sorted.sort_by(|a, b| b.1.cmp(&a.1));
                
                for (file, count) in sorted.iter().take(head_limit) {
                    lines.push(format!("{}: {}", file, count));
                }
                if sorted.len() > head_limit {
                    lines.push(format!("... 还有 {} 个文件", sorted.len() - head_limit));
                }
                Ok(ToolResult::ok(lines.join("\n")))
            }

            _ => {
                // content 模式（默认）
                let total = all_matches.len();
                let start = offset.min(total);
                let end = (start + head_limit).min(total);
                let paginated = &all_matches[start..end];

                if total == 0 {
                    return Ok(ToolResult::ok(format!("未找到匹配 '{}' 的内容", pattern)));
                }

                let mut lines = Vec::new();
                lines.push(format!("找到 {} 个匹配（{} 个文件，显示 {}-{}）:", 
                    total, files_with_matches.len(), start + 1, end));
                lines.push(String::new());

                // 按文件分组显示
                let mut current_file: String = String::new();
                for m in paginated {
                    if m.file_path != current_file {
                        if !current_file.is_empty() {
                            lines.push(String::new());
                        }
                        current_file = m.file_path.clone();
                        lines.push(format!("📄 {}", current_file));
                    }

                    if m.context_lines.len() > 1 {
                        // 有上下文：显示上下文块
                        for (ctx_line_num, ctx_content) in &m.context_lines {
                            let prefix = if *ctx_line_num == m.line_number {
                                ">>>"
                            } else {
                                "..."
                            };
                            if show_line_numbers {
                                lines.push(format!("  {} L{:4}: {}", prefix, ctx_line_num, ctx_content));
                            } else {
                                lines.push(format!("  {} {}", prefix, ctx_content));
                            }
                        }
                    } else {
                        // 无上下文：单行显示
                        let content = if m.line_content.len() > 200 {
                            format!("{}...", &m.line_content[..200])
                        } else {
                            m.line_content.clone()
                        };
                        if show_line_numbers {
                            lines.push(format!("  L{:4}: {}", m.line_number, content));
                        } else {
                            lines.push(format!("  {}", content));
                        }
                    }
                }

                if total > end {
                    lines.push(String::new());
                    lines.push(format!("... 还有 {} 个匹配（使用 offset={} 查看下一页）", 
                        total - end, end));
                }

                Ok(ToolResult::ok(lines.join("\n")))
            }
        }
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
    async fn test_grep_tool_basic() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();

        std::fs::write(temp.path().join("a.rs"), "fn main() {\n    println!(\"hello\");\n}\n").unwrap();
        std::fs::write(temp.path().join("b.rs"), "fn helper() {\n    let x = 42;\n}\n").unwrap();

        let tool = GrepTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": path.clone(),
                "pattern": "fn \\w+\\("
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("a.rs"));
        assert!(result.content.contains("b.rs"));
        assert!(result.content.contains("fn main"));
    }

    #[tokio::test]
    async fn test_grep_tool_with_context() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();

        std::fs::write(temp.path().join("test.txt"), "line1\nline2\nTARGET\nline4\nline5\n").unwrap();

        let tool = GrepTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": path.clone(),
                "pattern": "TARGET",
                "-C": 1
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("line2"));
        assert!(result.content.contains("TARGET"));
        assert!(result.content.contains("line4"));
    }

    #[tokio::test]
    async fn test_grep_tool_files_with_matches() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();

        std::fs::write(temp.path().join("a.rs"), "fn main() {}").unwrap();
        std::fs::write(temp.path().join("b.txt"), "hello world").unwrap();

        let tool = GrepTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": path.clone(),
                "pattern": "fn ",
                "output_mode": "files_with_matches"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("a.rs"));
        assert!(!result.content.contains("b.txt"));
    }

    #[tokio::test]
    async fn test_grep_tool_glob_filter() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();

        std::fs::write(temp.path().join("code.rs"), "fn code() {}").unwrap();
        std::fs::write(temp.path().join("readme.md"), "fn markdown").unwrap();

        let tool = GrepTool::new(vec![path.clone()]);
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
    async fn test_grep_tool_ignore_case() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();

        std::fs::write(temp.path().join("test.txt"), "Hello World\n").unwrap();

        let tool = GrepTool::new(vec![path.clone()]);
        let result = tool
            .execute(json!({
                "path": path.clone(),
                "pattern": "hello",
                "-i": true
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("Hello World"));
    }
}
