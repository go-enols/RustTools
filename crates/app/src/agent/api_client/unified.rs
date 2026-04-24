use super::provider::*;
use super::{
    anthropic::AnthropicProvider, gemini::GeminiProvider, model_registry::ModelRegistry,
    ollama::OllamaProvider, openai::OpenAIProvider,
};
use crate::agent::config::{ModelConfig, ProviderType as ConfigProviderType, TaskCondition};
use futures::stream::{BoxStream, StreamExt};
use std::collections::HashMap;

/// UnifiedClient —— 统一的LLM调用入口
///
/// 根据配置自动路由到对应的Provider，支持Auto模式根据任务条件选择模型
pub struct UnifiedClient {
    providers: HashMap<String, Box<dyn LLMProvider>>,
    registry: ModelRegistry,
}

impl UnifiedClient {
    /// 从ModelConfig列表创建UnifiedClient
    pub fn new(configs: &[ModelConfig]) -> Result<Self, ApiError> {
        let mut providers: HashMap<String, Box<dyn LLMProvider>> = HashMap::new();

        for config in configs {
            let provider: Box<dyn LLMProvider> = match config.provider {
                ConfigProviderType::OpenAI => Box::new(
                    OpenAIProvider::new(
                        &config.name,
                        config.api_key.clone(),
                        config.base_url.clone(),
                        &config.default_model,
                    )
                    .with_provider_type(ProviderType::OpenAI),
                ),
                ConfigProviderType::Anthropic => {
                    let api_key = config.api_key.clone().ok_or_else(|| {
                        ApiError::Config(format!("Anthropic模型 '{}' 需要API key", config.id))
                    })?;
                    Box::new(AnthropicProvider::new(
                        &config.name,
                        api_key,
                        config.base_url.clone(),
                        &config.default_model,
                    ))
                }
                ConfigProviderType::Gemini => {
                    let api_key = config.api_key.clone().ok_or_else(|| {
                        ApiError::Config(format!("Gemini模型 '{}' 需要API key", config.id))
                    })?;
                    Box::new(GeminiProvider::new(
                        &config.name,
                        api_key,
                        config.base_url.clone(),
                        &config.default_model,
                    ))
                }
                ConfigProviderType::Ollama => Box::new(OllamaProvider::new(
                    &config.name,
                    config.base_url.clone(),
                    &config.default_model,
                )),
                ConfigProviderType::OpenAICompatible => Box::new(
                    OpenAIProvider::new(
                        &config.name,
                        config.api_key.clone(),
                        config.base_url.clone(),
                        &config.default_model,
                    )
                    .with_provider_type(ProviderType::OpenAICompatible),
                ),
            };

            providers.insert(config.id.clone(), provider);
        }

        Ok(Self {
            providers,
            registry: ModelRegistry::new(),
        })
    }

    /// 注册自定义Provider
    pub fn register_provider(
        &mut self,
        model_id: impl Into<String>,
        provider: Box<dyn LLMProvider>,
    ) {
        self.providers.insert(model_id.into(), provider);
    }

    /// 非流式聊天 —— 根据model_id路由到对应Provider
    pub async fn chat(
        &self,
        model_id: &str,
        request: ChatRequest,
    ) -> Result<ChatResponse, ApiError> {
        let provider = self
            .providers
            .get(model_id)
            .ok_or_else(|| ApiError::ProviderNotFound(model_id.to_string()))?;
        provider.chat(request).await
    }

