use super::provider::*;
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::stream::{BoxStream, StreamExt};
use futures::future;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// OpenAI格式Provider —— 支持OpenAI、Azure、DeepSeek、Moonshot、通义千问等
pub struct OpenAIProvider {
    name: String,
    client: Client,
    api_key: Option<String>,
    base_url: String,
    default_model: String,
    supports_tools: bool,
    supports_vision: bool,
    max_context_length: usize,
    provider_type: ProviderType,
}

impl OpenAIProvider {
    /// 创建新的OpenAI格式Provider
    pub fn new(
        name: impl Into<String>,
        api_key: Option<String>,
        base_url: Option<String>,
        default_model: impl Into<String>,
    ) -> Self {
        let base_url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let base_url = base_url.trim_end_matches('/').to_string();

        Self {
            name: name.into(),
            client: Client::new(),
            api_key,
            base_url,
            default_model: default_model.into(),
            supports_tools: true,
            supports_vision: true,
            max_context_length: 128000,
            provider_type: ProviderType::OpenAI,
        }
    }

    /// 设置Provider类型（用于区分OpenAI兼容的不同服务）
    pub fn with_provider_type(mut self, provider_type: ProviderType) -> Self {
        self.provider_type = provider_type;
        self
    }

    /// 设置是否支持工具调用
    pub fn with_tools_support(mut self, supports: bool) -> Self {
        self.supports_tools = supports;
        self
    }

    /// 设置是否支持视觉输入
    pub fn with_vision_support(mut self, supports: bool) -> Self {
        self.supports_vision = supports;
        self
    }

    /// 设置最大上下文长度
    pub fn with_max_context_length(mut self, length: usize) -> Self {
        self.max_context_length = length;
        self
    }

