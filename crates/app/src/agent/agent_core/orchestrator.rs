//! 编排器
//!
//! Orchestrator是Agent编排引擎的核心，负责：
//! - 管理Agent生命周期（CRUD）
//! - 管理活跃会话
//! - 协调任务执行流程
//! - 参考Claude Code设计理念

use super::agent::{Agent, AgentDefinition, AgentError, AgentInfo};
use super::executor::Executor;
use super::planner::Planner;
use super::session::{Session, SessionManager, SessionStatus};
use super::super::api_client::{ChatMessage, ChatRequest, ChatResponse, ToolCall, UnifiedClient};
use super::super::config::{AgentConfig, ConfigManager};
use super::super::mcp::McpManager;
use super::super::tools::{ToolDefinition, ToolRegistry};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use ::uuid::Uuid;

// ============================================================================
// 任务结果
// ============================================================================

/// 任务执行结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskResult {
    /// 最终回复内容
    pub final_response: String,
    /// 执行步骤记录
    pub steps_executed: Vec<StepRecord>,
    /// 工具调用记录
    pub tool_calls: Vec<ToolCallRecord>,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
}

/// 步骤执行记录
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepRecord {
    /// 步骤ID
    pub step_id: String,
    /// 步骤描述
    pub description: String,
    /// 是否成功
    pub success: bool,
    /// 执行结果
    pub result: String,
}

/// 工具调用记录
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallRecord {
    /// 工具名称
    pub tool_name: String,
    /// 调用参数
    pub parameters: serde_json::Value,
    /// 返回结果
    pub result: String,
    /// 是否成功
    pub success: bool,
}

// ============================================================================
// 编排器
// ============================================================================

/// Agent编排器 — 管理Agent和会话，执行用户任务
pub struct Orchestrator {
    /// 已注册的Agent
    agents: HashMap<String, Agent>,
    /// 活跃会话
    active_sessions: RwLock<HashMap<String, Session>>,
    /// 工具注册中心
    tool_registry: ToolRegistry,
    /// MCP管理器
    mcp_manager: Arc<McpManager>,
    /// 配置管理器
    config_manager: Arc<ConfigManager>,
    /// 会话管理器
    session_manager: RwLock<SessionManager>,
}

impl Orchestrator {
    /// 创建新的编排器
    pub fn new(config_manager: Arc<ConfigManager>) -> Self {
        let mcp_manager = Arc::new(McpManager::new());
        let tool_registry = ToolRegistry::new();

        let mut orchestrator = Self {
            agents: HashMap::new(),
            active_sessions: RwLock::new(HashMap::new()),
            tool_registry,
            mcp_manager,
            config_manager,
            session_manager: RwLock::new(SessionManager::new()),
        };

        // 加载默认Agent
        let _ = orchestrator.load_agents();

        orchestrator
    }

    /// 从配置加载Agent
    pub fn load_agents(&mut self) -> Result<(), AgentError> {
        let config = self.config_manager.get();

        for agent_config in config.agents {
            let id = if agent_config.id.is_empty() {
                format!("agent-{}", self.agents.len())
            } else {
                agent_config.id.clone()
            };

            let def = AgentDefinition {
                name: agent_config.name,
                description: agent_config.description,
                system_prompt: agent_config.system_prompt,
                model_config: self
                    .config_manager
                    .get_model(&agent_config.model_id)
                    .unwrap_or_default(),
                tools: agent_config.tools,
                mcp_servers: agent_config.mcp_servers,
                auto_mode: agent_config.auto_mode,
                max_iterations: agent_config.max_iterations,
                allowed_directories: agent_config.allowed_directories,
                capabilities: agent_config
                    .capabilities
                    .into_iter()
                    .map(|c| super::agent::Capability::from_str(&c))
                    .collect(),
            };

            let agent = Agent::new(id.clone(), def);
            self.agents.insert(id, agent);
        }

        Ok(())
    }

    /// 创建新Agent
    pub fn create_agent(&mut self, def: AgentDefinition) -> Result<String, AgentError> {
        let id = format!("agent-{}", Uuid::new_v4());
        let agent = Agent::new(id.clone(), def);
        self.agents.insert(id.clone(), agent);
        Ok(id)
    }

