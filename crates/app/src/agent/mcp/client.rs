//! MCP客户端 — 实现MCP协议的核心交互逻辑
//!
//! 负责与服务器的握手、工具发现、工具调用和资源管理。

use serde_json::Value;

use crate::agent::tools::tool::{FunctionDefinition, ToolDefinition};
use crate::agent::McpError;
use super::transport::McpTransport;
use super::types::*;

/// MCP客户端 — 与单个MCP服务器通信
///
/// 生命周期:
/// 1. `connect()` — 建立传输连接
/// 2. `initialize()` — 协议握手
/// 3. `list_tools()` / `call_tool()` / `list_resources()` — 正常使用
/// 4. `disconnect()` — 优雅关闭
pub struct McpClient {
    pub server_name: String,
    transport: Box<dyn McpTransport>,
    pub tools: Vec<McpTool>,
    pub resources: Vec<McpResource>,
    initialized: bool,
}

impl std::fmt::Debug for McpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpClient")
            .field("server_name", &self.server_name)
            .field("tools", &self.tools)
            .field("resources", &self.resources)
            .field("initialized", &self.initialized)
            .finish_non_exhaustive()
    }
}

impl McpClient {
    /// 使用给定传输创建客户端，此时还未初始化
    pub async fn connect(transport: Box<dyn McpTransport>, server_name: String) -> Result<Self, McpError> {
        if !transport.is_connected() {
            return Err(McpError::transport("传输层未连接"));
        }

        Ok(Self {
            server_name,
            transport,
            tools: vec![],
            resources: vec![],
            initialized: false,
        })
    }

    /// 执行MCP协议初始化握手
    ///
    /// 发送 `initialize` 请求，然后发送 `notifications/initialized` 通知。
    pub async fn initialize(&mut self) -> Result<McpInitializeResult, McpError> {
        let init_req = McpInitializeRequest::default();
        let params = serde_json::to_value(&init_req)
            .map_err(|e| McpError::protocol(&format!("序列化初始化请求失败: {}", e)))?;

        let request = JsonRpcRequest::new(1, "initialize", Some(params));
        let response = self.transport.send(request).await?;

        if !response.is_success() {
            let err = response.error.unwrap_or_else(|| JsonRpcError {
                code: -1,
                message: "未知错误".to_string(),
                data: None,
            });
            return Err(McpError::protocol(&format!(
                "初始化失败 [{}]: {}",
                err.code, err.message
            )));
        }

        let result: McpInitializeResult = serde_json::from_value(response.result.unwrap())
            .map_err(|e| McpError::protocol(&format!("解析初始化结果失败: {}", e)))?;

        // 发送初始化完成通知
        let notif = JsonRpcNotification::new("notifications/initialized", None);
        self.transport.notify(notif).await?;

        self.initialized = true;
        Ok(result)
    }

    /// 获取服务器工具列表
    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>, McpError> {
        self.ensure_initialized()?;

        let request = JsonRpcRequest::new(2, "tools/list", None);
        let response = self.transport.send(request).await?;

        if !response.is_success() {
            return Err(McpError::protocol("获取工具列表失败"));
        }

        let result: McpListToolsResult = serde_json::from_value(response.result.unwrap())
            .map_err(|e| McpError::protocol(&format!("解析工具列表失败: {}", e)))?;

        self.tools = result.tools.clone();
        Ok(result.tools)
    }

