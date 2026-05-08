//! Git 状态工具
//!
//! 让 AI 能够获取当前 Git 仓库的状态信息，
//! 包括分支、修改文件、提交历史等。

use super::tool::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

/// Git 状态查询工具
pub struct GitStatusTool;

impl GitStatusTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &str {
        "git_status"
    }

    fn description(&self) -> &str {
        "获取当前 Git 仓库的状态信息，包括当前分支、修改的文件、未跟踪文件和最近提交历史。\
         在执行代码修改前或需要了解代码变更历史时调用此工具。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Git 仓库路径，默认为当前工作目录"
                },
                "include_log": {
                    "type": "boolean",
                    "description": "是否包含最近提交历史，默认为 true"
                }
            }
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let include_log = params
            .get("include_log")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let result = gather_git_status(path, include_log)?;
        Ok(ToolResult::ok_json(result))
    }
}

/// 收集 Git 状态信息
fn gather_git_status(path: &str, include_log: bool) -> Result<String, ToolError> {
    let _repo_path = Path::new(path);

    // 检查是否是 git 仓库
    let git_dir_check = std::process::Command::new("git")
        .args(["-C", path, "rev-parse", "--git-dir"])
        .output()
        .map_err(|e| ToolError::CommandExecution(format!("无法执行 git 命令: {}", e)))?;

    if !git_dir_check.status.success() {
        return Ok(serde_json::json!({
            "is_git_repo": false,
            "error": "当前目录不是 Git 仓库"
        }).to_string());
    }

    // 获取当前分支
    let branch_output = std::process::Command::new("git")
        .args(["-C", path, "branch", "--show-current"])
        .output()
        .map_err(|e| ToolError::CommandExecution(format!("获取分支失败: {}", e)))?;
    let branch = String::from_utf8_lossy(&branch_output.stdout).trim().to_string();

    // 获取状态（--porcelain 格式便于解析）
    let status_output = std::process::Command::new("git")
        .args(["-C", path, "status", "--porcelain", "-b"])
        .output()
        .map_err(|e| ToolError::CommandExecution(format!("获取状态失败: {}", e)))?;
    let status_lines: Vec<String> = String::from_utf8_lossy(&status_output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect();

    // 解析状态
    let mut staged: Vec<String> = Vec::new();
    let mut unstaged: Vec<String> = Vec::new();
    let mut untracked: Vec<String> = Vec::new();
    let mut renamed: Vec<String> = Vec::new();

    for line in &status_lines {
        if line.starts_with("## ") {
            continue; // 分支信息行
        }
        if line.len() < 3 {
            continue;
        }
        let status_code = &line[..2];
        let file_path = &line[3..];

        match status_code {
            "??" => untracked.push(file_path.to_string()),
            "M " | "A " | "D " | "R " | "C " => staged.push(format!("{} ({})", file_path, status_code.trim())),
            " M" | " D" | "MM" | "AM" | "DM" => unstaged.push(format!("{} ({})", file_path, status_code.trim())),
            "R " => {
                // 重命名格式: R  old -> new
                let parts: Vec<&str> = file_path.split(" -> ").collect();
                if parts.len() == 2 {
                    renamed.push(format!("{} -> {}", parts[0], parts[1]));
                } else {
                    renamed.push(file_path.to_string());
                }
            }
            _ => {}
        }
    }

    // 获取最近提交历史
    let mut log: Vec<Value> = Vec::new();
    if include_log {
        let log_output = std::process::Command::new("git")
            .args(["-C", path, "log", "--oneline", "-n", "10", "--no-decorate"])
            .output()
            .map_err(|e| ToolError::CommandExecution(format!("获取日志失败: {}", e)))?;

        for line in String::from_utf8_lossy(&log_output.stdout).lines() {
            if let Some((hash, msg)) = line.split_once(' ') {
                log.push(serde_json::json!({
                    "hash": hash,
                    "message": msg
                }));
            }
        }
    }

    let result = serde_json::json!({
        "is_git_repo": true,
        "branch": branch,
        "ahead_behind": extract_ahead_behind(&status_lines),
        "staged": staged,
        "unstaged": unstaged,
        "untracked": untracked,
        "renamed": renamed,
        "has_changes": !staged.is_empty() || !unstaged.is_empty() || !untracked.is_empty(),
        "recent_commits": log,
    });

    serde_json::to_string_pretty(&result)
        .map_err(|e| ToolError::JsonParse(e.to_string()))
}

/// 从状态行中提取 ahead/behind 信息
fn extract_ahead_behind(lines: &[String]) -> Value {
    for line in lines {
        if let Some(branch_info) = line.strip_prefix("## ") {
            // 格式如: main...origin/main [ahead 2, behind 1]
            if let Some(start) = branch_info.find('[') {
                if let Some(end) = branch_info.find(']') {
                    let info = &branch_info[start + 1..end];
                    let mut ahead = 0i32;
                    let mut behind = 0i32;

                    for part in info.split(',') {
                        let part = part.trim();
                        if part.starts_with("ahead ") {
                            ahead = part[6..].parse().unwrap_or(0);
                        } else if part.starts_with("behind ") {
                            behind = part[7..].parse().unwrap_or(0);
                        }
                    }

                    return serde_json::json!({
                        "ahead": ahead,
                        "behind": behind,
                        "synced": ahead == 0 && behind == 0
                    });
                }
            }
            return serde_json::json!({
                "ahead": 0,
                "behind": 0,
                "synced": true
            });
        }
    }
    serde_json::json!({"ahead": 0, "behind": 0, "synced": true})
}
