//! Agent定义
//!
//! Agent是AI助手的核心配置单元，包含模型配置、系统提示词、
//! 可用工具列表和特殊能力标记。

use super::super::config::ModelConfig;
use serde::{Deserialize, Serialize};

// ============================================================================
// Agent能力枚举
// ============================================================================

/// Agent特殊能力标记
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    /// 代码生成能力
    CodeGeneration,
    /// 代码审查能力
    CodeReview,
    /// 测试编写能力
    Testing,
    /// 文档编写能力
    Documentation,
    /// 分析能力
    Analysis,
    /// 任务规划能力
    Planning,
    /// 自定义能力
    Custom(String),
}

impl Capability {
    /// 转换为字符串
    pub fn as_str(&self) -> String {
        match self {
            Capability::CodeGeneration => "code_generation".to_string(),
            Capability::CodeReview => "code_review".to_string(),
            Capability::Testing => "testing".to_string(),
            Capability::Documentation => "documentation".to_string(),
            Capability::Analysis => "analysis".to_string(),
            Capability::Planning => "planning".to_string(),
            Capability::Custom(s) => s.clone(),
        }
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> Self {
        match s {
            "code_generation" => Capability::CodeGeneration,
            "code_review" => Capability::CodeReview,
            "testing" => Capability::Testing,
            "documentation" => Capability::Documentation,
            "analysis" => Capability::Analysis,
            "planning" => Capability::Planning,
            _ => Capability::Custom(s.to_string()),
        }
    }
}

// ============================================================================
// Agent定义
// ============================================================================

/// Agent信息摘要（用于列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tool_count: usize,
    pub auto_mode: bool,
    pub capabilities: Vec<String>,
}

/// Agent定义（用于创建/更新Agent）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentDefinition {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub model_config: ModelConfig,
    pub tools: Vec<String>,
    pub mcp_servers: Vec<String>,
    pub auto_mode: bool,
    pub max_iterations: u32,
    pub allowed_directories: Vec<String>,
    pub capabilities: Vec<Capability>,
}

/// Agent完整结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Agent唯一ID
    pub id: String,
    /// Agent名称
    pub name: String,
    /// Agent描述
    pub description: String,
    /// 系统提示词
    pub system_prompt: String,
    /// 模型配置
    pub model_config: ModelConfig,
    /// 可用工具ID列表
    pub tools: Vec<String>,
    /// 关联的MCP服务器
    pub mcp_servers: Vec<String>,
    /// 是否允许自动执行工具
    pub auto_mode: bool,
    /// 最大迭代次数
    pub max_iterations: u32,
    /// 允许访问的目录列表
    pub allowed_directories: Vec<String>,
    /// 特殊能力标记
    pub capabilities: Vec<Capability>,
}

impl Agent {
    /// 创建新的Agent
    pub fn new(id: impl Into<String>, def: AgentDefinition) -> Self {
        let id_str: String = id.into();
        let capabilities_str = def
            .capabilities
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        log::info!(
            "创建Agent '{}' (id={}, tools=[{}], capabilities=[{}])",
            def.name,
            id_str,
            def.tools.join(", "),
            capabilities_str
        );

        Self {
            id: id_str,
            name: def.name,
            description: def.description,
            system_prompt: def.system_prompt,
            model_config: def.model_config,
            tools: def.tools,
            mcp_servers: def.mcp_servers,
            auto_mode: def.auto_mode,
            max_iterations: def.max_iterations,
            allowed_directories: def.allowed_directories,
            capabilities: def.capabilities,
        }
    }

    /// 获取Agent信息摘要
    pub fn to_info(&self) -> AgentInfo {
        AgentInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            tool_count: self.tools.len(),
            auto_mode: self.auto_mode,
            capabilities: self.capabilities.iter().map(|c| c.as_str()).collect(),
        }
    }

    /// 更新Agent配置
    pub fn update(&mut self, def: AgentDefinition) {
        self.name = def.name;
        self.description = def.description;
        self.system_prompt = def.system_prompt;
        self.model_config = def.model_config;
        self.tools = def.tools;
        self.mcp_servers = def.mcp_servers;
        self.auto_mode = def.auto_mode;
        self.max_iterations = def.max_iterations;
        self.allowed_directories = def.allowed_directories;
        self.capabilities = def.capabilities;
    }

    /// 检查Agent是否拥有指定能力
    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }
}

