//! MCP 工具适配器
//!
//! 将 MCP 服务器的工具包装为本地 Tool trait 实现，
//! 使其可以注册到 ToolRegistry 中，被 Executor 统一调用。

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::agent::tools::tool::{Tool, ToolError, ToolResult};
use super::server::McpServerManager;

/// MCP 工具适配器 — 包装单个 MCP 服务器工具
pub struct McpToolAdapter {
    /// 工具名称（MCP 服务器中的原始名称）
    name: String,
    /// 工具描述
    description: String,
    /// 参数 JSON Schema
    parameters: Value,
    /// 所属 MCP 服务器名称
    server_name: String,
    /// MCP 服务器管理器引用
    manager: Arc<McpServerManager>,
}

impl McpToolAdapter {
    /// 创建新的 MCP 工具适配器
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: Value,
        server_name: impl Into<String>,
        manager: Arc<McpServerManager>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
            server_name: server_name.into(),
            manager,
        }
    }

    /// 为指定服务器创建所有工具的适配器
    pub fn create_for_server(
        server_name: &str,
        manager: Arc<McpServerManager>,
    ) -> Vec<Self> {
        let defs = manager.get_server_tools(server_name);

        defs.into_iter()
            .map(|def| Self {
                name: def.function.name.clone(),
                description: def.function.description.clone(),
                parameters: def.function.parameters.clone(),
                server_name: server_name.to_string(),
                manager: Arc::clone(&manager),
            })
            .collect()
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        self.parameters.clone()
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let result = self
            .manager
            .call_tool(&self.server_name, &self.name, params)
            .await
            .map_err(|e| ToolError::Other(format!("MCP 工具调用失败: {}", e.message)))?;

        // 将 MCP 结果转换为 ToolResult，保留所有类型内容
        let mut text_parts: Vec<String> = Vec::new();
        for c in result.content {
            match c {
                super::types::McpToolContent::Text { text } => {
                    text_parts.push(text);
                }
                super::types::McpToolContent::Image { data, mime_type } => {
                    log::warn!("MCP 工具返回了 Image 类型的内容 (mime_type: {})，已转换为描述性文本", mime_type);
                    text_parts.push(format!("[Image: {} 数据 ({} bytes)]", mime_type, data.len()));
                }
                super::types::McpToolContent::Resource { resource } => {
                    log::warn!("MCP 工具返回了 Resource 类型的内容 (uri: {})，已转换为描述性文本", resource.uri);
                    text_parts.push(format!("[Resource: {} (mimeType: {:?})]", resource.uri, resource.mime_type));
                }
            }
        }
        let content = text_parts.join("\n");

        if result.is_error {
            Ok(ToolResult::err(content))
        } else {
            Ok(ToolResult::ok(content))
        }
    }
}