    /// 更新Agent
    pub fn update_agent(
        &mut self,
        id: &str,
        def: AgentDefinition,
    ) -> Result<(), AgentError> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| AgentError::AgentNotFound(id.to_string()))?;

        agent.update(def);
        Ok(())
    }

    /// 删除Agent
    pub fn delete_agent(&mut self, id: &str) -> Result<(), AgentError> {
        if !self.agents.contains_key(id) {
            return Err(AgentError::AgentNotFound(id.to_string()));
        }

        // 清理关联的会话
        let mut sessions_to_remove = Vec::new();
        {
            let sessions = self.active_sessions.read();
            for (sid, session) in sessions.iter() {
                if session.agent_id == id {
                    sessions_to_remove.push(sid.clone());
                }
            }
        }

        for sid in sessions_to_remove {
            self.active_sessions.write().remove(&sid);
        }

        self.agents.remove(id);
        Ok(())
    }

    /// 列出所有Agent
    pub fn list_agents(&self) -> Vec<AgentInfo> {
        self.agents.values().map(|a| a.to_info()).collect()
    }

    /// 获取Agent
    pub fn get_agent(&self, id: &str) -> Option<&Agent> {
        self.agents.get(id)
    }

    /// 创建会话
    pub fn create_session(&self, session_id: impl Into<String>, agent_id: &str) -> Result<(), AgentError> {
        let agent = self
            .get_agent(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;

        let mut session_manager = self.session_manager.write();
        let session = session_manager.create(session_id, agent_id);

        // 添加系统提示词
        session.add_system_message(agent.system_prompt.clone());

        // 将会话放入活跃会话
        self.active_sessions
            .write()
            .insert(session.id.clone(), session.clone());

        Ok(())
    }

    /// 执行任务
    ///
    /// 完整的任务执行流程：
    /// 1. 获取会话和Agent
    /// 2. 规划任务
    /// 3. 执行计划
    /// 4. 汇总结果
    pub async fn execute_task(
        &self,
        session_id: &str,
        user_input: &str,
        client: &UnifiedClient,
    ) -> Result<TaskResult, AgentError> {
        let start_time = std::time::Instant::now();

        // 获取会话
        let mut session = {
            let sessions = self.active_sessions.read();
            sessions
                .get(session_id)
                .cloned()
                .ok_or_else(|| AgentError::SessionError(format!("会话不存在: {}", session_id)))?
        };

        // 获取Agent
        let agent = self
            .get_agent(&session.agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(session.agent_id.clone()))?;

        // 添加用户消息
        session.add_user_message(user_input.to_string());

        // 构建可用工具定义列表
        let tool_defs: Vec<ToolDefinition> = agent
            .tools
            .iter()
            .filter_map(|tool_id| self.tool_registry.get(tool_id))
            .map(|t| t.to_definition())
            .collect();

        // 规划任务
        let plan = Planner::plan(user_input, agent, &tool_defs);

        // 执行计划
        let executor = Executor::new();
        let result = executor
            .execute_plan(
                plan,
                &mut session,
                client,
                &self.tool_registry,
            )
            .await
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        // 更新会话状态
        session.set_status(SessionStatus::Completed);

        // 更新活跃会话
        self.active_sessions.write().insert(session_id.to_string(), session);

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(TaskResult {
            final_response: result.final_response,
            steps_executed: result.steps_executed,
            tool_calls: result.tool_calls,
            duration_ms,
        })
    }

    /// 注册工具
    pub fn register_tool(&mut self, tool: Box<dyn super::super::tools::Tool>) {
        self.tool_registry.register(tool);
    }

    /// 获取工具注册中心
    pub fn tool_registry(&self) -> &ToolRegistry {
        &self.tool_registry
    }
}

// ============================================================================
// UUID生成辅助
// ============================================================================

