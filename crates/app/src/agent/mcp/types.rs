//! MCP协议类型定义 — JSON-RPC 2.0 和 MCP 协议结构

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================
// JSON-RPC 2.0 基础类型
// ============================================================

/// JSON-RPC 2.0 请求
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub method: String,
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// 创建一个新的JSON-RPC请求
    pub fn new(id: u64, method: &str, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            method: method.to_string(),
            params,
        }
    }

    /// 创建通知 (无id)
    pub fn notification(method: &str, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: method.to_string(),
            params,
        }
    }
}

/// JSON-RPC 2.0 响应
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// 创建成功响应
    pub fn success(id: u64, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            result: Some(result),
            error: None,
        }
    }

    /// 创建错误响应
    pub fn error(id: Option<u64>, code: i32, message: &str, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data,
            }),
        }
    }

    /// 检查是否为成功响应
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }
}

/// JSON-RPC 2.0 错误
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// JSON-RPC 2.0 通知 (服务端推送/客户端通知)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    pub fn new(method: &str, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        }
    }
}

// ============================================================
// MCP 协议类型
// ============================================================

/// MCP初始化请求参数
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpInitializeRequest {
    pub protocol_version: String,
    pub capabilities: McpClientCapabilities,
    pub client_info: McpImplementationInfo,
}

impl Default for McpInitializeRequest {
    fn default() -> Self {
        Self {
            protocol_version: "2024-11-05".to_string(),
            capabilities: McpClientCapabilities::default(),
            client_info: McpImplementationInfo {
                name: "rusttools-agent".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }
}

/// MCP初始化结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpInitializeResult {
    pub protocol_version: String,
    pub capabilities: McpServerCapabilities,
    pub server_info: McpImplementationInfo,
}

/// MCP实现信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpImplementationInfo {
    pub name: String,
    pub version: String,
}

/// MCP客户端能力
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct McpClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<Value>,
}

/// MCP服务端能力
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct McpServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,
}

/// MCP工具定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// MCP资源定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// 调用工具请求
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpCallToolRequest {
    pub name: String,
    pub arguments: Value,
}

/// 调用工具结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpCallToolResult {
    pub content: Vec<McpToolContent>,
    #[serde(default)]
    pub is_error: bool,
}

/// 工具输出内容类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum McpToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { resource: McpResource },
}

/// 工具列表响应
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpListToolsResult {
    pub tools: Vec<McpTool>,
}

/// 资源列表响应
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpListResourcesResult {
    pub resources: Vec<McpResource>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_rpc_request_serialization() {
        let req = JsonRpcRequest::new(1, "initialize", Some(json!({"version": "1.0"})));
        let json_str = serde_json::to_string(&req).unwrap();
        assert!(json_str.contains("\"jsonrpc\":\"2.0\""));
        assert!(json_str.contains("\"id\":1"));
        assert!(json_str.contains("initialize"));

        let deserialized: JsonRpcRequest = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized, req);
    }

    #[test]
    fn test_json_rpc_notification_serialization() {
        let notif = JsonRpcNotification::new("notifications/progress", Some(json!({"progress": 50})));
        let json_str = serde_json::to_string(&notif).unwrap();
        assert!(!json_str.contains("\"id\"")); // 通知没有id
        assert!(json_str.contains("notifications/progress"));

        let deserialized: JsonRpcNotification = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized, notif);
    }

    #[test]
    fn test_json_rpc_response_success_and_error() {
        let success = JsonRpcResponse::success(1, json!({"status": "ok"}));
        assert!(success.is_success());

        let error = JsonRpcResponse::error(Some(2), -32600, "Invalid Request", None);
        assert!(!error.is_success());
        assert_eq!(error.error.as_ref().unwrap().code, -32600);

        // 序列化/反序列化
        let s = serde_json::to_string(&success).unwrap();
        let d: JsonRpcResponse = serde_json::from_str(&s).unwrap();
        assert_eq!(d.id, Some(1));
    }

    #[test]
    fn test_mcp_initialize_request_default() {
        let req = McpInitializeRequest::default();
        assert_eq!(req.protocol_version, "2024-11-05");
        assert_eq!(req.client_info.name, "rusttools-agent");
    }

    #[test]
    fn test_mcp_tool_serialization() {
        let tool = McpTool {
            name: "read_file".to_string(),
            description: "读取文件内容".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                }
            }),
        };
        let json_str = serde_json::to_string(&tool).unwrap();
        let deserialized: McpTool = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.name, "read_file");
        assert_eq!(deserialized.description, "读取文件内容");
    }

    #[test]
    fn test_mcp_call_tool_result_serialization() {
        let result = McpCallToolResult {
            content: vec![
                McpToolContent::Text {
                    text: "Hello World".to_string(),
                },
                McpToolContent::Image {
                    data: "base64data".to_string(),
                    mime_type: "image/png".to_string(),
                },
            ],
            is_error: false,
        };
        let json_str = serde_json::to_string(&result).unwrap();
        let deserialized: McpCallToolResult = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.content.len(), 2);
        assert!(matches!(deserialized.content[0], McpToolContent::Text { .. }));
        assert!(matches!(deserialized.content[1], McpToolContent::Image { .. }));
    }

    #[test]
    fn test_mcp_tool_content_resource() {
        let content = McpToolContent::Resource {
            resource: McpResource {
                uri: "file:///test.txt".to_string(),
                name: "test.txt".to_string(),
                mime_type: Some("text/plain".to_string()),
            },
        };
        let json_str = serde_json::to_string(&content).unwrap();
        assert!(json_str.contains("resource"));
        let deserialized: McpToolContent = serde_json::from_str(&json_str).unwrap();
        assert!(matches!(deserialized, McpToolContent::Resource { .. }));
    }

    #[test]
    fn test_mcp_list_tools_result() {
        let result = McpListToolsResult {
            tools: vec![McpTool {
                name: "tool1".to_string(),
                description: "desc1".to_string(),
                input_schema: json!({}),
            }],
        };
        let s = serde_json::to_string(&result).unwrap();
        let d: McpListToolsResult = serde_json::from_str(&s).unwrap();
        assert_eq!(d.tools.len(), 1);
    }
}