// ============================================================================
// Agent错误类型
// ============================================================================

/// Agent操作错误
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Agent未找到: {0}")]
    AgentNotFound(String),
    #[error("Agent已存在: {0}")]
    AgentAlreadyExists(String),
    #[error("配置错误: {0}")]
    ConfigError(String),
    #[error("执行错误: {0}")]
    ExecutionError(String),
    #[error("工具调用错误: {0}")]
    ToolCallError(String),
    #[error("会话错误: {0}")]
    SessionError(String),
    #[error("最大迭代次数 exceeded: {0}")]
    MaxIterationsExceeded(String),
    #[error("{0}")]
    Other(String),
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let def = AgentDefinition {
            name: "测试Agent".to_string(),
            description: "用于测试".to_string(),
            system_prompt: "你是一个测试助手".to_string(),
            model_config: ModelConfig {
                id: "test-model".to_string(),
                name: "Test Model".to_string(),
                ..Default::default()
            },
            tools: vec!["fs_read".to_string(), "terminal".to_string()],
            auto_mode: true,
            max_iterations: 50,
            capabilities: vec![Capability::CodeGeneration, Capability::Analysis],
            ..Default::default()
        };

        let agent = Agent::new("test-001", def);

        assert_eq!(agent.id, "test-001");
        assert_eq!(agent.name, "测试Agent");
        assert_eq!(agent.tools.len(), 2);
        assert!(agent.auto_mode);
        assert_eq!(agent.max_iterations, 50);
    }

    #[test]
    fn test_agent_to_info() {
        let def = AgentDefinition {
            name: "信息测试".to_string(),
            description: "测试信息转换".to_string(),
            tools: vec!["a".to_string(), "b".to_string()],
            capabilities: vec![Capability::Testing],
            ..Default::default()
        };

        let agent = Agent::new("info-test", def);
        let info = agent.to_info();

        assert_eq!(info.id, "info-test");
        assert_eq!(info.name, "信息测试");
        assert_eq!(info.tool_count, 2);
        assert!(info.capabilities.contains(&"testing".to_string()));
    }

    #[test]
    fn test_agent_update() {
        let def1 = AgentDefinition {
            name: "旧名称".to_string(),
            description: "旧描述".to_string(),
            system_prompt: "旧提示词".to_string(),
            ..Default::default()
        };

        let mut agent = Agent::new("update-test", def1);

        let def2 = AgentDefinition {
            name: "新名称".to_string(),
            description: "新描述".to_string(),
            system_prompt: "新提示词".to_string(),
            tools: vec!["new_tool".to_string()],
            auto_mode: false,
            max_iterations: 100,
            ..Default::default()
        };

        agent.update(def2);

        assert_eq!(agent.name, "新名称");
        assert_eq!(agent.description, "新描述");
        assert_eq!(agent.system_prompt, "新提示词");
        assert_eq!(agent.tools, vec!["new_tool"]);
        assert!(!agent.auto_mode);
        assert_eq!(agent.max_iterations, 100);
    }

    #[test]
    fn test_capability_as_str() {
        assert_eq!(Capability::CodeGeneration.as_str(), "code_generation");
        assert_eq!(Capability::Custom("special".to_string()).as_str(), "special");
    }

    #[test]
    fn test_capability_from_str() {
        assert_eq!(
            Capability::from_str("code_generation"),
            Capability::CodeGeneration
        );
        assert_eq!(
            Capability::from_str("unknown"),
            Capability::Custom("unknown".to_string())
        );
    }

    #[test]
    fn test_has_capability() {
        let def = AgentDefinition {
            capabilities: vec![Capability::CodeGeneration, Capability::Testing],
            ..Default::default()
        };

        let agent = Agent::new("cap-test", def);
        assert!(agent.has_capability(&Capability::CodeGeneration));
        assert!(!agent.has_capability(&Capability::Documentation));
    }

    #[test]
    fn test_agent_error_display() {
        let e1 = AgentError::AgentNotFound("missing".to_string());
        assert_eq!(e1.to_string(), "Agent未找到: missing");

        let e2 = AgentError::MaxIterationsExceeded("test".to_string());
        assert!(e2.to_string().contains("最大迭代次数"));
    }
}
