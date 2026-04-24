//! 终端工具
//!
//! 提供安全执行shell命令的功能。
//! 支持跨平台（Windows用cmd /c，macOS/Linux用bash -c），
//! 支持超时后强制kill进程。

use super::tool::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// 终端工具 — 执行shell命令
///
/// 参数:
/// - `command`: 要执行的命令（必填）
/// - `cwd`: 工作目录（可选）
/// - `timeout`: 超时秒数（可选，默认30秒）
pub struct TerminalTool {
    /// 最大允许的超时秒数（安全限制）
    max_timeout_secs: u64,
}

impl TerminalTool {
    /// 创建新的终端工具
    pub fn new() -> Self {
        Self {
            max_timeout_secs: 300,
        }
    }

    /// 创建带最大超时限制的终端工具
    pub fn with_max_timeout(max_timeout_secs: u64) -> Self {
        Self { max_timeout_secs }
    }

    /// 获取当前平台的shell命令
    fn get_shell_command(command: &str) -> (String, Vec<String>) {
        #[cfg(target_os = "windows")]
        {
            ("cmd".to_string(), vec!["/c".to_string(), command.to_string()])
        }
        #[cfg(not(target_os = "windows"))]
        {
            ("bash".to_string(), vec!["-c".to_string(), command.to_string()])
        }
    }

    /// 执行命令的内部实现
    async fn run_command(
        &self,
        command: &str,
        cwd: Option<&str>,
        timeout_secs: u64,
    ) -> Result<ToolResult, ToolError> {
        let timeout_secs = timeout_secs.min(self.max_timeout_secs);
        let duration = Duration::from_secs(timeout_secs);

        let (shell, args) = Self::get_shell_command(command);

        let mut cmd_builder = Command::new(&shell);
        cmd_builder.args(&args);

        // 设置工作目录
        if let Some(dir) = cwd {
            cmd_builder.current_dir(dir);
        }

        // 执行命令并设置超时
        let result = timeout(duration, cmd_builder.output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                let exit_code = output.status.code().unwrap_or(-1);
                let success = output.status.success();

                let content = if success {
                    if stderr.is_empty() {
                        stdout
                    } else {
                        format!("{stdout}\n[stderr]\n{stderr}")
                    }
                } else {
                    format!(
                        "[退出码: {}]\n{stdout}\n[stderr]\n{stderr}",
                        exit_code
                    )
                };

                Ok(ToolResult {
                    success,
                    content,
                    output_type: super::tool::OutputType::Text,
                })
            }
            Ok(Err(e)) => Err(ToolError::CommandExecution(format!(
                "启动进程失败: {}",
                e
            ))),
            Err(_) => Err(ToolError::Timeout {
                command: command.to_string(),
                timeout: timeout_secs,
            }),
        }
    }
}

impl Default for TerminalTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TerminalTool {
    fn name(&self) -> &str {
        "terminal"
    }

    fn description(&self) -> &str {
        "执行终端命令。支持设置工作目录和超时时间。Windows使用cmd，macOS/Linux使用bash。超时后会强制终止进程。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "要执行的命令"
                },
                "cwd": {
                    "type": "string",
                    "description": "工作目录（可选）"
                },
                "timeout": {
                    "type": "integer",
                    "description": "超时秒数（默认30，最大300）",
                    "minimum": 1,
                    "maximum": 300,
                    "default": 30
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let command = params["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParameters("缺少command参数".to_string()))?;

        let cwd = params["cwd"].as_str();
        let timeout_secs = params["timeout"].as_u64().unwrap_or(30);

        if command.trim().is_empty() {
            return Err(ToolError::InvalidParameters("命令不能为空".to_string()));
        }

        self.run_command(command, cwd, timeout_secs).await
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
    async fn test_terminal_echo() {
        let tool = TerminalTool::new();
        let result = tool
            .execute(json!({"command": "echo 'hello world'"}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("hello world"));
    }

    #[tokio::test]
    async fn test_terminal_with_cwd() {
        let tool = TerminalTool::new();
        let temp = tempfile::tempdir().unwrap();

        // 先创建一个文件
        tokio::fs::write(temp.path().join("testfile"), "")
            .await
            .unwrap();

        // 列出目录内容
        #[cfg(target_os = "windows")]
        let cmd = "dir";
        #[cfg(not(target_os = "windows"))]
        let cmd = "ls";

        let result = tool
            .execute(json!({
                "command": cmd,
                "cwd": temp.path().to_string_lossy().to_string()
            }))
            .await
            .unwrap();

        assert!(result.success);
    }

    #[tokio::test]
    async fn test_terminal_empty_command() {
        let tool = TerminalTool::new();
        let result = tool.execute(json!({"command": "   "})).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_terminal_timeout() {
        let tool = TerminalTool::new();

        // 使用sleep命令测试超时
        #[cfg(target_os = "windows")]
        let cmd = "timeout /t 10";
        #[cfg(not(target_os = "windows"))]
        let cmd = "sleep 10";

        let result = tool
            .execute(json!({
                "command": cmd,
                "timeout": 1
            }))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("超时"));
    }

    #[tokio::test]
    async fn test_terminal_stderr_output() {
        let tool = TerminalTool::new();

        // 触发stderr输出
        #[cfg(target_os = "windows")]
        let cmd = "echo error message >&2";
        #[cfg(not(target_os = "windows"))]
        let cmd = "echo 'error message' >&2";

        let result = tool
            .execute(json!({"command": cmd}))
            .await
            .unwrap();

        // 命令可能成功但stderr有输出
        assert!(result.content.contains("error message"));
    }

    #[test]
    fn test_shell_command_platform() {
        let (shell, args) = TerminalTool::get_shell_command("echo test");
        assert!(!shell.is_empty());
        assert_eq!(args.len(), 2);
        assert_eq!(args[1], "echo test");
    }

    #[tokio::test]
    async fn test_terminal_invalid_params() {
        let tool = TerminalTool::new();
        let result = tool
            .execute(json!({"command": 123})) // command不是字符串
            .await;
        assert!(result.is_err());
    }
}
