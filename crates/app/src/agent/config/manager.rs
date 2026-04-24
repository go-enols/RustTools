use super::models::*;
use anyhow::Result;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// 配置管理器错误类型
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON解析错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("配置未找到: {0}")]
    NotFound(String),
    #[error("模型未找到: {0}")]
    ModelNotFound(String),
    #[error("Agent未找到: {0}")]
    AgentNotFound(String),
    #[error("无效的配置路径")]
    InvalidPath,
}

/// 配置管理器 —— 负责Agent配置的加载、保存和查询
pub struct ConfigManager {
    config_path: PathBuf,
    config: RwLock<AgentConfig>,
}

impl ConfigManager {
    /// 创建新的配置管理器，自动加载已有配置或创建默认配置
    pub fn new() -> Result<Self, ConfigError> {
        let config_path = Self::default_config_path()?;
        let config = if config_path.exists() {
            Self::load_from_path(&config_path)?
        } else {
            AgentConfig::default()
        };

        Ok(Self {
            config_path,
            config: RwLock::new(config),
        })
    }

    /// 从指定路径创建配置管理器
    pub fn with_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let config_path = path.as_ref().to_path_buf();
        let config = if config_path.exists() {
            Self::load_from_path(&config_path)?
        } else {
            AgentConfig::default()
        };

