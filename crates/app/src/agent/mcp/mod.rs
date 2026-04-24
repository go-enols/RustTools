//! MCP (Model Context Protocol) 兼容层
//!
//! 完整实现MCP协议，支持通过stdio/SSE/WebSocket等传输方式
//! 与MCP服务器通信，实现工具发现、调用和资源管理。

pub mod types;
pub mod transport;
pub mod client;
pub mod server;

// 公共API重导出
pub use types::*;
pub use transport::{McpTransport, StdioTransport, MockTransport};
pub use client::McpClient;
pub use server::{McpManager, McpServerManager, McpServerInstance};
