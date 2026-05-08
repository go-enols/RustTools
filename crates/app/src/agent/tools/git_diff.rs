//! Git Diff 工具
//!
//! 让 AI 能够查看当前 Git 仓库的改动差异。

use super::tool::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

/// Git Diff 查询工具
pub struct GitDiffTool;

impl GitDiffTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &str {
        "git_diff"
    }

    fn description(&self) -> &str {
        "获取当前 Git 仓库的代码改动差异（diff）。\
         在需要查看具体修改内容、进行代码审查或确认变更范围时调用此工具。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Git 仓库路径，默认为当前工作目录"
                },
                "staged": {
                    "type": "boolean",
                    "description": "是否只查看已暂存(staged)的改动，默认为 false（查看所有未暂存改动）"
                },
                "file": {
                    "type": "string",
                    "description": "指定文件路径查看该文件的 diff，不指定则查看全部"
                }
            }
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let path = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let staged = params.get("staged").and_then(|v| v.as_bool()).unwrap_or(false);
        let file = params.get("file").and_then(|v| v.as_str());

        let mut args = vec!["-C", path, "diff"];
        if staged {
            args.push("--staged");
        }
        if let Some(f) = file {
            args.push("--");
            args.push(f);
        }

        let output = std::process::Command::new("git")
            .args(&args)
            .output()
            .map_err(|e| ToolError::CommandExecution(format!("执行 git diff 失败: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() && !stderr.is_empty() {
            return Ok(ToolResult::err(format!("git diff 错误: {}", stderr)));
        }

        let result = serde_json::json!({
            "has_changes": !stdout.trim().is_empty(),
            "diff": stdout.to_string(),
            "staged": staged,
            "file": file,
        });

        let json_str = serde_json::to_string_pretty(&result)
            .map_err(|e| ToolError::JsonParse(e.to_string()))?;

        Ok(ToolResult::ok_json(json_str))
    }
}
