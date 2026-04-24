use super::provider::ProviderType;
use std::collections::HashMap;

/// 模型能力信息
#[derive(Debug, Clone)]
pub struct ModelCapability {
    pub id: String,
    pub context_length: usize,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub cost_per_1k_input: Option<f64>,
    pub cost_per_1k_output: Option<f64>,
}

/// 模型注册表 —— 管理已知模型的能力信息
pub struct ModelRegistry {
    models: HashMap<String, ModelCapability>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRegistry {
    /// 创建预置了常见模型能力的注册表
    pub fn new() -> Self {
        let mut models = HashMap::new();

        // OpenAI模型
        models.insert(
            "gpt-4".to_string(),
            ModelCapability {
                id: "gpt-4".to_string(),
                context_length: 8192,
                supports_tools: true,
                supports_vision: false,
                cost_per_1k_input: Some(0.03),
                cost_per_1k_output: Some(0.06),
            },
        );
        models.insert(
            "gpt-4-turbo".to_string(),
            ModelCapability {
                id: "gpt-4-turbo".to_string(),
                context_length: 128000,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(0.01),
                cost_per_1k_output: Some(0.03),
            },
        );
        models.insert(
            "gpt-4o".to_string(),
            ModelCapability {
                id: "gpt-4o".to_string(),
                context_length: 128000,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(0.005),
                cost_per_1k_output: Some(0.015),
            },
        );
        models.insert(
            "gpt-4o-mini".to_string(),
            ModelCapability {
                id: "gpt-4o-mini".to_string(),
                context_length: 128000,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(0.00015),
                cost_per_1k_output: Some(0.0006),
            },
        );
        models.insert(
            "gpt-3.5-turbo".to_string(),
            ModelCapability {
                id: "gpt-3.5-turbo".to_string(),
                context_length: 16385,
                supports_tools: true,
                supports_vision: false,
                cost_per_1k_input: Some(0.0005),
                cost_per_1k_output: Some(0.0015),
            },
        );

        // Anthropic模型
        models.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            ModelCapability {
                id: "claude-3-5-sonnet-20241022".to_string(),
                context_length: 200000,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(0.003),
                cost_per_1k_output: Some(0.015),
            },
        );
        models.insert(
            "claude-3-5-haiku-20241022".to_string(),
            ModelCapability {
                id: "claude-3-5-haiku-20241022".to_string(),
                context_length: 200000,
                supports_tools: true,
                supports_vision: false,
                cost_per_1k_input: Some(0.0008),
                cost_per_1k_output: Some(0.004),
            },
        );
        models.insert(
            "claude-3-opus-20240229".to_string(),
            ModelCapability {
                id: "claude-3-opus-20240229".to_string(),
                context_length: 200000,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(0.015),
                cost_per_1k_output: Some(0.075),
            },
        );

        // Gemini模型
        models.insert(
            "gemini-1.5-pro".to_string(),
            ModelCapability {
                id: "gemini-1.5-pro".to_string(),
                context_length: 1000000,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(0.0035),
                cost_per_1k_output: Some(0.0105),
            },
        );
        models.insert(
            "gemini-1.5-flash".to_string(),
            ModelCapability {
                id: "gemini-1.5-flash".to_string(),
                context_length: 1000000,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(0.00035),
                cost_per_1k_output: Some(0.00105),
            },
        );
        models.insert(
            "gemini-pro".to_string(),
            ModelCapability {
                id: "gemini-pro".to_string(),
                context_length: 32768,
                supports_tools: true,
                supports_vision: false,
                cost_per_1k_input: Some(0.0005),
                cost_per_1k_output: Some(0.0015),
            },
        );

        // Ollama模型（本地，成本为0）
        models.insert(
            "llama3.2".to_string(),
            ModelCapability {
                id: "llama3.2".to_string(),
                context_length: 32768,
                supports_tools: true,
                supports_vision: false,
                cost_per_1k_input: Some(0.0),
                cost_per_1k_output: Some(0.0),
            },
        );
        models.insert(
            "llama3.1".to_string(),
            ModelCapability {
                id: "llama3.1".to_string(),
                context_length: 128000,
                supports_tools: true,
                supports_vision: false,
                cost_per_1k_input: Some(0.0),
                cost_per_1k_output: Some(0.0),
            },
        );
        models.insert(
            "mistral".to_string(),
            ModelCapability {
                id: "mistral".to_string(),
                context_length: 32768,
                supports_tools: true,
                supports_vision: false,
                cost_per_1k_input: Some(0.0),
                cost_per_1k_output: Some(0.0),
            },
        );
        models.insert(
            "qwen2.5".to_string(),
            ModelCapability {
                id: "qwen2.5".to_string(),
                context_length: 128000,
                supports_tools: true,
                supports_vision: false,
                cost_per_1k_input: Some(0.0),
                cost_per_1k_output: Some(0.0),
            },
        );

        Self { models }
    }