    /// 构建请求URL
    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        if let Some(ref key) = self.api_key {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", key).parse().unwrap(),
            );
        }
        headers
    }

    /// 转换统一ChatMessage为OpenAI格式消息
    fn convert_messages(&self, messages: &[ChatMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|msg| match msg {
                ChatMessage::System { content } => json!({
                    "role": "system",
                    "content": content
                }),
                ChatMessage::User { content } => match content {
                    MessageContent::Text(text) => json!({
                        "role": "user",
                        "content": text
                    }),
                    MessageContent::MultiPart(parts) => {
                        let content_parts: Vec<serde_json::Value> = parts
                            .iter()
                            .map(|part| match part.part_type.as_str() {
                                "text" => json!({"type": "text", "text": part.content}),
                                "image_url" => {
                                    json!({"type": "image_url", "image_url": {"url": part.content}})
                                }
                                _ => json!({"type": "text", "text": part.content}),
                            })
                            .collect();
                        json!({"role": "user", "content": content_parts})
                    }
                },
                ChatMessage::Assistant {
                    content,
                    tool_calls,
                } => {
                    let mut msg = json!({"role": "assistant"});
                    if let Some(ref c) = content {
                        msg["content"] = json!(c);
                    } else {
                        msg["content"] = json!(null);
                    }
                    if let Some(ref calls) = tool_calls {
                        msg["tool_calls"] = json!(calls);
                    }
                    msg
                }
                ChatMessage::Tool {
                    tool_call_id,
                    content,
                } => json!({
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "content": content
                }),
            })
            .collect()
    }

    /// 转换工具定义为OpenAI格式
    fn convert_tools(&self, tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
        tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.function.name,
                        "description": tool.function.description,
                        "parameters": tool.function.parameters
                    }
                })
            })
            .collect()
    }

    /// 解析OpenAI格式响应
    fn parse_response(&self, body: &str) -> Result<ChatResponse, ApiError> {
        let resp: OpenAIChatResponse = serde_json::from_str(body).map_err(ApiError::Json)?;

        if let Some(error) = resp.error {
            return Err(ApiError::ApiError {
                status_code: 400,
                message: error.message,
            });
        }

        let choice = resp.choices.first().ok_or_else(|| {
            ApiError::General("响应中没有choices".to_string())
        })?;

        let tool_calls = if let Some(ref calls) = choice.message.tool_calls {
            Some(
                calls
                    .iter()
                    .map(|call| ToolCall {
                        id: call.id.clone(),
                        call_type: "function".to_string(),
                        function: FunctionCall {
                            name: call.function.name.clone(),
                            arguments: call.function.arguments.clone(),
                        },
                    })
                    .collect(),
            )
        } else {
            None
        };

        Ok(ChatResponse {
            id: resp.id,
            model: resp.model,
            content: choice.message.content.clone().unwrap_or_default(),
            tool_calls,
            usage: resp.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: choice.finish_reason.clone(),
        })
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn provider_type(&self) -> ProviderType {
        self.provider_type
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ApiError> {
        let url = self.chat_url();
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };

        let mut payload = json!({
            "model": model,
            "messages": self.convert_messages(&request.messages),
            "stream": false,
        });

        if let Some(temp) = request.temperature {
            payload["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            payload["max_tokens"] = json!(max_tokens);
        }
        if let Some(ref tools) = request.tools {
            if self.supports_tools {
                payload["tools"] = json!(self.convert_tools(tools));
            }
        }

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&payload)
            .send()
            .await
            .map_err(ApiError::Http)?;

        let status = response.status();
        let body = response.text().await.map_err(ApiError::Http)?;

        if !status.is_success() {
            return Err(ApiError::ApiError {
                status_code: status.as_u16(),
                message: body,
            });
        }

        self.parse_response(&body)
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamChunk, ApiError>>, ApiError> {
        let url = self.chat_url();
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };

        let mut payload = json!({
            "model": model,
            "messages": self.convert_messages(&request.messages),
            "stream": true,
        });

        if let Some(temp) = request.temperature {
            payload["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            payload["max_tokens"] = json!(max_tokens);
        }
        if let Some(ref tools) = request.tools {
            if self.supports_tools {
                payload["tools"] = json!(self.convert_tools(tools));
            }
        }

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&payload)
            .send()
            .await
            .map_err(ApiError::Http)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.map_err(ApiError::Http)?;
            return Err(ApiError::ApiError {
                status_code: status.as_u16(),
                message: body,
            });
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| {
                event.map_err(|e| ApiError::StreamParse(e.to_string())).and_then(|event| {
                    if event.data == "[DONE]" {
                        return Ok(StreamChunk::Done);
                    }

                    let chunk: OpenAIStreamChunk = serde_json::from_str(&event.data)
                        .map_err(|e| ApiError::StreamParse(format!("解析SSE失败: {}", e)))?;

                    if let Some(choice) = chunk.choices.first() {
                        if let Some(ref delta) = choice.delta.content {
                            return Ok(StreamChunk::Content { delta: delta.clone() });
                        }
                        if let Some(ref calls) = choice.delta.tool_calls {
                            if let Some(call) = calls.first() {
                                return Ok(StreamChunk::ToolCall {
                                    tool_call: ToolCall {
                                        id: call.id.clone().unwrap_or_default(),
                                        call_type: "function".to_string(),
                                        function: FunctionCall {
                                            name: call
                                                .function
                                                .as_ref()
                                                .and_then(|f| f.name.clone())
                                                .unwrap_or_default(),
                                            arguments: call
                                                .function
                                                .as_ref()
                                                .map(|f| f.arguments.clone().unwrap_or_default())
                                                .unwrap_or_default(),
                                        },
                                    },
                                });
                            }
                        }
                        if choice.finish_reason.is_some() {
                            return Ok(StreamChunk::Done);
                        }
                    }

                    Ok(StreamChunk::Content {
                        delta: String::new(),
                    })
                })
            })
            .filter(|item| {
                // 过滤掉空内容块
                future::ready(
                    if let Ok(StreamChunk::Content { delta }) = item {
                        !delta.is_empty()
                    } else {
                        true
                    }
                )
            });

        Ok(Box::pin(stream))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ApiError> {
        let url = format!("{}/models", self.base_url);
        let response = self
            .client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await
            .map_err(ApiError::Http)?;

        let status = response.status();
        let body = response.text().await.map_err(ApiError::Http)?;

        if !status.is_success() {
            return Err(ApiError::ApiError {
                status_code: status.as_u16(),
                message: body,
            });
        }

        let resp: OpenAIModelsResponse = serde_json::from_str(&body).map_err(ApiError::Json)?;

        let models = resp
            .data
            .into_iter()
            .map(|m| ModelInfo {
                id: m.id.clone(),
                name: m.id,
                provider: self.provider_type,
                context_length: None,
                supports_tools: self.supports_tools,
                supports_vision: self.supports_vision,
            })
            .collect();

        Ok(models)
    }

    fn supports_tools(&self) -> bool {
        self.supports_tools
    }

    fn supports_vision(&self) -> bool {
        self.supports_vision
    }

    fn max_context_length(&self) -> usize {
        self.max_context_length
    }
}

// OpenAI API响应结构