    /// 流式聊天 —— 根据model_id路由到对应Provider
    pub async fn chat_stream(
        &self,
        model_id: &str,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamChunk, ApiError>>, ApiError> {
        let provider = self
            .providers
            .get(model_id)
            .ok_or_else(|| ApiError::ProviderNotFound(model_id.to_string()))?;
        provider.chat_stream(request).await
    }

    /// Auto模式聊天 —— 根据任务条件自动选择模型
    ///
    /// # 参数
    /// - `condition`: 任务条件，用于选择最合适的模型
    /// - `request`: 聊天请求
    pub async fn auto_chat(
        &self,
        condition: &TaskCondition,
        request: ChatRequest,
    ) -> Result<ChatResponse, ApiError> {
        let model_id = self.resolve_model(condition)?;
        self.chat(&model_id, request).await
    }

    /// Auto模式流式聊天
    pub async fn auto_chat_stream(
        &self,
        condition: &TaskCondition,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamChunk, ApiError>>, ApiError> {
        let model_id = self.resolve_model(condition)?;
        self.chat_stream(&model_id, request).await
    }

    /// 列出所有已配置的模型信息
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, ApiError> {
        let mut all_models = Vec::new();
        for (_, provider) in &self.providers {
            match provider.list_models().await {
                Ok(mut models) => all_models.append(&mut models),
                Err(_) => {
                    // 如果API调用失败，添加默认模型信息
                    all_models.push(ModelInfo {
                        id: provider.name().to_string(),
                        name: provider.name().to_string(),
                        provider: provider.provider_type(),
                        context_length: Some(provider.max_context_length()),
                        supports_tools: provider.supports_tools(),
                        supports_vision: provider.supports_vision(),
                    });
                }
            }
        }
        Ok(all_models)
    }

    /// 获取指定Provider
    pub fn get_provider(&self, model_id: &str) -> Option<&dyn LLMProvider> {
        self.providers.get(model_id).map(|p| p.as_ref())
    }

    /// 获取所有已注册的model_id
    pub fn available_models(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// 获取模型注册表
    pub fn registry(&self) -> &ModelRegistry {
        &self.registry
    }

    /// 检查指定模型是否已配置
    pub fn has_model(&self, model_id: &str) -> bool {
        self.providers.contains_key(model_id)
    }

    /// 根据任务条件解析应使用的模型
    fn resolve_model(&self, condition: &TaskCondition) -> Result<String, ApiError> {
        use crate::agent::config::{Complexity, TaskType};

        // 1. 首先尝试基于任务类型的规则匹配
        let mut candidates: Vec<(String, u32)> = Vec::new();

        // 根据任务类型和复杂度选择模型
        if let Some(ref task_type) = condition.task_type {
            match task_type {
                TaskType::Code => {
                    // 代码任务优先选择Claude或GPT-4
                    candidates.push(("claude-3-5".to_string(), 100));
                    candidates.push(("gpt-4".to_string(), 90));
                    candidates.push(("gpt-4o".to_string(), 85));
                }
                TaskType::Image => {
                    // 图像任务需要视觉支持
                    candidates.push(("gpt-4o".to_string(), 100));
                    candidates.push(("gemini-1.5-pro".to_string(), 90));
                    candidates.push(("claude-3-5-sonnet-20241022".to_string(), 85));
                }
                TaskType::Long => {
                    // 长文本任务需要大上下文
                    candidates.push(("gemini-1.5-pro".to_string(), 100));
                    candidates.push(("claude-3-5-sonnet-20241022".to_string(), 90));
                    candidates.push(("gpt-4-turbo".to_string(), 80));
                }
                TaskType::Analysis => {
                    // 分析任务
                    candidates.push(("claude-3-5-sonnet-20241022".to_string(), 100));
                    candidates.push(("gpt-4o".to_string(), 95));
                    candidates.push(("gpt-4".to_string(), 85));
                }
                TaskType::Chat => {
                    // 简单对话可以用轻量模型
                    candidates.push(("gpt-4o-mini".to_string(), 100));
                    candidates.push(("gpt-4o".to_string(), 90));
                    candidates.push(("claude-3-5-haiku-20241022".to_string(), 85));
                }
            }
        }

        // 2. 根据复杂度调整
        if let Some(ref complexity) = condition.complexity {
            match complexity {
                Complexity::Simple => {
                    // 简单任务优先轻量模型
                    candidates.insert(0, ("gpt-4o-mini".to_string(), 110));
                    candidates.push(("ollama-local".to_string(), 60));
                }
                Complexity::Complex => {
                    // 复杂任务需要强模型
                    candidates.insert(0, ("claude-3-5-sonnet-20241022".to_string(), 110));
                    candidates.insert(1, ("gpt-4o".to_string(), 105));
                }
                _ => {}
            }
        }

        // 3. 根据上下文大小需求过滤
        if let Some(ref context_size) = condition.context_size {
            if let Some(max) = context_size.max {
                // 小上下文任务可以用本地模型
                if max <= 32000 {
                    candidates.push(("ollama-local".to_string(), 40));
                }
            }
            if let Some(min) = context_size.min {
                if min > 128000 {
                    // 超大上下文需要Gemini
                    candidates.insert(0, ("gemini-1.5-pro".to_string(), 120));
                }
            }
        }

        // 4. 根据可用性过滤，选择最高优先级的可用模型
        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        for (model_id, _) in candidates {
            if self.has_model(&model_id) {
                return Ok(model_id);
            }
            // 尝试用注册表解析完整模型ID
            if let Some(resolved) = self.registry.resolve_model_id(&model_id) {
                if self.has_model(&resolved) {
                    return Ok(resolved);
                }
            }
        }

        // 5. 回退到第一个可用的Provider
        if let Some(first) = self.providers.keys().next() {
            return Ok(first.clone());
        }

        Err(ApiError::Config("没有可用的模型Provider".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::config::{Complexity, ContextSizeRange, ModelConfig, TaskType};

    /// 测试UnifiedClient创建
    #[test]
    fn test_unified_client_creation() {
        let configs = vec![
            ModelConfig {
                id: "gpt-4".to_string(),
                name: "GPT-4".to_string(),
                provider: ConfigProviderType::OpenAI,
                api_key: Some("sk-test".to_string()),
                base_url: Some("https://api.openai.com/v1".to_string()),
                models_list: vec!["gpt-4".to_string()],
                default_model: "gpt-4".to_string(),
                timeout_ms: 60000,
            },
            ModelConfig {
                id: "claude".to_string(),
                name: "Claude".to_string(),
                provider: ConfigProviderType::Anthropic,
                api_key: Some("sk-ant-test".to_string()),
                base_url: None,
                models_list: vec!["claude-3-5-sonnet".to_string()],
                default_model: "claude-3-5-sonnet".to_string(),
                timeout_ms: 120000,
            },
        ];

        let client = UnifiedClient::new(&configs);
        assert!(client.is_ok());

        let client = client.unwrap();
        assert!(client.has_model("gpt-4"));
        assert!(client.has_model("claude"));
        assert!(!client.has_model("non-existent"));

        let models = client.available_models();
        assert_eq!(models.len(), 2);
    }

    /// 测试空配置创建
    #[test]
    fn test_unified_client_empty_config() {
        let configs: Vec<ModelConfig> = vec![];
        let client = UnifiedClient::new(&configs);
        assert!(client.is_ok());

        let client = client.unwrap();
        assert!(client.available_models().is_empty());
    }

    /// 测试模型解析（基于任务条件）
    #[test]
    fn test_resolve_model_code_task() {
        let configs = vec![
            ModelConfig {
                id: "claude-3-5-sonnet-20241022".to_string(),
                name: "Claude 3.5".to_string(),
                provider: ConfigProviderType::Anthropic,
                api_key: Some("key".to_string()),
                base_url: None,
                models_list: vec![],
                default_model: "claude-3-5-sonnet".to_string(),
                timeout_ms: 60000,
            },
            ModelConfig {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o-mini".to_string(),
                provider: ConfigProviderType::OpenAI,
                api_key: Some("key".to_string()),
                base_url: None,
                models_list: vec![],
                default_model: "gpt-4o-mini".to_string(),
                timeout_ms: 60000,
            },
        ];

        let client = UnifiedClient::new(&configs).expect("创建失败");

        // 代码任务应选中Claude
        let code_condition = TaskCondition {
            task_type: Some(TaskType::Code),
            complexity: None,
            context_size: None,
            required_capability: None,
        };
        let resolved = client.resolve_model(&code_condition);
        assert!(resolved.is_ok());
        assert_eq!(resolved.unwrap(), "claude-3-5-sonnet-20241022");
    }

    /// 测试简单对话任务回退到轻量模型
    #[test]
    fn test_resolve_model_chat_task() {
        let configs = vec![
            ModelConfig {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o-mini".to_string(),
                provider: ConfigProviderType::OpenAI,
                api_key: Some("key".to_string()),
                base_url: None,
                models_list: vec![],
                default_model: "gpt-4o-mini".to_string(),
                timeout_ms: 60000,
            },
            ModelConfig {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                provider: ConfigProviderType::OpenAI,
                api_key: Some("key".to_string()),
                base_url: None,
                models_list: vec![],
                default_model: "gpt-4o".to_string(),
                timeout_ms: 60000,
            },
        ];

        let client = UnifiedClient::new(&configs).expect("创建失败");

        let chat_condition = TaskCondition {
            task_type: Some(TaskType::Chat),
            complexity: None,
            context_size: None,
            required_capability: None,
        };
        let resolved = client.resolve_model(&chat_condition);
        assert!(resolved.is_ok());
        // 应该选中gpt-4o-mini（对于Chat任务优先级更高）
        assert_eq!(resolved.unwrap(), "gpt-4o-mini");
    }

    /// 测试回退到第一个可用模型
    #[test]
    fn test_resolve_model_fallback() {
        let configs = vec![ModelConfig {
            id: "only-model".to_string(),
            name: "唯一模型".to_string(),
            provider: ConfigProviderType::OpenAI,
            api_key: Some("key".to_string()),
            base_url: None,
            models_list: vec![],
            default_model: "gpt-4".to_string(),
            timeout_ms: 60000,
        }];

        let client = UnifiedClient::new(&configs).expect("创建失败");

        let any_condition = TaskCondition {
            task_type: Some(TaskType::Long),
            complexity: Some(Complexity::Complex),
            context_size: Some(ContextSizeRange {
                min: Some(200000),
                max: Some(500000),
            }),
            required_capability: None,
        };
        let resolved = client.resolve_model(&any_condition);
        assert!(resolved.is_ok());
        // 当没有完美匹配时，回退到第一个可用模型
        assert_eq!(resolved.unwrap(), "only-model");
    }

    /// 测试模型注册表访问
    #[test]
    fn test_registry_access() {
        let client = UnifiedClient::new(&[]).expect("创建失败");
        let registry = client.registry();
        assert!(registry.get_capability("gpt-4").is_some());
        assert!(registry.get_capability("gemini-1.5-pro").is_some());
    }

    /// 测试Provider获取
    #[test]
    fn test_get_provider() {
        let configs = vec![ModelConfig {
            id: "test-gpt".to_string(),
            name: "Test GPT".to_string(),
            provider: ConfigProviderType::OpenAI,
            api_key: Some("key".to_string()),
            base_url: None,
            models_list: vec![],
            default_model: "gpt-4".to_string(),
            timeout_ms: 60000,
        }];

        let client = UnifiedClient::new(&configs).expect("创建失败");
        let provider = client.get_provider("test-gpt");
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().name(), "Test GPT");
    }

    /// 测试注册自定义Provider（通过MockProvider模式）
    #[test]
    fn test_register_provider() {
        // 由于无法直接构造Box<dyn LLMProvider>的mock，我们测试基本功能
        let mut client = UnifiedClient::new(&[]).expect("创建失败");

        // 使用已有的OpenAIProvider作为"自定义Provider"
        let custom = OpenAIProvider::new("Custom", None, None, "custom-model");
        client.register_provider("custom", Box::new(custom));

        assert!(client.has_model("custom"));
        let provider = client.get_provider("custom");
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().name(), "Custom");
    }

    /// 测试没有Provider时的错误
    #[test]
    fn test_no_provider_error() {
        let client = UnifiedClient::new(&[]).expect("创建失败");

        let condition = TaskCondition {
            task_type: Some(TaskType::Code),
            complexity: None,
            context_size: None,
            required_capability: None,
        };

        let result = client.resolve_model(&condition);
        assert!(result.is_err());
        match result {
            Err(ApiError::Config(msg)) => {
                assert!(msg.contains("没有可用"));
            }
            _ => panic!("应返回Config错误"),
        }
    }
}
