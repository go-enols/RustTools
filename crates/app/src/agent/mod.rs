//! Agent 模块 — AI Agent IDE 的后端核心
//!
//! 包含以下子模块：
//! - `config`: 配置管理（模型、Agent、MCP服务器配置）
//! - `api_client`: 模型API统一封装（OpenAI/Anthropic/Gemini/Ollama）
//! - `tools`: AI可用工具封装（文件系统、终端、代码编辑、搜索）
//! - `mcp`: MCP (Model Context Protocol) 兼容层
//! - `long_task`: 长任务稳定性（检查点、恢复、任务队列）
//! - `agent_core`: Agent编排引擎

pub mod agent_core;
pub mod api_client;
pub mod config;
pub mod long_task;
pub mod mcp;
pub mod tools;

// ============================================================
// 公共错误类型（本模块层定义）
// ============================================================

/// MCP协议错误
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct McpError {
    pub code: String,
    pub message: String,
}

impl McpError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }

    pub fn transport(msg: &str) -> Self {
        Self::new("TRANSPORT_ERROR", msg)
    }

    pub fn protocol(msg: &str) -> Self {
        Self::new("PROTOCOL_ERROR", msg)
    }

    pub fn not_initialized() -> Self {
        Self::new("NOT_INITIALIZED", "MCP客户端尚未初始化")
    }

    pub fn server_not_found(name: &str) -> Self {
        Self::new(
            "SERVER_NOT_FOUND",
            &format!("未找到MCP服务器: {}", name),
        )
    }
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for McpError {}

/// 检查点错误
#[derive(Debug, Clone, PartialEq)]
pub enum CheckpointError {
    Io(String),
    Serialization(String),
    NotFound(String),
}

impl std::fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckpointError::Io(msg) => write!(f, "Checkpoint IO错误: {}", msg),
            CheckpointError::Serialization(msg) => {
                write!(f, "Checkpoint序列化错误: {}", msg)
            }
            CheckpointError::NotFound(msg) => write!(f, "Checkpoint未找到: {}", msg),
        }
    }
}

impl std::error::Error for CheckpointError {}

impl From<std::io::Error> for CheckpointError {
    fn from(err: std::io::Error) -> Self {
        CheckpointError::Io(err.to_string())
    }
}

/// 恢复错误
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryError {
    MaxRetriesExceeded { task_id: String, retries: u32 },
    NoCheckpoint(String),
    Execution(String),
    Checkpoint(CheckpointError),
}

impl std::fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryError::MaxRetriesExceeded { task_id, retries } => {
                write!(f, "任务 {} 超过最大重试次数 ({})", task_id, retries)
            }
            RecoveryError::NoCheckpoint(id) => write!(f, "无可用检查点: {}", id),
            RecoveryError::Execution(msg) => write!(f, "执行错误: {}", msg),
            RecoveryError::Checkpoint(e) => write!(f, "检查点错误: {}", e),
        }
    }
}

impl std::error::Error for RecoveryError {}

impl From<CheckpointError> for RecoveryError {
    fn from(err: CheckpointError) -> Self {
        RecoveryError::Checkpoint(err)
    }
}

// ============================================================
// MCP 服务器信息类型
// ============================================================

/// MCP服务器状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ServerStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl std::fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerStatus::Disconnected => write!(f, "未连接"),
            ServerStatus::Connecting => write!(f, "连接中"),
            ServerStatus::Connected => write!(f, "已连接"),
            ServerStatus::Error(e) => write!(f, "错误: {}", e),
        }
    }
}

/// MCP服务器信息摘要
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct McpServerInfo {
    pub name: String,
    pub transport: String,
    pub command: String,
    pub status: ServerStatus,
    pub tool_count: usize,
    pub resource_count: usize,
    pub last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_error_display() {
        let err = McpError::transport("连接超时");
        assert_eq!(err.code, "TRANSPORT_ERROR");
        assert!(err.to_string().contains("连接超时"));
    }

    #[test]
    fn test_server_status_display() {
        assert_eq!(ServerStatus::Connected.to_string(), "已连接");
        assert_eq!(
            ServerStatus::Error("fail".to_string()).to_string(),
            "错误: fail"
        );
    }

    #[test]
    fn test_checkpoint_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let cp_err: CheckpointError = io_err.into();
        assert!(matches!(cp_err, CheckpointError::Io(_)));
    }
}