        Ok(Self {
            config_path,
            config: RwLock::new(config),
        })
    }

    /// 获取默认配置路径
    pub fn default_config_path() -> Result<PathBuf, ConfigError> {
        let data_dir = dirs::data_dir().ok_or(ConfigError::InvalidPath)?;
        let config_dir = data_dir.join("rusttools");
        std::fs::create_dir_all(&config_dir)?;
        Ok(config_dir.join("agent_config.json"))
    }

    /// 从文件系统加载配置
    pub fn load(&self) -> Result<AgentConfig, ConfigError> {
        Self::load_from_path(&self.config_path)
    }

    /// 保存配置到文件系统
    pub fn save(&self, config: &AgentConfig) -> Result<(), ConfigError> {
        // 确保父目录存在
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(config)?;
        std::fs::write(&self.config_path, json)?;
        // 更新内存中的配置
        *self.config.write() = config.clone();
        Ok(())
    }

    /// 获取当前配置的克隆副本
    pub fn get(&self) -> AgentConfig {
        self.config.read().clone()
    }

    /// 使用闭包更新配置并自动保存
    pub fn update(&self, f: impl FnOnce(&mut AgentConfig)) -> Result<(), ConfigError> {
        let mut config = self.config.write();
        f(&mut config);
        drop(config); // 释放写锁
        let config = self.get();
        self.save(&config)
    }

    /// 根据ID获取模型配置
    pub fn get_model(&self, id: &str) -> Option<ModelConfig> {
        self.config.read().models.iter().find(|m| m.id == id).cloned()
    }

    /// 根据ID获取Agent定义
    pub fn get_agent(&self, id: &str) -> Option<AgentDefinition> {
        self.config.read().agents.iter().find(|a| a.id == id).cloned()
    }

    /// 根据任务条件解析应使用的模型（Auto路由）
    pub fn resolve_model_for_task(&self, task: &TaskCondition) -> Option<String> {
        let config = self.config.read();

        // 按优先级排序规则（高优先级优先）
        let mut rules: Vec<_> = config.auto_router_rules.iter().collect();
        rules.sort_by_key(|r| std::cmp::Reverse(r.priority));

        for rule in rules {
            if Self::condition_matches(&rule.condition, task) {
                return Some(rule.target_model.clone());
            }
        }

        // 没有匹配规则时返回active_model
        Some(config.active_model.clone())
    }

    /// 获取配置路径
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    // 内部辅助方法

    fn load_from_path(path: &Path) -> Result<AgentConfig, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: AgentConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 判断规则条件是否匹配任务条件
    fn condition_matches(rule: &TaskCondition, task: &TaskCondition) -> bool {
        // 如果规则中指定了字段，则任务必须满足该字段
        if let Some(ref rule_type) = rule.task_type {
            if let Some(ref task_type) = task.task_type {
                if rule_type != task_type {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref rule_complexity) = rule.complexity {
            if let Some(ref task_complexity) = task.complexity {
                if rule_complexity != task_complexity {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref rule_size) = rule.context_size {
            if let Some(ref task_size) = task.context_size {
                // 检查min约束：任务的min必须大于等于规则min
                if let Some(rule_min) = rule_size.min {
                    let task_estimate = task_size.min.unwrap_or(0);
                    if task_estimate < rule_min {
                        return false;
                    }
                }
                // 检查max约束：任务必须小于等于规则max
                if let Some(rule_max) = rule_size.max {
                    let task_estimate = task_size.max.unwrap_or(usize::MAX);
                    if task_estimate > rule_max {
                        return false;
                    }
                }
            }
        }

        if let Some(ref rule_cap) = rule.required_capability {
            if let Some(ref task_cap) = task.required_capability {
                if !task_cap.contains(rule_cap) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 测试配置保存与加载
    #[test]
    fn test_save_and_load_config() {
        let temp_dir = tempfile::tempdir().expect("创建临时目录失败");
        let config_path = temp_dir.path().join("test_agent_config.json");

        let manager = ConfigManager::with_path(&config_path).expect("创建管理器失败");

        let config = AgentConfig {
            version: "1.0".to_string(),
            active_model: "gpt-4".to_string(),
            models: vec![ModelConfig {
                id: "gpt-4".to_string(),
                name: "GPT-4".to_string(),
                provider: ProviderType::OpenAI,
                api_key: Some("sk-test".to_string()),
                base_url: Some("https://api.openai.com/v1".to_string()),
                models_list: vec!["gpt-4".to_string()],
                default_model: "gpt-4".to_string(),
                timeout_ms: 60000,
            }],
            auto_router_rules: vec![],
            agents: vec![AgentDefinition {
                id: "default".to_string(),
                name: "通用助手".to_string(),
                description: "测试助手".to_string(),
                system_prompt: "你是测试助手".to_string(),
                model_id: "gpt-4".to_string(),
                tools: vec![],
                mcp_servers: vec![],
                auto_mode: true,
                max_iterations: 10,
                allowed_directories: vec![],
                capabilities: vec![],
            }],
            mcp_servers: vec![],
            skills: vec![],
        };

        manager.save(&config).expect("保存配置失败");
        assert!(config_path.exists());

        let loaded = manager.load().expect("加载配置失败");
        assert_eq!(loaded.version, "1.0");
        assert_eq!(loaded.active_model, "gpt-4");
        assert_eq!(loaded.models.len(), 1);
        assert_eq!(loaded.agents.len(), 1);
    }

    /// 测试更新配置
    #[test]
    fn test_update_config() {
        let temp_dir = tempfile::tempdir().expect("创建临时目录失败");
        let config_path = temp_dir.path().join("test_agent_config.json");

        let manager = ConfigManager::with_path(&config_path).expect("创建管理器失败");

        manager
            .update(|config| {
                config.active_model = "claude-3-5".to_string();
                config.models.push(ModelConfig {
                    id: "claude-3-5".to_string(),
                    name: "Claude 3.5".to_string(),
                    provider: ProviderType::Anthropic,
                    api_key: Some("sk-ant-test".to_string()),
                    base_url: None,
                    models_list: vec!["claude-3-5-sonnet".to_string()],
                    default_model: "claude-3-5-sonnet".to_string(),
                    timeout_ms: 120000,
                });
            })
            .expect("更新配置失败");

        let config = manager.get();
        assert_eq!(config.active_model, "claude-3-5");
        assert_eq!(config.models.len(), 1);
    }

    /// 测试获取模型和Agent
    #[test]
    fn test_get_model_and_agent() {
        let temp_dir = tempfile::tempdir().expect("创建临时目录失败");
        let config_path = temp_dir.path().join("test_agent_config.json");

        let mut config = AgentConfig::default();
        config.models.push(ModelConfig {
            id: "gpt-4".to_string(),
            name: "GPT-4".to_string(),
            provider: ProviderType::OpenAI,
            api_key: Some("sk-test".to_string()),
            base_url: Some("https://api.openai.com/v1".to_string()),
            models_list: vec!["gpt-4".to_string()],
            default_model: "gpt-4".to_string(),
            timeout_ms: 60000,
        });
        config.agents.push(AgentDefinition {
            id: "coder".to_string(),
            name: "代码助手".to_string(),
            description: "专门编写代码".to_string(),
            system_prompt: "你是代码专家".to_string(),
            model_id: "gpt-4".to_string(),
            tools: vec!["terminal".to_string()],
            mcp_servers: vec![],
            auto_mode: true,
            max_iterations: 20,
            allowed_directories: vec!["/tmp".to_string()],
            capabilities: vec![],
        });

        let manager = ConfigManager::with_path(&config_path).expect("创建管理器失败");
        manager.save(&config).expect("保存配置失败");

        let model = manager.get_model("gpt-4");
        assert!(model.is_some());
        assert_eq!(model.unwrap().name, "GPT-4");

        let model_not_found = manager.get_model("non-existent");
        assert!(model_not_found.is_none());

        let agent = manager.get_agent("coder");
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().name, "代码助手");
    }

    /// 测试Auto路由规则匹配
    #[test]
    fn test_auto_router_resolve() {
        let temp_dir = tempfile::tempdir().expect("创建临时目录失败");
        let config_path = temp_dir.path().join("test_agent_config.json");

        let mut config = AgentConfig::default();
        config.active_model = "default-model".to_string();
        config.auto_router_rules = vec![
            RouterRule {
                condition: TaskCondition {
                    task_type: Some(TaskType::Code),
                    complexity: Some(Complexity::Complex),
                    context_size: None,
                    required_capability: None,
                },
                target_model: "claude-3-5".to_string(),
                priority: 100,
            },
            RouterRule {
                condition: TaskCondition {
                    task_type: Some(TaskType::Image),
                    complexity: None,
                    context_size: None,
                    required_capability: None,
                },
                target_model: "gpt-4".to_string(),
                priority: 90,
            },
            RouterRule {
                condition: TaskCondition {
                    task_type: None,
                    complexity: None,
                    context_size: Some(ContextSizeRange {
                        min: None,
                        max: Some(8000),
                    }),
                    required_capability: None,
                },
                target_model: "ollama-local".to_string(),
                priority: 50,
            },
        ];

        let manager = ConfigManager::with_path(&config_path).expect("创建管理器失败");
        manager.save(&config).expect("保存配置失败");

        // 测试代码复杂任务 -> claude-3-5
        let code_task = TaskCondition {
            task_type: Some(TaskType::Code),
            complexity: Some(Complexity::Complex),
            context_size: None,
            required_capability: None,
        };
        assert_eq!(
            manager.resolve_model_for_task(&code_task),
            Some("claude-3-5".to_string())
        );

        // 测试图像任务 -> gpt-4
        let image_task = TaskCondition {
            task_type: Some(TaskType::Image),
            complexity: Some(Complexity::Simple),
            context_size: None,
            required_capability: None,
        };
        assert_eq!(
            manager.resolve_model_for_task(&image_task),
            Some("gpt-4".to_string())
        );

        // 测试小上下文任务 -> ollama-local
        let small_context_task = TaskCondition {
            task_type: Some(TaskType::Chat),
            complexity: Some(Complexity::Simple),
            context_size: Some(ContextSizeRange {
                min: Some(100),
                max: Some(5000),
            }),
            required_capability: None,
        };
        assert_eq!(
            manager.resolve_model_for_task(&small_context_task),
            Some("ollama-local".to_string())
        );

        // 测试无匹配规则 -> 使用active_model
        let unmatched_task = TaskCondition {
            task_type: Some(TaskType::Long),
            complexity: Some(Complexity::Complex),
            context_size: Some(ContextSizeRange {
                min: Some(100000),
                max: Some(200000),
            }),
            required_capability: None,
        };
        assert_eq!(
            manager.resolve_model_for_task(&unmatched_task),
            Some("default-model".to_string())
        );
    }

    /// 测试路由规则优先级
    #[test]
    fn test_router_priority() {
        let temp_dir = tempfile::tempdir().expect("创建临时目录失败");
        let config_path = temp_dir.path().join("test_agent_config.json");

        let mut config = AgentConfig::default();
        // 低优先级通用规则先添加
        config.auto_router_rules.push(RouterRule {
            condition: TaskCondition {
                task_type: Some(TaskType::Code),
                complexity: None,
                context_size: None,
                required_capability: None,
            },
            target_model: "low-priority".to_string(),
            priority: 10,
        });
        // 高优先级复杂代码规则后添加
        config.auto_router_rules.push(RouterRule {
            condition: TaskCondition {
                task_type: Some(TaskType::Code),
                complexity: Some(Complexity::Complex),
                context_size: None,
                required_capability: None,
            },
            target_model: "high-priority".to_string(),
            priority: 100,
        });

        let manager = ConfigManager::with_path(&config_path).expect("创建管理器失败");
        manager.save(&config).expect("保存配置失败");

        let complex_code = TaskCondition {
            task_type: Some(TaskType::Code),
            complexity: Some(Complexity::Complex),
            context_size: None,
            required_capability: None,
        };
        // 应该匹配高优先级规则
        assert_eq!(
            manager.resolve_model_for_task(&complex_code),
            Some("high-priority".to_string())
        );
    }

    /// 测试默认配置路径生成
    #[test]
    fn test_default_config_path() {
        let path = ConfigManager::default_config_path();
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("rusttools"));
        assert!(path.to_string_lossy().contains("agent_config.json"));
    }
}
