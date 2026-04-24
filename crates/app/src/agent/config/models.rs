use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 全局Agent配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    pub version: String,
    pub active_model: String,
    pub models: Vec<ModelConfig>,
    pub auto_router_rules: Vec<RouterRule>,
    pub agents: Vec<AgentDefinition>,
    pub mcp_servers: Vec<McpServerConfig>,
    pub skills: Vec<SkillConfig>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            active_model: "auto".to_string(),
            models: vec![],
            auto_router_rules: vec![],
            agents: vec![],
            mcp_servers: vec![],
            skills: vec![],
        }
    }
}

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub provider: ProviderType,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub models_list: Vec<String>,
    pub default_model: String,
    pub timeout_ms: u64,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Default Model".to_string(),
            provider: ProviderType::OpenAICompatible,
            api_key: None,
            base_url: None,
            models_list: vec![],
            default_model: "default".to_string(),
            timeout_ms: 60000,
        }
    }
}

/// Provider类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
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

/// 自动路由规则
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouterRule {
    pub condition: TaskCondition,
    pub target_model: String,
    pub priority: u32,
}

/// 任务条件（用于Auto路由）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct TaskCondition {
    pub task_type: Option<TaskType>,
    pub complexity: Option<Complexity>,
    pub context_size: Option<ContextSizeRange>,
    pub required_capability: Option<String>,
}

/// 任务类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Chat,
    Code,
    Analysis,
    Image,
    Long,
}

/// 任务复杂度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Complexity {
    Simple,
    Medium,
    Complex,
}

/// 上下文大小范围
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextSizeRange {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

/// Agent定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub model_id: String,
    pub tools: Vec<String>,
    pub mcp_servers: Vec<String>,
    pub auto_mode: bool,
    pub max_iterations: u32,
    pub allowed_directories: Vec<String>,
    pub capabilities: Vec<String>,
}

impl Default for AgentDefinition {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            system_prompt: String::new(),
            model_id: "auto".to_string(),
            tools: vec![],
            mcp_servers: vec![],
            auto_mode: true,
            max_iterations: 50,
            allowed_directories: vec![],
            capabilities: vec![],
        }
    }
}

/// MCP服务器配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransportType,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: Option<String>,
    pub enabled: bool,
}

/// MCP传输类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum McpTransportType {
    Stdio,
    Sse,
    Websocket,
}

/// Skill配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub parameters: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试配置序列化与反序列化
    #[test]
    fn test_config_serialization() {
        let config = AgentConfig {
            version: "1.0".to_string(),
            active_model: "auto".to_string(),
            models: vec![ModelConfig {
                id: "gpt-4".to_string(),
                name: "GPT-4".to_string(),
                provider: ProviderType::OpenAI,
                api_key: Some("sk-test".to_string()),
                base_url: Some("https://api.openai.com/v1".to_string()),
                models_list: vec!["gpt-4".to_string(), "gpt-4-turbo".to_string()],
                default_model: "gpt-4".to_string(),
                timeout_ms: 60000,
            }],
            auto_router_rules: vec![RouterRule {
                condition: TaskCondition {
                    task_type: Some(TaskType::Code),
                    complexity: Some(Complexity::Complex),
                    context_size: None,
                    required_capability: None,
                },
                target_model: "claude-3-5".to_string(),
                priority: 100,
            }],
            agents: vec![AgentDefinition {
                id: "default".to_string(),
                name: "通用助手".to_string(),
                description: "通用AI助手".to_string(),
                system_prompt: "你是一个有用的AI助手".to_string(),
                model_id: "auto".to_string(),
                tools: vec!["fs_read".to_string(), "terminal".to_string()],
                mcp_servers: vec![],
                auto_mode: true,
                max_iterations: 50,
                allowed_directories: vec![],
                capabilities: vec![],
            }],
            mcp_servers: vec![McpServerConfig {
                name: "filesystem".to_string(),
                transport: McpTransportType::Stdio,
                command: Some("npx".to_string()),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()],
                env: HashMap::new(),
                url: None,
                enabled: true,
            }],
            skills: vec![SkillConfig {
                id: "translate".to_string(),
                name: "翻译".to_string(),
                description: "翻译文本".to_string(),
                enabled: true,
                parameters: serde_json::json!({"target_lang": "zh"}),
            }],
        };

        let json = serde_json::to_string_pretty(&config).expect("序列化失败");
        let deserialized: AgentConfig = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(config, deserialized);
    }

    /// 测试ProviderType序列化
    #[test]
    fn test_provider_type_serialization() {
        let providers = vec![
            ProviderType::OpenAI,
            ProviderType::Anthropic,
            ProviderType::Gemini,
            ProviderType::Ollama,
            ProviderType::OpenAICompatible,
        ];

        for provider in providers {
            let json = serde_json::to_string(&provider).expect("序列化失败");
            let deserialized: ProviderType = serde_json::from_str(&json).expect("反序列化失败");
            assert_eq!(provider, deserialized);
        }
    }

    /// 测试默认配置
    #[test]
    fn test_default_config() {
        let config = AgentConfig::default();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.active_model, "auto");
        assert!(config.models.is_empty());
    }

    /// 测试TaskCondition匹配逻辑
    #[test]
    fn test_task_condition_matching() {
        let rule = TaskCondition {
            task_type: Some(TaskType::Code),
            complexity: Some(Complexity::Complex),
            context_size: Some(ContextSizeRange {
                min: Some(1000),
                max: Some(8000),
            }),
            required_capability: Some("coding".to_string()),
        };

        // 测试部分匹配
        let partial = TaskCondition {
            task_type: Some(TaskType::Code),
            complexity: None,
            context_size: None,
            required_capability: None,
        };

        // 这里的匹配逻辑在manager.rs中实现，这里只测试结构
        assert!(partial.task_type.is_some());
        assert!(rule.task_type.is_some());
        assert_eq!(partial.task_type, rule.task_type);
    }
}