    /// 根据模型ID获取能力信息
    pub fn get_capability(&self, model_id: &str) -> Option<&ModelCapability> {
        self.models.get(model_id)
    }

    /// 注册自定义模型能力
    pub fn register(&mut self, capability: ModelCapability) {
        self.models.insert(capability.id.clone(), capability);
    }

    /// 获取所有模型能力
    pub fn all_capabilities(&self) -> Vec<&ModelCapability> {
        self.models.values().collect()
    }

    /// 获取支持工具调用的模型
    pub fn models_with_tools(&self) -> Vec<&ModelCapability> {
        self.models
            .values()
            .filter(|m| m.supports_tools)
            .collect()
    }

    /// 获取支持视觉输入的模型
    pub fn models_with_vision(&self) -> Vec<&ModelCapability> {
        self.models
            .values()
            .filter(|m| m.supports_vision)
            .collect()
    }

    /// 获取满足最小上下文长度要求的模型
    pub fn models_with_min_context(&self, min_context: usize) -> Vec<&ModelCapability> {
        self.models
            .values()
            .filter(|m| m.context_length >= min_context)
            .collect()
    }

    /// 根据任务需求推荐模型
    ///
    /// # 参数
    /// - `needs_tools`: 是否需要工具调用
    /// - `needs_vision`: 是否需要视觉输入
    /// - `min_context`: 最小上下文长度要求
    /// - `prefer_cheap`: 是否优先选择低成本模型
    pub fn recommend_model(
        &self,
        needs_tools: bool,
        needs_vision: bool,
        min_context: usize,
        prefer_cheap: bool,
    ) -> Option<&ModelCapability> {
        let mut candidates: Vec<_> = self
            .models
            .values()
            .filter(|m| {
                (!needs_tools || m.supports_tools)
                    && (!needs_vision || m.supports_vision)
                    && m.context_length >= min_context
            })
            .collect();

        if prefer_cheap {
            candidates.sort_by(|a, b| {
                let cost_a = a.cost_per_1k_input.unwrap_or(f64::MAX);
                let cost_b = b.cost_per_1k_input.unwrap_or(f64::MAX);
                cost_a.partial_cmp(&cost_b).unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            // 优先选择上下文更大的模型
            candidates.sort_by(|a, b| b.context_length.cmp(&a.context_length));
        }

        candidates.first().copied()
    }

    /// 查找最佳匹配的模型ID（支持部分匹配）
    pub fn resolve_model_id(&self, model_id: &str) -> Option<String> {
        // 首先精确匹配
        if self.models.contains_key(model_id) {
            return Some(model_id.to_string());
        }

        // 然后前缀匹配
        let matches: Vec<_> = self
            .models
            .keys()
            .filter(|k| k.starts_with(model_id) || model_id.starts_with(k.as_str()))
            .collect();

        // 返回最长匹配（最具体的）
        matches.into_iter().max_by_key(|k| k.len()).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试创建注册表
    #[test]
    fn test_registry_creation() {
        let registry = ModelRegistry::new();
        let caps = registry.all_capabilities();
        assert!(!caps.is_empty());
        // 检查预置模型存在
        assert!(registry.get_capability("gpt-4").is_some());
        assert!(registry.get_capability("claude-3-5-sonnet-20241022").is_some());
    }

    /// 测试获取模型能力
    #[test]
    fn test_get_capability() {
        let registry = ModelRegistry::new();
        let gpt4 = registry.get_capability("gpt-4").expect("gpt-4应该存在");
        assert_eq!(gpt4.context_length, 8192);
        assert!(gpt4.supports_tools);
        assert!(!gpt4.supports_vision);
        assert_eq!(gpt4.cost_per_1k_input, Some(0.03));
    }

    /// 测试注册自定义模型
    #[test]
    fn test_register_custom_model() {
        let mut registry = ModelRegistry::new();
        registry.register(ModelCapability {
            id: "custom-model".to_string(),
            context_length: 64000,
            supports_tools: true,
            supports_vision: false,
            cost_per_1k_input: Some(0.001),
            cost_per_1k_output: Some(0.002),
        });

        let model = registry.get_capability("custom-model").expect("自定义模型应存在");
        assert_eq!(model.context_length, 64000);
    }

    /// 测试工具支持过滤
    #[test]
    fn test_models_with_tools() {
        let registry = ModelRegistry::new();
        let tool_models = registry.models_with_tools();
        assert!(!tool_models.is_empty());
        for model in &tool_models {
            assert!(model.supports_tools, "模型 {} 应该支持工具", model.id);
        }
    }

    /// 测试视觉支持过滤
    #[test]
    fn test_models_with_vision() {
        let registry = ModelRegistry::new();
        let vision_models = registry.models_with_vision();
        assert!(!vision_models.is_empty());
        for model in &vision_models {
            assert!(model.supports_vision, "模型 {} 应该支持视觉", model.id);
        }
    }

    /// 测试上下文长度过滤
    #[test]
    fn test_models_with_min_context() {
        let registry = ModelRegistry::new();
        let large_context = registry.models_with_min_context(100000);
        assert!(!large_context.is_empty());
        for model in &large_context {
            assert!(
                model.context_length >= 100000,
                "模型 {} 的上下文应 >= 100000",
                model.id
            );
        }
    }

    /// 测试模型推荐（低成本优先）
    #[test]
    fn test_recommend_model_cheap() {
        let registry = ModelRegistry::new();
        let model = registry
            .recommend_model(true, false, 8192, true)
            .expect("应找到推荐模型");
        // GPT-4o-mini应该是工具支持模型中最便宜的之一
        assert!(model.supports_tools);
        assert!(model.context_length >= 8192);
    }

    /// 测试模型推荐（高性能优先）
    #[test]
    fn test_recommend_model_performance() {
        let registry = ModelRegistry::new();
        let model = registry
            .recommend_model(true, true, 128000, false)
            .expect("应找到推荐模型");
        assert!(model.supports_tools);
        assert!(model.supports_vision);
        assert!(model.context_length >= 128000);
    }

    /// 测试模型ID解析
    #[test]
    fn test_resolve_model_id() {
        let registry = ModelRegistry::new();
        // 精确匹配
        assert_eq!(
            registry.resolve_model_id("gpt-4"),
            Some("gpt-4".to_string())
        );
        // 前缀匹配
        assert_eq!(
            registry.resolve_model_id("claude-3-5-sonnet"),
            Some("claude-3-5-sonnet-20241022".to_string())
        );
        // 不存在模型
        assert!(registry.resolve_model_id("non-existent-model").is_none());
    }

    /// 测试Gemini模型能力
    #[test]
    fn test_gemini_capabilities() {
        let registry = ModelRegistry::new();
        let gemini = registry
            .get_capability("gemini-1.5-pro")
            .expect("gemini-1.5-pro应存在");
        assert_eq!(gemini.context_length, 1000000);
        assert!(gemini.supports_tools);
        assert!(gemini.supports_vision);
    }

    /// 测试Ollama本地模型（成本为0）
    #[test]
    fn test_ollama_zero_cost() {
        let registry = ModelRegistry::new();
        let llama = registry.get_capability("llama3.2").expect("llama3.2应存在");
        assert_eq!(llama.cost_per_1k_input, Some(0.0));
        assert_eq!(llama.cost_per_1k_output, Some(0.0));
    }
}