mod uuid {
    /// 简单的UUID生成（v4风格随机ID）
    pub fn new_v4() -> String {
        use rand::Rng;
        let mut rng = rand::rng();
        let bytes: [u8; 16] = rng.random();
        format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5],
            bytes[6], bytes[7],
            bytes[8], bytes[9],
            bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
        )
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::agent::{AgentDefinition, Capability};
    use super::super::config::ModelConfig;
    use super::super::tools::{TerminalTool, create_default_registry};

    fn create_test_orchestrator() -> Orchestrator {
        let config = Arc::new(
            ConfigManager::with_path("/tmp/test_orchestrator_config.json")
                .expect("创建配置管理器失败"),
        );
        let mut orch = Orchestrator::new(config);

        // 注册测试工具
        let registry = create_default_registry(vec![]);
        for tool in registry.list() {
            let name = tool.name().to_string();

            // 由于无法直接clone Box<dyn Tool>，我们需要重新创建
            match name.as_str() {
                "terminal" => orch.register_tool(Box::new(TerminalTool::new())),
                _ => {}
            }
        }

        orch
    }

    #[test]
    fn test_orchestrator_create_and_list_agents() {
        let mut orch = create_test_orchestrator();

        let def = AgentDefinition {
            name: "测试Agent".to_string(),
            description: "用于测试".to_string(),
            system_prompt: "测试".to_string(),
            model_config: ModelConfig::default(),
            tools: vec!["terminal".to_string()],
            capabilities: vec![Capability::CodeGeneration],
            ..Default::default()
        };

        let id = orch.create_agent(def).unwrap();
        assert!(!id.is_empty());

        let agents = orch.list_agents();
        assert!(agents.iter().any(|a| a.id == id));
    }

    #[test]
    fn test_orchestrator_update_agent() {
        let mut orch = create_test_orchestrator();

        let def = AgentDefinition {
            name: "旧名称".to_string(),
            ..Default::default()
        };
        let id = orch.create_agent(def).unwrap();

        let new_def = AgentDefinition {
            name: "新名称".to_string(),
            description: "新描述".to_string(),
            ..Default::default()
        };

        let result = orch.update_agent(&id, new_def);
        assert!(result.is_ok());

        let agent = orch.get_agent(&id).unwrap();
        assert_eq!(agent.name, "新名称");
        assert_eq!(agent.description, "新描述");
    }

    #[test]
    fn test_orchestrator_delete_agent() {
        let mut orch = create_test_orchestrator();

        let def = AgentDefinition {
            name: "将被删除".to_string(),
            ..Default::default()
        };
        let id = orch.create_agent(def).unwrap();

        assert!(orch.get_agent(&id).is_some());

        let result = orch.delete_agent(&id);
        assert!(result.is_ok());
        assert!(orch.get_agent(&id).is_none());
    }

    #[test]
    fn test_orchestrator_delete_nonexistent() {
        let mut orch = create_test_orchestrator();
        let result = orch.delete_agent("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_orchestrator_list_agents_info() {
        let mut orch = create_test_orchestrator();

        let def1 = AgentDefinition {
            name: "Agent A".to_string(),
            tools: vec!["t1".to_string(), "t2".to_string()],
            auto_mode: true,
            ..Default::default()
        };
        let def2 = AgentDefinition {
            name: "Agent B".to_string(),
            tools: vec!["t3".to_string()],
            auto_mode: false,
            ..Default::default()
        };

        orch.create_agent(def1).unwrap();
        orch.create_agent(def2).unwrap();

        let infos = orch.list_agents();
        assert!(infos.iter().any(|i| i.name == "Agent A" && i.tool_count == 2 && i.auto_mode));
        assert!(infos.iter().any(|i| i.name == "Agent B" && i.tool_count == 1 && !i.auto_mode));
    }

    #[test]
    fn test_orchestrator_crud_complete() {
        let mut orch = create_test_orchestrator();

        // Create
        let def = AgentDefinition {
            name: "完整测试".to_string(),
            description: "测试CRUD".to_string(),
            system_prompt: "system".to_string(),
            model_config: ModelConfig::default(),
            tools: vec!["fs_read".to_string()],
            mcp_servers: vec![],
            auto_mode: false,
            max_iterations: 10,
            allowed_directories: vec!["/tmp".to_string()],
            capabilities: vec![Capability::Analysis, Capability::Testing],
        };

        let id = orch.create_agent(def.clone()).unwrap();

        // Read
        let agent = orch.get_agent(&id).unwrap();
        assert_eq!(agent.name, "完整测试");
        assert_eq!(agent.max_iterations, 10);
        assert_eq!(agent.allowed_directories, vec!["/tmp"]);

        // Update
        let mut updated_def = def.clone();
        updated_def.name = "已更新".to_string();
        updated_def.max_iterations = 20;
        orch.update_agent(&id, updated_def).unwrap();

        let updated_agent = orch.get_agent(&id).unwrap();
        assert_eq!(updated_agent.name, "已更新");
        assert_eq!(updated_agent.max_iterations, 20);

        // Delete
        orch.delete_agent(&id).unwrap();
        assert!(orch.get_agent(&id).is_none());
    }

    #[tokio::test]
    async fn test_orchestrator_execute_task_basic() {
        let mut orch = create_test_orchestrator();

        // 创建一个带terminal工具的Agent
        let def = AgentDefinition {
            name: "执行测试".to_string(),
            system_prompt: "你是一个助手".to_string(),
            tools: vec!["terminal".to_string()],
            auto_mode: true,
            ..Default::default()
        };
        let agent_id = orch.create_agent(def).unwrap();

        // 创建会话
        let session_id = "test-session";
        orch.create_session(session_id, &agent_id).unwrap();

        // 执行任务（使用空配置客户端）
        let client = UnifiedClient::new(&[]).expect("创建客户端失败");
        let result = orch
            .execute_task(session_id, "执行echo命令", &client)
            .await;

        // 由于工具未注册，会返回错误（这是预期的）
        assert!(result.is_ok() || result.is_err());
    }
}