    /// 调用指定工具
    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: Value,
    ) -> Result<McpCallToolResult, McpError> {
        self.ensure_initialized()?;

        let call_req = McpCallToolRequest {
            name: name.to_string(),
            arguments,
        };
        let params = serde_json::to_value(&call_req)
            .map_err(|e| McpError::protocol(&format!("序列化工具调用请求失败: {}", e)))?;

        let request = JsonRpcRequest::new(3, "tools/call", Some(params));
        let response = self.transport.send(request).await?;

        if !response.is_success() {
            let err = response.error.unwrap_or_else(|| JsonRpcError {
                code: -1,
                message: "工具调用失败".to_string(),
                data: None,
            });
            return Err(McpError::protocol(&format!(
                "工具 {} 调用失败 [{}]: {}",
                name, err.code, err.message
            )));
        }

        let result: McpCallToolResult = serde_json::from_value(response.result.unwrap())
            .map_err(|e| McpError::protocol(&format!("解析工具调用结果失败: {}", e)))?;

        Ok(result)
    }

    /// 获取服务器资源列表
    pub async fn list_resources(&mut self) -> Result<Vec<McpResource>, McpError> {
        self.ensure_initialized()?;

        let request = JsonRpcRequest::new(4, "resources/list", None);
        let response = self.transport.send(request).await?;

        if !response.is_success() {
            return Err(McpError::protocol("获取资源列表失败"));
        }

        let result: McpListResourcesResult = serde_json::from_value(response.result.unwrap())
            .map_err(|e| McpError::protocol(&format!("解析资源列表失败: {}", e)))?;

        self.resources = result.resources.clone();
        Ok(result.resources)
    }

    /// 将MCP工具转换为LLM可用的ToolDefinition格式
    pub fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .iter()
            .map(|tool| ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    parameters: tool.input_schema.clone(),
                },
            })
            .collect()
    }

    /// 断开与服务器的连接
    pub async fn disconnect(&mut self) -> Result<(), McpError> {
        self.initialized = false;
        self.tools.clear();
        self.resources.clear();
        self.transport.close().await
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// 获取连接状态
    pub fn is_connected(&self) -> bool {
        self.transport.is_connected()
    }

    /// 确保已初始化，否则返回错误
    fn ensure_initialized(&self) -> Result<(), McpError> {
        if self.initialized {
            Ok(())
        } else {
            Err(McpError::not_initialized())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::transport::MockTransport;
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_mock_transport() -> MockTransport {
        let mut responses = HashMap::new();
        responses.insert(
            "initialize".to_string(),
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "serverInfo": {"name": "test-server", "version": "1.0"}
            }),
        );
        responses.insert(
            "tools/list".to_string(),
            json!({
                "tools": [
                    {
                        "name": "read_file",
                        "description": "读取文件",
                        "inputSchema": {
                            "type": "object",
                            "properties": {"path": {"type": "string"}}
                        }
                    }
                ]
            }),
        );
        responses.insert(
            "tools/call".to_string(),
            json!({
                "content": [{"type": "text", "text": "file content"}],
                "isError": false
            }),
        );
        responses.insert(
            "resources/list".to_string(),
            json!({
                "resources": [
                    {"uri": "file:///test.txt", "name": "test.txt", "mimeType": "text/plain"}
                ]
            }),
        );
        MockTransport::new(responses)
    }

    #[tokio::test]
    async fn test_mcp_client_full_lifecycle() {
        let mock = create_mock_transport();
        let mut client = McpClient::connect(Box::new(mock), "test-server".to_string())
            .await
            .unwrap();

        assert!(!client.is_initialized());

        // 初始化
        let init_result = client.initialize().await.unwrap();
        assert_eq!(init_result.protocol_version, "2024-11-05");
        assert!(client.is_initialized());

        // 获取工具
        let tools = client.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "read_file");
        assert_eq!(client.tools.len(), 1);

        // 转换为ToolDefinition
        let defs = client.get_tool_definitions();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].function.name, "read_file");

        // 调用工具
        let result = client.call_tool("read_file", json!({"path": "/test"})).await.unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);

        // 获取资源
        let resources = client.list_resources().await.unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].uri, "file:///test.txt");

        // 断开
        client.disconnect().await.unwrap();
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_not_initialized_error() {
        let mock = create_mock_transport();
        let mut client = McpClient::connect(Box::new(mock), "test".to_string())
            .await
            .unwrap();

        let result = client.list_tools().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "NOT_INITIALIZED");
    }

    #[tokio::test]
    async fn test_transport_not_connected() {
        let mut mock = MockTransport::new(HashMap::new());
        mock.close().await.unwrap();
        let result = McpClient::connect(Box::new(mock), "test".to_string()).await;
        assert!(result.is_err());
    }
}
