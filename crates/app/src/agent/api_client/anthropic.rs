use super::provider::*;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Anthropic Messages API Provider —— 支持Claude系列模型
pub struct AnthropicProvider {
    name: String,
    client: Client,
    api_key: String,
    base_url: String,
    default_model: String,
    max_context_length: usize,
}

impl AnthropicProvider {
    /// 创建新的Anthropic Provider
    pub fn new(
        name: impl Into<String>,
        api_key: String,
        base_url: Option<String>,
        default_model: impl Into<String>,
    ) -> Self {
        let base_url = base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string());
        let base_url = base_url.trim_end_matches('/').to_string();

        Self {
            name: name.into(),
            client: Client::new(),
            api_key,
            base_url,
            default_model: default_model.into(),
            max_context_length: 200000,
        }
    }

    /// 设置最大上下文长度
    pub fn with_max_context_length(mut self, length: usize) -> Self {
        self.max_context_length = length;
        self
    }

    /// 构建请求URL
    fn chat_url(&self) -> String {
        format!("{}/v1/messages", self.base_url)
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers.insert(
            "x-api-key",
            self.api_key.parse().unwrap(),
        );
        headers.insert(
            "anthropic-version",
            "2023-06-01".parse().unwrap(),
        );
        headers
    }

    /// 将统一消息格式转换为Anthropic Messages API格式
    fn convert_messages(
        &self,
        messages: &[ChatMessage],
    ) -> (Option<String>, Vec<serde_json::Value>) {
        let mut system_prompt = None;
        let mut anthropic_messages = Vec::new();

        for msg in messages {
            match msg {
                ChatMessage::System { content } => {
                    system_prompt = Some(content.clone());
                }
                ChatMessage::User { content } => match content {
                    MessageContent::Text(text) => {
                        anthropic_messages.push(json!({
                            "role": "user",
                            "content": [{"type": "text", "text": text}]
                        }));
                    }
                    MessageContent::MultiPart(parts) => {
                        let content_parts: Vec<serde_json::Value> = parts
                            .iter()
                            .map(|part| match part.part_type.as_str() {
                                "text" => json!({"type": "text", "text": part.content}),
                                "image_url" | "image" => {
                                    json!({"type": "image", "source": {
                                        "type": "url",
                                        "url": part.content
                                    }})
                                }
                                _ => json!({"type": "text", "text": part.content}),
                            })
                            .collect();
                        anthropic_messages.push(json!({
                            "role": "user",
                            "content": content_parts
                        }));
                    }
                },
                ChatMessage::Assistant {
                    content,
                    tool_calls,
                } => {
                    let mut anthropic_content = Vec::new();

                    if let Some(ref c) = content {
                        if !c.is_empty() {
                            anthropic_content.push(json!({
                                "type": "text",
                                "text": c
                            }));
                        }
                    }

                    if let Some(ref calls) = tool_calls {
                        for call in calls {
                            anthropic_content.push(json!({
                                "type": "tool_use",
                                "id": call.id.clone(),
                                "name": call.function.name.clone(),
                                "input": serde_json::from_str::<serde_json::Value>(&call.function.arguments)
                                    .unwrap_or(json!({}))
                            }));
                        }
                    }

                    anthropic_messages.push(json!({
                        "role": "assistant",
                        "content": anthropic_content
                    }));
                }
                ChatMessage::Tool {
                    tool_call_id,
                    content,
                } => {
                    anthropic_messages.push(json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": tool_call_id,
                            "content": content
                        }]
                    }));
                }
            }
        }

        (system_prompt, anthropic_messages)
    }

    /// 转换工具定义为Anthropic格式
    fn convert_tools(&self, tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
        tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.function.name,
                    "description": tool.function.description,
                    "input_schema": tool.function.parameters
                })
            })
            .collect()
    }

    /// 解析Anthropic响应
    fn parse_response(&self, body: &str) -> Result<ChatResponse, ApiError> {
        let resp: AnthropicResponse = serde_json::from_str(body).map_err(ApiError::Json)?;

        if let Some(error) = resp.error {
            return Err(ApiError::ApiError {
                status_code: error.error_type.parse().unwrap_or(400),
                message: error.message,
            });
        }

        let mut content_text = String::new();
        let mut tool_calls = Vec::new();

        for block in resp.content {
            match block.content_type.as_str() {
                "text" => {
                    if let Some(text) = block.text {
                        content_text.push_str(&text);
                    }
                }
                "tool_use" => {
                    if let Some(input) = block.input {
                        tool_calls.push(ToolCall {
                            id: block.id.unwrap_or_default(),
                            call_type: "function".to_string(),
                            function: FunctionCall {
                                name: block.name.unwrap_or_default(),
                                arguments: input.to_string(),
                            },
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(ChatResponse {
            id: resp.id,
            model: resp.model,
            content: content_text,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            usage: resp.usage.map(|u| TokenUsage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
            }),
            finish_reason: resp.stop_reason,
        })
    }

    /// 解析流式事件
    fn parse_stream_event(event_type: &str, data: &str) -> Result<Option<StreamChunk>, ApiError> {
        if event_type == "message_stop" {
            return Ok(Some(StreamChunk::Done));
        }

        if event_type != "content_block_delta" {
            return Ok(None);
        }

        let delta: AnthropicStreamDelta = match serde_json::from_str(data) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };

        if let Some(text) = delta.delta.text {
            if !text.is_empty() {
                return Ok(Some(StreamChunk::Content { delta: text }));
            }
        }

        Ok(None)
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Anthropic
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ApiError> {
        let url = self.chat_url();
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };

        let (system_prompt, messages) = self.convert_messages(&request.messages);

        let mut payload = json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        if let Some(ref system) = system_prompt {
            payload["system"] = json!(system);
        }
        if let Some(temp) = request.temperature {
            payload["temperature"] = json!(temp);
        }
        if let Some(ref tools) = request.tools {
            payload["tools"] = json!(self.convert_tools(tools));
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

        let (system_prompt, messages) = self.convert_messages(&request.messages);

        let mut payload = json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true,
        });

        if let Some(ref system) = system_prompt {
            payload["system"] = json!(system);
        }
        if let Some(temp) = request.temperature {
            payload["temperature"] = json!(temp);
        }
        if let Some(ref tools) = request.tools {
            payload["tools"] = json!(self.convert_tools(tools));
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

        // Anthropic的流式响应格式特殊：event类型标识数据类型
        let stream = response
            .bytes_stream()
            .eventsource_stream()
            .map(|event| {
                event.map_err(|e| ApiError::StreamParse(e.to_string())).and_then(|event| {
                    Self::parse_stream_event(&event.event, &event.data)
                })
            })
            .filter_map(|item| async { item.transpose() });

        Ok(Box::pin(stream))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ApiError> {
        // Anthropic目前没有公开的模型列表API，返回已知模型
        Ok(vec![
            ModelInfo {
                id: "claude-3-5-sonnet-20241022".to_string(),
                name: "Claude 3.5 Sonnet".to_string(),
                provider: ProviderType::Anthropic,
                context_length: Some(200000),
                supports_tools: true,
                supports_vision: true,
            },
            ModelInfo {
                id: "claude-3-5-haiku-20241022".to_string(),
                name: "Claude 3.5 Haiku".to_string(),
                provider: ProviderType::Anthropic,
                context_length: Some(200000),
                supports_tools: true,
                supports_vision: false,
            },
            ModelInfo {
                id: "claude-3-opus-20240229".to_string(),
                name: "Claude 3 Opus".to_string(),
                provider: ProviderType::Anthropic,
                context_length: Some(200000),
                supports_tools: true,
                supports_vision: true,
            },
        ])
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        true
    }

    fn max_context_length(&self) -> usize {
        self.max_context_length
    }
}

// Anthropic API响应结构

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    model: String,
    content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    error: Option<AnthropicError>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct AnthropicError {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamDelta {
    delta: AnthropicDeltaContent,
}

#[derive(Debug, Deserialize, Default)]
struct AnthropicDeltaContent {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    content_type: Option<String>,
}

// 简单的SSE事件结构
#[derive(Debug)]
struct AnthropicStreamEvent {
    event: String,
    data: String,
}

// 将bytes_stream转换为eventsource_stream的辅助trait
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

struct EventSourceStream<S> {
    inner: S,
    buffer: String,
}

impl<S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin> Stream for EventSourceStream<S> {
    type Item = Result<AnthropicStreamEvent, ApiError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                let text = String::from_utf8_lossy(&bytes);
                self.buffer.push_str(&text);

                if let Some(pos) = self.buffer.find("\n\n") {
                    let chunk = self.buffer[..pos].to_string();
                    self.buffer.drain(..pos + 2);

                    let mut event = "message".to_string();
                    let mut data = String::new();

                    for line in chunk.lines() {
                        if let Some(val) = line.strip_prefix("event: ") {
                            event = val.to_string();
                        } else if let Some(val) = line.strip_prefix("data: ") {
                            data = val.to_string();
                        }
                    }

                    Poll::Ready(Some(Ok(AnthropicStreamEvent { event, data })))
                } else {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(ApiError::Http(e)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

trait EventsourceStreamExt<S> {
    fn eventsource_stream(self) -> EventSourceStream<S>;
}

impl<S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin> EventsourceStreamExt<S> for S {
    fn eventsource_stream(self) -> EventSourceStream<S> {
        EventSourceStream {
            inner: self,
            buffer: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试创建AnthropicProvider
    #[test]
    fn test_anthropic_provider_creation() {
        let provider = AnthropicProvider::new(
            "Claude",
            "sk-ant-test".to_string(),
            None,
            "claude-3-5-sonnet",
        );
        assert_eq!(provider.name(), "Claude");
        assert_eq!(provider.provider_type(), ProviderType::Anthropic);
        assert!(provider.supports_tools());
        assert!(provider.supports_vision());
        assert_eq!(provider.max_context_length(), 200000);
    }

    /// 测试消息转换
    #[test]
    fn test_convert_messages() {
        let provider = AnthropicProvider::new("test", "key".to_string(), None, "claude");
        let messages = vec![
            ChatMessage::System {
                content: "你是助手".to_string(),
            },
            ChatMessage::User {
                content: MessageContent::Text("你好".to_string()),
            },
        ];

        let (system_prompt, converted) = provider.convert_messages(&messages);
        assert_eq!(system_prompt, Some("你是助手".to_string()));
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["role"], "user");
    }

    /// 测试工具调用消息转换
    #[test]
    fn test_convert_tool_messages() {
        let provider = AnthropicProvider::new("test", "key".to_string(), None, "claude");
        let messages = vec![
            ChatMessage::User {
                content: MessageContent::Text("读取文件".to_string()),
            },
            ChatMessage::Assistant {
                content: None,
                tool_calls: Some(vec![ToolCall {
                    id: "call_1".to_string(),
                    call_type: "function".to_string(),
                    function: FunctionCall {
                        name: "fs_read".to_string(),
                        arguments: "{\"path\": \"/test.txt\"}".to_string(),
                    },
                }]),
            },
            ChatMessage::Tool {
                tool_call_id: "call_1".to_string(),
                content: "文件内容".to_string(),
            },
        ];

        let (_, converted) = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 3);
        // 检查tool_result消息是否正确转换
        assert_eq!(converted[2]["role"], "user");
    }

    /// 测试工具定义转换
    #[test]
    fn test_convert_tools() {
        let provider = AnthropicProvider::new("test", "key".to_string(), None, "claude");
        let tools = vec![ToolDefinition {
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
        }];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["name"], "fs_read");
        assert_eq!(converted[0]["description"], "读取文件");
        assert!(converted[0]["input_schema"].is_object());
    }

    /// 测试响应解析
    #[test]
    fn test_parse_response() {
        let provider = AnthropicProvider::new("test", "key".to_string(), None, "claude");
        let body = r#"{
            "id": "msg_123",
            "model": "claude-3-5-sonnet",
            "content": [
                {"type": "text", "text": "Hello!"}
            ],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            },
            "stop_reason": "end_turn"
        }"#;

        let response = provider.parse_response(body).expect("解析失败");
        assert_eq!(response.id, "msg_123");
        assert_eq!(response.model, "claude-3-5-sonnet");
        assert_eq!(response.content, "Hello!");
        assert!(response.usage.is_some());
    }

    /// 测试带工具调用的响应解析
    #[test]
    fn test_parse_tool_use_response() {
        let provider = AnthropicProvider::new("test", "key".to_string(), None, "claude");
        let body = r#"{
            "id": "msg_456",
            "model": "claude-3-5-sonnet",
            "content": [
                {"type": "tool_use", "id": "tool_1", "name": "fs_read", "input": {"path": "/test.txt"}}
            ],
            "stop_reason": "tool_use"
        }"#;

        let response = provider.parse_response(body).expect("解析失败");
        assert!(response.tool_calls.is_some());
        let calls = response.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "tool_1");
        assert_eq!(calls[0].function.name, "fs_read");
    }

    /// 测试base_url尾部斜杠处理
    #[test]
    fn test_base_url_trimming() {
        let provider = AnthropicProvider::new(
            "test",
            "key".to_string(),
            Some("https://api.anthropic.com/".to_string()),
            "claude",
        );
        assert_eq!(provider.base_url, "https://api.anthropic.com");
    }

    /// 测试流式事件解析
    #[test]
    fn test_stream_event_parsing() {
        let provider = AnthropicProvider::new("test", "key".to_string(), None, "claude");

        // message_stop事件
        let result = provider.parse_stream_event("message_stop", "");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Some(StreamChunk::Done)));

        // 内容增量事件
        let result = provider.parse_stream_event(
            "content_block_delta",
            r#"{"delta": {"text": "Hello"}}"#,
        );
        assert!(result.is_ok());
        if let Some(StreamChunk::Content { delta }) = result.unwrap() {
            assert_eq!(delta, "Hello");
        }

        // 未知事件类型
        let result = provider.parse_stream_event("ping", "");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