#[derive(Debug, Deserialize)]
struct OpenAIChatResponse {
    id: String,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
    #[serde(default)]
    error: Option<OpenAIError>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAIToolCall>>,
    role: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAIFunctionCall,
}

#[derive(Debug, Deserialize)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct OpenAIError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    id: String,
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct OpenAIDelta {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAIStreamToolCall>>,
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamToolCall {
    index: u32,
    id: Option<String>,
    #[serde(rename = "type")]
    call_type: Option<String>,
    function: Option<OpenAIStreamFunction>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModelData>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModelData {
    id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试创建OpenAIProvider
    #[test]
    fn test_openai_provider_creation() {
        let provider = OpenAIProvider::new(
            "OpenAI",
            Some("sk-test".to_string()),
            Some("https://api.openai.com/v1".to_string()),
            "gpt-4",
        );
        assert_eq!(provider.name(), "OpenAI");
        assert_eq!(provider.provider_type(), ProviderType::OpenAI);
        assert!(provider.supports_tools());
        assert!(provider.supports_vision());
        assert_eq!(provider.max_context_length(), 128000);
    }

    /// 测试消息转换
    #[test]
    fn test_convert_messages() {
        let provider = OpenAIProvider::new("test", None, None, "gpt-4");
        let messages = vec![
            ChatMessage::System {
                content: "你是助手".to_string(),
            },
            ChatMessage::User {
                content: MessageContent::Text("你好".to_string()),
            },
            ChatMessage::Assistant {
                content: Some("你好！".to_string()),
                tool_calls: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0]["role"], "system");
        assert_eq!(converted[1]["role"], "user");
        assert_eq!(converted[2]["role"], "assistant");
    }

    /// 测试多模态消息转换
    #[test]
    fn test_convert_multimodal_messages() {
        let provider = OpenAIProvider::new("test", None, None, "gpt-4");
        let messages = vec![ChatMessage::User {
            content: MessageContent::MultiPart(vec![
                Part {
                    part_type: "text".to_string(),
                    content: "描述这张图".to_string(),
                },
                Part {
                    part_type: "image_url".to_string(),
                    content: "https://example.com/image.png".to_string(),
                },
            ]),
        }];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        let content = converted[0]["content"].as_array().expect("内容应为数组");
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[1]["type"], "image_url");
    }

    /// 测试工具转换
    #[test]
    fn test_convert_tools() {
        let provider = OpenAIProvider::new("test", None, None, "gpt-4");
        let tools = vec![ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "fs_read".to_string(),
                description: "读取文件".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"]
                }),
            },
        }];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["type"], "function");
        assert_eq!(converted[0]["function"]["name"], "fs_read");
    }

    /// 测试响应解析
    #[test]
    fn test_parse_response() {
        let provider = OpenAIProvider::new("test", None, None, "gpt-4");
        let body = r#"{
            "id": "chat-123",
            "model": "gpt-4",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#;

        let response = provider.parse_response(body).expect("解析失败");
        assert_eq!(response.id, "chat-123");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.content, "Hello!");
        assert_eq!(response.finish_reason, Some("stop".to_string()));
        assert!(response.usage.is_some());
        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
    }

    /// 测试带工具调用的响应解析
    #[test]
    fn test_parse_tool_call_response() {
        let provider = OpenAIProvider::new("test", None, None, "gpt-4");
        let body = r#"{
            "id": "chat-456",
            "model": "gpt-4",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "fs_read",
                            "arguments": "{\"path\": \"/test.txt\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }"#;

        let response = provider.parse_response(body).expect("解析失败");
        assert_eq!(response.content, "");
        assert!(response.tool_calls.is_some());
        let calls = response.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_123");
        assert_eq!(calls[0].function.name, "fs_read");
    }

    /// 测试自定义base_url处理
    #[test]
    fn test_base_url_trimming() {
        let provider = OpenAIProvider::new(
            "test",
            None,
            Some("https://api.deepseek.com/v1/".to_string()),
            "deepseek-chat",
        );
        assert_eq!(provider.base_url, "https://api.deepseek.com/v1");
    }

    /// 测试Provider链式配置
    #[test]
    fn test_provider_chain_config() {
        let provider = OpenAIProvider::new("Custom", None, None, "model")
            .with_provider_type(ProviderType::OpenAICompatible)
            .with_tools_support(false)
            .with_vision_support(false)
            .with_max_context_length(32000);

        assert_eq!(provider.provider_type(), ProviderType::OpenAICompatible);
        assert!(!provider.supports_tools());
        assert!(!provider.supports_vision());
        assert_eq!(provider.max_context_length(), 32000);
    }
}
