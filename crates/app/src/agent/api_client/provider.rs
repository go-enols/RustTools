use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Provider类型（与config中的ProviderType对应，但独立以保持模块边界）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Gemini,
    Ollama,
    OpenAICompatible,
}

impl ProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderType::OpenAI => "openai",
            ProviderType::Anthropic => "anthropic",
            ProviderType::Gemini => "gemini",
            ProviderType::Ollama => "ollama",
            ProviderType::OpenAICompatible => "openai_compatible",
        }
    }
}

/// LLM Provider Trait —— 所有Provider必须实现的统一接口
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Provider名称
    fn name(&self) -> &str;

    /// Provider类型
    fn provider_type(&self) -> ProviderType;

    /// 非流式聊天请求
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ApiError>;

    /// 流式聊天请求
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamChunk, ApiError>>, ApiError>;

    /// 列出可用模型
    async fn list_models(&self) -> Result<Vec<ModelInfo>, ApiError>;

    /// 是否支持工具调用
    fn supports_tools(&self) -> bool;

    /// 是否支持视觉输入
    fn supports_vision(&self) -> bool;

    /// 最大上下文长度
    fn max_context_length(&self) -> usize;
}

/// 聊天请求统一格式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub stream: bool,
}

impl Default for ChatRequest {
    fn default() -> Self {
        Self {
            model: String::new(),
            messages: vec![],
            temperature: Some(0.7),
            max_tokens: None,
            tools: None,
            stream: false,
        }
    }
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum ChatMessage {
    System { content: String },
    User { content: MessageContent },
    Assistant {
        content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}

/// 消息内容（支持文本或多模态）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    MultiPart(Vec<Part>),
}

impl MessageContent {
    /// 获取文本内容（如果是纯文本）
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(s) => Some(s),
            MessageContent::MultiPart(_) => None,
        }
    }
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        MessageContent::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        MessageContent::Text(s.to_string())
    }
}

/// 消息多模态部分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    #[serde(rename = "type")]
    pub part_type: String,
    pub content: String,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// 函数调用详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// 工具定义（用于描述可用工具）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

/// 函数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// 聊天响应
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub usage: Option<TokenUsage>,
    pub finish_reason: Option<String>,
}

/// Token使用量
#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// 流式响应块
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// 内容增量
    Content { delta: String },
    /// 工具调用增量
    ToolCall { tool_call: ToolCall },
    /// 响应完成
    Done,
    /// 错误
    Error { message: String },
}

/// 模型信息
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: ProviderType,
    pub context_length: Option<usize>,
    pub supports_tools: bool,
    pub supports_vision: bool,
}

/// API错误类型
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("HTTP请求错误: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON解析错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("API返回错误: {status_code} - {message}")]
    ApiError { status_code: u16, message: String },
    #[error("配置错误: {0}")]
    Config(String),
    #[error("Provider未找到: {0}")]
    ProviderNotFound(String),
    #[error("模型未找到: {0}")]
    ModelNotFound(String),
    #[error("不支持的特性: {0}")]
    UnsupportedFeature(String),
    #[error("流式解析错误: {0}")]
    StreamParse(String),
    #[error("通用错误: {0}")]
    General(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试ChatMessage序列化
    #[test]
    fn test_chat_message_serialization() {
        let system_msg = ChatMessage::System {
            content: "你是助手".to_string(),
        };
        let json = serde_json::to_string(&system_msg).expect("序列化失败");
        assert!(json.contains("system"));
        assert!(json.contains("你是助手"));

        let user_msg = ChatMessage::User {
            content: MessageContent::Text("你好".to_string()),
        };
        let json = serde_json::to_string(&user_msg).expect("序列化失败");
        assert!(json.contains("user"));
        assert!(json.contains("你好"));

        let assistant_msg = ChatMessage::Assistant {
            content: Some("你好！".to_string()),
            tool_calls: None,
        };
        let json = serde_json::to_string(&assistant_msg).expect("序列化失败");
        assert!(json.contains("assistant"));
    }

    /// 测试MessageContent转换
    #[test]
    fn test_message_content_conversions() {
        let text: MessageContent = "hello".into();
        assert_eq!(text.as_text(), Some("hello"));

        let text2: MessageContent = "world".to_string().into();
        assert_eq!(text2.as_text(), Some("world"));

        let multi = MessageContent::MultiPart(vec![Part {
            part_type: "text".to_string(),
            content: "hello".to_string(),
        }]);
        assert_eq!(multi.as_text(), None);
    }

    /// 测试ToolDefinition序列化
    #[test]
    fn test_tool_definition_serialization() {
        let tool = ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "fs_read".to_string(),
                description: "读取文件".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    }
                }),
            },
        };
        let json = serde_json::to_string(&tool).expect("序列化失败");
        assert!(json.contains("fs_read"));
        assert!(json.contains("读取文件"));
    }

    /// 测试ProviderType as_str
    #[test]
    fn test_provider_type_str() {
        assert_eq!(ProviderType::OpenAI.as_str(), "openai");
        assert_eq!(ProviderType::Anthropic.as_str(), "anthropic");
        assert_eq!(ProviderType::Gemini.as_str(), "gemini");
        assert_eq!(ProviderType::Ollama.as_str(), "ollama");
        assert_eq!(ProviderType::OpenAICompatible.as_str(), "openai_compatible");
    }

    /// 测试ChatRequest默认值
    #[test]
    fn test_chat_request_default() {
        let req = ChatRequest::default();
        assert!(req.model.is_empty());
        assert!(req.messages.is_empty());
        assert_eq!(req.temperature, Some(0.7));
        assert_eq!(req.max_tokens, None);
        assert_eq!(req.tools, None);
        assert!(!req.stream);
    }

    /// 测试TokenUsage
    #[test]
    fn test_token_usage() {
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }

    /// 测试StreamChunk变体
    #[test]
    fn test_stream_chunk_variants() {
        let content = StreamChunk::Content {
            delta: "hello".to_string(),
        };
        match content {
            StreamChunk::Content { delta } => assert_eq!(delta, "hello"),
            _ => panic!("不匹配"),
        }

        let done = StreamChunk::Done;
        match done {
            StreamChunk::Done => {}
            _ => panic!("不匹配"),
        }

        let error = StreamChunk::Error {
            message: "test error".to_string(),
        };
        match error {
            StreamChunk::Error { message } => assert_eq!(message, "test error"),
            _ => panic!("不匹配"),
        }
    }
}
