use super::provider::*;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Google Gemini API Provider
pub struct GeminiProvider {
    name: String,
    client: Client,
    api_key: String,
    base_url: String,
    default_model: String,
    max_context_length: usize,
}

impl GeminiProvider {
    /// 创建新的Gemini Provider
    pub fn new(
        name: impl Into<String>,
        api_key: String,
        base_url: Option<String>,
        default_model: impl Into<String>,
    ) -> Self {
        let base_url = base_url
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string());
        let base_url = base_url.trim_end_matches('/').to_string();

        Self {
            name: name.into(),
            client: Client::new(),
            api_key,
            base_url,
            default_model: default_model.into(),
            max_context_length: 1000000, // Gemini 1.5 Pro支持1M上下文
        }
    }

    /// 设置最大上下文长度
    pub fn with_max_context_length(mut self, length: usize) -> Self {
        self.max_context_length = length;
        self
    }

    /// 构建请求URL（Gemini使用API key作为查询参数）
    fn chat_url(&self, model: &str, stream: bool) -> String {
        let action = if stream { "streamGenerateContent" } else { "generateContent" };
        format!(
            "{}/models/{}:{}?key={}",
            self.base_url, model, action, self.api_key
        )
    }

    /// 将统一消息格式转换为Gemini格式
    fn convert_messages(&self, messages: &[ChatMessage]) -> Vec<serde_json::Value> {
        let mut gemini_messages = Vec::new();

        for msg in messages {
            match msg {
                ChatMessage::System { content } => {
                    // Gemini通过systemInstruction字段传递系统提示
                    // 这里转换为user/assistant对话形式
                    gemini_messages.push(json!({
                        "role": "user",
                        "parts": [{"text": format!("[系统指令] {}", content)}]
                    }));
                    gemini_messages.push(json!({
                        "role": "model",
                        "parts": [{"text": "我明白了。"}]
                    }));
                }
                ChatMessage::User { content } => match content {
                    MessageContent::Text(text) => {
                        gemini_messages.push(json!({
                            "role": "user",
                            "parts": [{"text": text}]
                        }));
                    }
                    MessageContent::MultiPart(parts) => {
                        let gemini_parts: Vec<serde_json::Value> = parts
                            .iter()
                            .map(|part| match part.part_type.as_str() {
                                "text" => json!({"text": part.content}),
                                "image_url" | "image" => {
                                    // 处理base64或URL图片
                                    if part.content.starts_with("data:") {
                                        // data:image/png;base64,xxx 格式
                                        let parts: Vec<&str> = part.content.splitn(2, ',').collect();
                                        if parts.len() == 2 {
                                            let mime = parts[0]
                                                .trim_start_matches("data:")
                                                .trim_end_matches(";base64");
                                            json!({
                                                "inlineData": {
                                                    "mimeType": mime,
                                                    "data": parts[1]
                                                }
                                            })
                                        } else {
                                            json!({"text": part.content})
                                        }
                                    } else {
                                        json!({
                                            "fileData": {
                                                "mimeType": "image/*",
                                                "fileUri": part.content
                                            }
                                        })
                                    }
                                }
                                _ => json!({"text": part.content}),
                            })
                            .collect();
                        gemini_messages.push(json!({
                            "role": "user",
                            "parts": gemini_parts
                        }));
                    }
                },
                ChatMessage::Assistant {
                    content,
                    tool_calls,
                } => {
                    let mut parts = Vec::new();
                    if let Some(ref c) = content {
                        if !c.is_empty() {
                            parts.push(json!({"text": c}));
                        }
                    }
                    if let Some(ref calls) = tool_calls {
                        for call in calls {
                            parts.push(json!({
                                "functionCall": {
                                    "name": call.function.name,
                                    "args": serde_json::from_str::<serde_json::Value>(&call.function.arguments)
                                        .unwrap_or(json!({}))
                                }
                            }));
                        }
                    }
                    if !parts.is_empty() {
                        gemini_messages.push(json!({
                            "role": "model",
                            "parts": parts
                        }));
                    }
                }
                ChatMessage::Tool {
                    tool_call_id: _,
                    content,
                } => {
                    // Gemini的function response
                    gemini_messages.push(json!({
                        "role": "user",
                        "parts": [{
                            "functionResponse": {
                                "name": "result",
                                "response": {
                                    "result": content
                                }
                            }
                        }]
                    }));
                }
            }
        }

        gemini_messages
    }

    /// 转换工具定义为Gemini格式
    fn convert_tools(&self, tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
        tools
            .iter()
            .map(|tool| {
                json!({
                    "functionDeclarations": [{
                        "name": tool.function.name,
                        "description": tool.function.description,
                        "parameters": tool.function.parameters
                    }]
                })
            })
            .collect()
    }

    /// 解析Gemini响应
    fn parse_response(&self, body: &str) -> Result<ChatResponse, ApiError> {
        let resp: GeminiResponse = serde_json::from_str(body).map_err(ApiError::Json)?;

        if let Some(ref error) = resp.error {
            return Err(ApiError::ApiError {
                status_code: error.code.parse().unwrap_or(400),
                message: error.message.clone(),
            });
        }

        let mut content_text = String::new();
        let mut tool_calls = Vec::new();

        for candidate in &resp.candidates {
            for part in &candidate.content.parts {
                if let Some(ref text) = part.text {
                    content_text.push_str(text);
                }
                if let Some(ref func_call) = part.function_call {
                    tool_calls.push(ToolCall {
                        id: format!("call_{}", func_call.name),
                        call_type: "function".to_string(),
                        function: FunctionCall {
                            name: func_call.name.clone(),
                            arguments: serde_json::to_string(&func_call.args).unwrap_or_default(),
                        },
                    });
                }
            }
        }

        let usage = resp.usage_metadata.as_ref().map(|u| TokenUsage {
            prompt_tokens: u.prompt_token_count as u64,
            completion_tokens: u.candidates_token_count as u64,
            total_tokens: u.total_token_count as u64,
        });

        Ok(ChatResponse {
            id: format!("gemini-{}", chrono::Utc::now().timestamp()),
            model: self.default_model.clone(),
            content: content_text,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            usage,
            finish_reason: resp.candidates.first().and_then(|c| c.finish_reason.clone()),
        })
    }
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Gemini
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ApiError> {
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };

        let url = self.chat_url(&model, false);
        let contents = self.convert_messages(&request.messages);

        let mut payload = json!({
            "contents": contents,
        });

        if let Some(temp) = request.temperature {
            payload["generationConfig"] = json!({
                "temperature": temp
            });
        }
        if let Some(max_tokens) = request.max_tokens {
            if payload.get_mut("generationConfig").is_none() || payload["generationConfig"].is_null() {
                payload["generationConfig"] = json!({"maxOutputTokens": max_tokens});
            } else {
                payload["generationConfig"]["maxOutputTokens"] = json!(max_tokens);
            }
        }
        if let Some(ref tools) = request.tools {
            payload["tools"] = json!(self.convert_tools(tools));
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
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };

        let url = self.chat_url(&model, true);
        let contents = self.convert_messages(&request.messages);

        let mut payload = json!({
            "contents": contents,
        });

        if let Some(temp) = request.temperature {
            payload["generationConfig"] = json!({
                "temperature": temp
            });
        }
        if let Some(ref tools) = request.tools {
            payload["tools"] = json!(self.convert_tools(tools));
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

        // Gemini流式响应是JSON数组格式
        let stream = response
            .bytes_stream()
            .map(|chunk| {
                chunk.map_err(ApiError::Http).and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes);
                    // Gemini流式响应以逗号分隔的JSON对象
                    let mut deltas: Vec<Result<StreamChunk, ApiError>> = Vec::new();

                    for line in text.lines() {
                        let line = line.trim().trim_start_matches(',').trim_start_matches('[').trim_end_matches(']');
                        if line.is_empty() {
                            continue;
                        }

                        if let Ok(chunk_resp) = serde_json::from_str::<GeminiResponse>(line) {
                            for candidate in &chunk_resp.candidates {
                                for part in &candidate.content.parts {
                                    if let Some(ref text) = part.text {
                                        if !text.is_empty() {
                                            deltas.push(Ok(StreamChunk::Content { delta: text.clone() }));
                                        }
                                    }
                                }
                                if candidate.finish_reason.is_some() {
                                    deltas.push(Ok(StreamChunk::Done));
                                }
                            }
                        }
                    }

                    if deltas.is_empty() {
                        Ok(vec![StreamChunk::Content { delta: String::new() }])
                    } else {
                        Ok(deltas.into_iter().filter_map(|r| r.ok()).collect())
                    }
                })
            })
            .flat_map(|result| {
                // 将Vec<StreamChunk>展开为单独的StreamChunk
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
        let url = format!("{}/models?key={}", self.base_url, self.api_key);
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

        let resp: GeminiModelsResponse = response.json().await.map_err(ApiError::Http)?;

        let models = resp
            .models
            .into_iter()
            .map(|m| ModelInfo {
                id: m.name.replace("models/", ""),
                name: m.display_name.unwrap_or_else(|| m.name.clone()),
                provider: ProviderType::Gemini,
                context_length: m
                    .input_token_limit
                    .map(|l| l as usize),
                supports_tools: true,
                supports_vision: m.name.contains("vision") || m.name.contains("pro"),
            })
            .collect();

        Ok(models)
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

// Gemini API响应结构

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsageMetadata>,
    #[serde(default)]
    error: Option<GeminiError>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
    #[serde(default)]
    finish_reason: Option<String>,
    #[serde(default)]
    index: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    #[serde(default)]
    role: Option<String>,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    #[serde(default)]
    text: Option<String>,
    #[serde(default, rename = "functionCall")]
    function_call: Option<GeminiFunctionCall>,
}

#[derive(Debug, Deserialize)]
struct GeminiFunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct GeminiUsageMetadata {
    prompt_token_count: i64,
    candidates_token_count: i64,
    total_token_count: i64,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    code: String,
    message: String,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiModelsResponse {
    models: Vec<GeminiModelData>,
}

#[derive(Debug, Deserialize)]
struct GeminiModelData {
    name: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    input_token_limit: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试创建GeminiProvider
    #[test]
    fn test_gemini_provider_creation() {
        let provider = GeminiProvider::new(
            "Gemini",
            "test-api-key".to_string(),
            None,
            "gemini-1.5-pro",
        );
        assert_eq!(provider.name(), "Gemini");
        assert_eq!(provider.provider_type(), ProviderType::Gemini);
        assert!(provider.supports_tools());
        assert!(provider.supports_vision());
        assert_eq!(provider.max_context_length(), 1000000);
    }

    /// 测试消息转换
    #[test]
    fn test_convert_messages() {
        let provider = GeminiProvider::new("test", "key".to_string(), None, "gemini");
        let messages = vec![
            ChatMessage::System {
                content: "你是助手".to_string(),
            },
            ChatMessage::User {
                content: MessageContent::Text("你好".to_string()),
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 3); // system转为user/model对 + user消息
        assert_eq!(converted[0]["role"], "user");
        assert_eq!(converted[1]["role"], "model");
        assert_eq!(converted[2]["role"], "user");
    }

    /// 测试纯文本消息转换
    #[test]
    fn test_convert_text_message() {
        let provider = GeminiProvider::new("test", "key".to_string(), None, "gemini");
        let messages = vec![ChatMessage::User {
            content: MessageContent::Text("Hello".to_string()),
        }];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["role"], "user");
        let parts = converted[0]["parts"].as_array().unwrap();
        assert_eq!(parts[0]["text"], "Hello");
    }

    /// 测试多模态消息转换
    #[test]
    fn test_convert_multimodal_messages() {
        let provider = GeminiProvider::new("test", "key".to_string(), None, "gemini");
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
        let parts = converted[0]["parts"].as_array().unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0]["text"], "描述这张图");
        assert!(parts[1]["inlineData"].is_object());
    }

    /// 测试工具定义转换
    #[test]
    fn test_convert_tools() {
        let provider = GeminiProvider::new("test", "key".to_string(), None, "gemini");
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
        assert!(converted[0]["functionDeclarations"].is_array());
        let func = &converted[0]["functionDeclarations"][0];
        assert_eq!(func["name"], "fs_read");
    }

    /// 测试URL构建
    #[test]
    fn test_chat_url_building() {
        let provider = GeminiProvider::new("test", "test-key".to_string(), None, "gemini-pro");
        let url = provider.chat_url("gemini-pro", false);
        assert!(url.contains("generateContent"));
        assert!(url.contains("key=test-key"));

        let stream_url = provider.chat_url("gemini-pro", true);
        assert!(stream_url.contains("streamGenerateContent"));
    }

    /// 测试base_url尾部斜杠处理
    #[test]
    fn test_base_url_trimming() {
        let provider = GeminiProvider::new(
            "test",
            "key".to_string(),
            Some("https://custom.googleapis.com/v1/".to_string()),
            "gemini",
        );
        assert_eq!(provider.base_url, "https://custom.googleapis.com/v1");
    }

    /// 测试响应解析
    #[test]
    fn test_parse_response() {
        let provider = GeminiProvider::new("test", "key".to_string(), None, "gemini-pro");
        let body = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello! How can I help you?"}]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 8,
                "totalTokenCount": 18
            }
        }"#;

        let response = provider.parse_response(body).expect("解析失败");
        assert_eq!(response.model, "gemini-pro");
        assert_eq!(response.content, "Hello! How can I help you?");
        assert!(response.usage.is_some());
        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 8);
    }

    /// 测试带工具调用的响应解析
    #[test]
    fn test_parse_tool_call_response() {
        let provider = GeminiProvider::new("test", "key".to_string(), None, "gemini-pro");
        let body = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{
                        "functionCall": {
                            "name": "fs_read",
                            "args": {"path": "/test.txt"}
                        }
                    }]
                },
                "finishReason": "STOP"
            }]
        }"#;

        let response = provider.parse_response(body).expect("解析失败");
        assert!(response.tool_calls.is_some());
        let calls = response.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "fs_read");
    }

    /// 测试空消息过滤
    #[test]
    fn test_empty_assistant_message_filtering() {
        let provider = GeminiProvider::new("test", "key".to_string(), None, "gemini");
        let messages = vec![ChatMessage::Assistant {
            content: Some("".to_string()),
            tool_calls: None,
        }];

        let converted = provider.convert_messages(&messages);
        // 空内容的消息不应该被添加
        assert!(converted.is_empty());
    }
}
