use super::provider::*;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Ollama本地模型 Provider
pub struct OllamaProvider {
    name: String,
    client: Client,
    base_url: String,
    default_model: String,
    supports_tools: bool,
    supports_vision: bool,
    max_context_length: usize,
}

impl OllamaProvider {
    /// 创建新的Ollama Provider
    pub fn new(
        name: impl Into<String>,
        base_url: Option<String>,
        default_model: impl Into<String>,
    ) -> Self {
        let base_url = base_url.unwrap_or_else(|| "http://localhost:11434".to_string());
        let base_url = base_url.trim_end_matches('/').to_string();

        Self {
            name: name.into(),
            client: Client::new(),
            base_url,
            default_model: default_model.into(),
            supports_tools: true,
            supports_vision: false,
            max_context_length: 32768, // 默认上下文，取决于具体模型
        }
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

    /// 构建Chat API URL
    fn chat_url(&self) -> String {
        format!("{}/api/chat", self.base_url)
    }

    /// 构建List API URL
    fn list_url(&self) -> String {
        format!("{}/api/tags", self.base_url)
    }

    /// 将统一消息格式转换为Ollama格式
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
                        // Ollama支持images数组
                        let mut text_content = String::new();
                        let mut images = Vec::new();

                        for part in parts {
                            match part.part_type.as_str() {
                                "text" => text_content.push_str(&part.content),
                                "image_url" => {
                                    // 提取base64数据
                                    if part.content.starts_with("data:") {
                                        if let Some(idx) = part.content.find(',') {
                                            images.push(part.content[idx + 1..].to_string());
                                        }
                                    } else {
                                        images.push(part.content.clone());
                                    }
                                }
                                _ => {}
                            }
                        }

                        let mut msg = json!({
                            "role": "user",
                            "content": text_content
                        });
                        if !images.is_empty() {
                            msg["images"] = json!(images);
                        }
                        msg
                    }
                },
                ChatMessage::Assistant {
                    content,
                    tool_calls,
                } => {
                    let mut msg = json!({
                        "role": "assistant"
                    });
                    if let Some(ref c) = content {
                        msg["content"] = json!(c);
                    }
                    if let Some(ref calls) = tool_calls {
                        let ollama_tools: Vec<serde_json::Value> = calls
                            .iter()
                            .map(|call| {
                                json!({
                                    "function": {
                                        "name": call.function.name,
                                        "arguments": serde_json::from_str::<serde_json::Value>(&call.function.arguments)
                                            .unwrap_or(json!({}))
                                    }
                                })
                            })
                            .collect();
                        if !ollama_tools.is_empty() {
                            msg["tool_calls"] = json!(ollama_tools);
                        }
                    }
                    msg
                }
                ChatMessage::Tool {
                    tool_call_id: _,
                    content,
                } => json!({
                    "role": "tool",
                    "content": content
                }),
            })
            .collect()
    }

    /// 转换工具定义为Ollama格式
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

    /// 解析Ollama响应
    fn parse_response(&self, body: &str) -> Result<ChatResponse, ApiError> {
        let resp: OllamaChatResponse = serde_json::from_str(body).map_err(ApiError::Json)?;

        if let Some(ref error) = resp.error {
            return Err(ApiError::ApiError {
                status_code: 500,
                message: error.clone(),
            });
        }

        let tool_calls = if let Some(ref calls) = resp.message.tool_calls {
            Some(
                calls
                    .iter()
                    .map(|call| ToolCall {
                        id: format!("call_{}", call.function.name),
                        call_type: "function".to_string(),
                        function: FunctionCall {
                            name: call.function.name.clone(),
                            arguments: serde_json::to_string(&call.function.arguments)
                                .unwrap_or_default(),
                        },
                    })
                    .collect(),
            )
        } else {
            None
        };

        Ok(ChatResponse {
            id: format!("ollama-{}", chrono::Utc::now().timestamp()),
            model: resp.model,
            content: resp.message.content.unwrap_or_default(),
            tool_calls,
            usage: resp.prompt_eval_count.map(|prompt_tokens| TokenUsage {
                prompt_tokens,
                completion_tokens: resp.eval_count.unwrap_or(0),
                total_tokens: prompt_tokens + resp.eval_count.unwrap_or(0),
            }),
            finish_reason: resp.done_reason,
        })
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Ollama
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ApiError> {
        let url = self.chat_url();
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };

        let messages = self.convert_messages(&request.messages);

        let mut payload = json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

        if let Some(temp) = request.temperature {
            payload["options"] = json!({
                "temperature": temp
            });
        }
        if let Some(max_tokens) = request.max_tokens {
            if payload.get_mut("options").is_none() || payload["options"].is_null() {
                payload["options"] = json!({"num_predict": max_tokens as i64});
            } else {
                payload["options"]["num_predict"] = json!(max_tokens as i64);
            }
        }
        if let Some(ref tools) = request.tools {
            if self.supports_tools {
                payload["tools"] = json!(self.convert_tools(tools));
            }
        }

        let response = self
            .client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
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

        let messages = self.convert_messages(&request.messages);

        let mut payload = json!({
            "model": model,
            "messages": messages,
            "stream": true,
        });

        if let Some(temp) = request.temperature {
            payload["options"] = json!({
                "temperature": temp
            });
        }
        if let Some(ref tools) = request.tools {
            if self.supports_tools {
                payload["tools"] = json!(self.convert_tools(tools));
            }
        }

        let response = self
            .client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
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

        // Ollama流式响应：每行一个JSON对象
        let stream = response
            .bytes_stream()
            .map(|chunk| {
                chunk.map_err(ApiError::Http).and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut results = Vec::new();

                    for line in text.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }

                        match serde_json::from_str::<OllamaStreamChunk>(line) {
                            Ok(chunk) => {
                                if let Some(content) = chunk.message.and_then(|m| m.content) {
                                    if !content.is_empty() {
                                        results.push(Ok(StreamChunk::Content { delta: content }));
                                    }
                                }
                                if chunk.done {
                                    results.push(Ok(StreamChunk::Done));
                                }
                            }
                            Err(_) => {
                                // 尝试解析为错误响应
                                if let Ok(err) = serde_json::from_str::<OllamaErrorResponse>(line) {
                                    results.push(Err(ApiError::ApiError {
                                        status_code: 500,
                                        message: err.error,
                                    }));
                                }
                            }
                        }
                    }

                    if results.is_empty() {
                        Ok(vec![StreamChunk::Content { delta: String::new() }])
                    } else {
                        Ok(results.into_iter().filter_map(|r| r.ok()).collect())
                    }
                })
            })
            .flat_map(|result| {
                futures::stream::iter(match result {
                    Ok(chunks) => chunks.into_iter().map(Ok).collect::<Vec<_>>(),
                    Err(e) => vec![Err(e)],
                })
            })
            .filter(|item| {
                futures::future::ready(
                    !matches!(item, Ok(StreamChunk::Content { delta }) if delta.is_empty())
                )
            });

        Ok(Box::pin(stream))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ApiError> {
        let url = self.list_url();
        let response = self
            .client
            .get(&url)
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

        let resp: OllamaListResponse = response.json().await.map_err(ApiError::Http)?;

        let models = resp
            .models
            .into_iter()
            .map(|m| ModelInfo {
                id: m.name.clone(),
                name: m.name.clone(),
                provider: ProviderType::Ollama,
                context_length: m.details.context_length.map(|l| l as usize),
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

// Ollama API响应结构

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    model: String,
    message: OllamaMessage,
    #[serde(default)]
    done_reason: Option<String>,
    #[serde(default)]
    prompt_eval_count: Option<u64>,
    #[serde(default)]
    eval_count: Option<u64>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    role: String,
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OllamaToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OllamaToolCall {
    function: OllamaFunctionCall,
}

#[derive(Debug, Deserialize)]
struct OllamaFunctionCall {
    name: String,
    arguments: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamChunk {
    #[serde(default)]
    message: Option<OllamaMessage>,
    #[serde(default)]
    done: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaErrorResponse {
    error: String,
}

#[derive(Debug, Deserialize)]
struct OllamaListResponse {
    models: Vec<OllamaModelData>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelData {
    name: String,
    #[serde(default)]
    details: OllamaModelDetails,
}

#[derive(Debug, Deserialize, Default)]
struct OllamaModelDetails {
    #[serde(default)]
    context_length: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试创建OllamaProvider
    #[test]
    fn test_ollama_provider_creation() {
        let provider = OllamaProvider::new("Ollama", None, "llama3.2");
        assert_eq!(provider.name(), "Ollama");
        assert_eq!(provider.provider_type(), ProviderType::Ollama);
        assert_eq!(provider.base_url, "http://localhost:11434");
    }

    /// 测试自定义base_url
    #[test]
    fn test_custom_base_url() {
        let provider = OllamaProvider::new(
            "Ollama",
            Some("http://192.168.1.100:11434".to_string()),
            "mistral",
        );
        assert_eq!(provider.base_url, "http://192.168.1.100:11434");
    }

    /// 测试消息转换
    #[test]
    fn test_convert_messages() {
        let provider = OllamaProvider::new("test", None, "llama3.2");
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
        let provider = OllamaProvider::new("test", None, "llava").with_vision_support(true);
        let messages = vec![ChatMessage::User {
            content: MessageContent::MultiPart(vec![
                Part {
                    part_type: "text".to_string(),
                    content: "描述这张图".to_string(),
                },
                Part {
                    part_type: "image_url".to_string(),
                    content: "data:image/png;base64,iVBORw0KGgo".to_string(),
                },
            ]),
        }];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted[0]["role"], "user");
        assert!(converted[0]["images"].is_array());
        let images = converted[0]["images"].as_array().unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0], "iVBORw0KGgo");
    }

    /// 测试工具定义转换
    #[test]
    fn test_convert_tools() {
        let provider = OllamaProvider::new("test", None, "llama3.2");
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
        assert_eq!(converted[0]["type"], "function");
    }

    /// 测试响应解析
    #[test]
    fn test_parse_response() {
        let provider = OllamaProvider::new("test", None, "llama3.2");
        let body = r#"{
            "model": "llama3.2",
            "message": {
                "role": "assistant",
                "content": "Hello! I'm a local model."
            },
            "done_reason": "stop",
            "prompt_eval_count": 25,
            "eval_count": 15
        }"#;

        let response = provider.parse_response(body).expect("解析失败");
        assert_eq!(response.model, "llama3.2");
        assert_eq!(response.content, "Hello! I'm a local model.");
        assert!(response.usage.is_some());
        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 25);
        assert_eq!(usage.completion_tokens, 15);
        assert_eq!(usage.total_tokens, 40);
    }

    /// 测试错误响应解析
    #[test]
    fn test_parse_error_response() {
        let provider = OllamaProvider::new("test", None, "llama3.2");
        let body = r#"{
            "error": "model 'unknown' not found"
        }"#;

        let result = provider.parse_response(body);
        assert!(result.is_err());
        match result {
            Err(ApiError::ApiError { status_code, message }) => {
                assert_eq!(status_code, 500);
                assert!(message.contains("not found"));
            }
            _ => panic!("应该返回ApiError"),
        }
    }

    /// 测试URL构建
    #[test]
    fn test_url_building() {
        let provider = OllamaProvider::new("test", None, "llama");
        assert_eq!(provider.chat_url(), "http://localhost:11434/api/chat");
        assert_eq!(provider.list_url(), "http://localhost:11434/api/tags");
    }

    /// 测试链式配置
    #[test]
    fn test_chain_config() {
        let provider = OllamaProvider::new("test", None, "mistral")
            .with_tools_support(false)
            .with_vision_support(true)
            .with_max_context_length(64000);

        assert!(!provider.supports_tools());
        assert!(provider.supports_vision());
        assert_eq!(provider.max_context_length(), 64000);
    }
}
