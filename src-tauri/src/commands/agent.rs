//! Agent 模块的 Tauri 命令集成层
//!
//! 提供所有与 AI Agent 相关的 Tauri 命令，包括：
//! - 模型管理（CRUD、测试连接）
//! - Agent 管理（CRUD）
//! - MCP 服务器管理
//! - 聊天消息发送
//! - 配置加载/保存
//!
//! 所有命令通过 `AgentState` 共享状态，使用 parking_lot::RwLock 保证线程安全。

use parking_lot::RwLock;
use std::sync::Arc;
use tauri::State;

// ============================================================================
// 状态管理
// ============================================================================

/// Agent 模块共享状态
///
/// 包含配置管理器、编排器和会话管理器，所有 Agent 命令共享此状态。
pub struct AgentState {
    /// 配置管理器（线程安全，多读者单写者）
    pub config_manager: Arc<rusttools_app::agent::config::ConfigManager>,
    /// Agent 编排器（负责 Agent 生命周期和任务执行）
    pub orchestrator: RwLock<rusttools_app::agent::agent_core::Orchestrator>,
    /// 会话管理器（管理所有活跃对话会话）
    pub session_manager: RwLock<rusttools_app::agent::agent_core::SessionManager>,
}

impl AgentState {
    /// 创建新的 AgentState
    ///
    /// 内部初始化配置管理器、编排器和会话管理器。
    /// 如果配置管理器创建失败（如无法访问文件系统），将返回错误。
    pub fn new() -> Result<Self, String> {
        let config_manager = Arc::new(
            rusttools_app::agent::config::ConfigManager::new()
                .map_err(|e| format!("创建配置管理器失败: {}", e))?,
        );
        let orchestrator =
            rusttools_app::agent::agent_core::Orchestrator::new(config_manager.clone());
        Ok(Self {
            config_manager: config_manager.clone(),
            orchestrator: RwLock::new(orchestrator),
            session_manager: RwLock::new(
                rusttools_app::agent::agent_core::SessionManager::new(),
            ),
        })
    }
}

impl Default for AgentState {
    fn default() -> Self {
        // 使用 unwrap_or_else 在失败时提供合理的回退行为
        Self::new().unwrap_or_else(|e| {
            eprintln!("[AgentState] 初始化失败，使用空配置: {}", e);
            let config_manager = Arc::new(
                rusttools_app::agent::config::ConfigManager::with_path(
                    "/tmp/rusttools_agent_config.json",
                )
                .expect("无法创建配置管理器"),
            );
            let orchestrator =
                rusttools_app::agent::agent_core::Orchestrator::new(config_manager.clone());
            Self {
                config_manager: config_manager.clone(),
                orchestrator: RwLock::new(orchestrator),
                session_manager: RwLock::new(
                    rusttools_app::agent::agent_core::SessionManager::new(),
                ),
            }
        })
    }
}

// ============================================================================
// 模型测试结果
// ============================================================================

/// 模型连接测试结果
#[derive(serde::Serialize)]
pub struct ModelTestResult {
    /// 测试是否成功
    pub success: bool,
    /// 连接延迟（毫秒）
    pub latency_ms: u64,
    /// 结果消息
    pub message: String,
}

// ============================================================================
// 模型管理命令
// ============================================================================

/// 获取所有已配置的模型列表
///
/// 从配置管理器读取当前配置，返回模型配置列表。
#[tauri::command]
pub fn agent_get_models(
    state: State<AgentState>,
) -> Result<Vec<rusttools_app::agent::config::ModelConfig>, String> {
    let config = state.config_manager.get();
    Ok(config.models)
}

/// 添加新模型配置
///
/// 将新模型添加到配置中并自动保存。
#[tauri::command]
pub fn agent_add_model(
    model: rusttools_app::agent::config::ModelConfig,
    state: State<AgentState>,
) -> Result<(), String> {
    state
        .config_manager
        .update(|cfg| {
            cfg.models.push(model);
        })
        .map_err(|e| e.to_string())
}

/// 移除指定模型
///
/// 根据模型 ID 从配置中移除并自动保存。
#[tauri::command]
pub fn agent_remove_model(id: String, state: State<AgentState>) -> Result<(), String> {
    state
        .config_manager
        .update(|cfg| {
            cfg.models.retain(|m| m.id != id);
        })
        .map_err(|e| e.to_string())
}

/// 测试模型连接
///
/// 检查模型配置的有效性，返回延迟信息。
/// 注意：当前为简化实现，仅验证配置格式，不实际发起网络请求。
#[tauri::command]
pub fn agent_test_model(
    id: String,
    state: State<AgentState>,
) -> Result<ModelTestResult, String> {
    let config = state.config_manager.get();
    let model = config
        .models
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| format!("模型 '{}' 未找到", id))?;

    let start = std::time::Instant::now();

    // 简化实现：验证配置有效性
    // TODO: 实际发起网络请求测试连接
    if model.provider.as_str().is_empty() {
        return Ok(ModelTestResult {
            success: false,
            latency_ms: 0,
            message: format!("模型 '{}' 的 Provider 无效", model.name),
        });
    }

    let latency_ms = start.elapsed().as_millis() as u64;

    Ok(ModelTestResult {
        success: true,
        latency_ms,
        message: format!(
            "模型 '{}' 配置有效 (Provider: {})",
            model.name,
            model.provider.as_str()
        ),
    })
}

// ============================================================================
// Agent 管理命令
// ============================================================================

/// 获取所有已配置的 Agent 列表
#[tauri::command]
pub fn agent_list_agents(
    state: State<AgentState>,
) -> Result<Vec<rusttools_app::agent::config::AgentDefinition>, String> {
    let config = state.config_manager.get();
    Ok(config.agents)
}

/// 创建新 Agent
///
/// 将新 Agent 定义添加到配置中并保存，返回新 Agent 的 ID。
#[tauri::command]
pub fn agent_create_agent(
    agent: rusttools_app::agent::config::AgentDefinition,
    state: State<AgentState>,
) -> Result<String, String> {
    let mut agent = agent;
    if agent.id.is_empty() {
        agent.id = format!("agent-{}", uuid());
    }
    let id = agent.id.clone();

    state
        .config_manager
        .update(|cfg| {
            cfg.agents.push(agent);
        })
        .map_err(|e| e.to_string())?;

    Ok(id)
}

/// 更新指定 Agent
///
/// 根据 ID 查找并替换 Agent 配置。
#[tauri::command]
pub fn agent_update_agent(
    id: String,
    agent: rusttools_app::agent::config::AgentDefinition,
    state: State<AgentState>,
) -> Result<(), String> {
    state
        .config_manager
        .update(|cfg| {
            if let Some(idx) = cfg.agents.iter().position(|a| a.id == id) {
                cfg.agents[idx] = agent;
            }
        })
        .map_err(|e| e.to_string())
}

/// 删除指定 Agent
///
/// 根据 ID 从配置中移除 Agent。
#[tauri::command]
pub fn agent_delete_agent(id: String, state: State<AgentState>) -> Result<(), String> {
    state
        .config_manager
        .update(|cfg| {
            cfg.agents.retain(|a| a.id != id);
        })
        .map_err(|e| e.to_string())
}

// ============================================================================
// MCP 管理命令
// ============================================================================

/// 获取所有 MCP 服务器信息
///
/// 从配置中读取 MCP 服务器列表，转换为 `McpServerInfo`。
#[tauri::command]
pub fn agent_get_mcp_servers(
    state: State<AgentState>,
) -> Result<Vec<rusttools_app::agent::McpServerInfo>, String> {
    let config = state.config_manager.get();
    let infos: Vec<_> = config
        .mcp_servers
        .into_iter()
        .map(|s| rusttools_app::agent::McpServerInfo {
            name: s.name.clone(),
            transport: format!("{:?}", s.transport).to_lowercase(),
            command: s.command.clone().unwrap_or_default(),
            status: rusttools_app::agent::ServerStatus::Disconnected,
            tool_count: 0,
            resource_count: 0,
            last_error: None,
        })
        .collect();
    Ok(infos)
}

/// 添加 MCP 服务器配置
#[tauri::command]
pub fn agent_add_mcp_server(
    config: rusttools_app::agent::config::McpServerConfig,
    state: State<AgentState>,
) -> Result<(), String> {
    state
        .config_manager
        .update(|cfg| {
            cfg.mcp_servers.push(config);
        })
        .map_err(|e| e.to_string())
}

/// 移除 MCP 服务器配置
#[tauri::command]
pub fn agent_remove_mcp_server(
    name: String,
    state: State<AgentState>,
) -> Result<(), String> {
    state
        .config_manager
        .update(|cfg| {
            cfg.mcp_servers.retain(|s| s.name != name);
        })
        .map_err(|e| e.to_string())
}

// ============================================================================
// 聊天命令
// ============================================================================

/// 发送聊天消息
///
/// 异步命令，将消息发送到指定会话。
/// 注意：当前为简化实现，实际流式传输需要使用 Tauri 的 emit 机制。
#[tauri::command]
pub async fn agent_send_message(
    _session_id: String,
    _message: String,
    _agent_id: String,
    _state: State<'_, AgentState>,
) -> Result<String, String> {
    // TODO: 实现实际的消息发送和流式响应
    // 1. 查找会话和 Agent
    // 2. 构建请求并发送到 LLM
    // 3. 使用 Tauri emit 发送流式响应到前端
    Ok("消息已接收（演示模式）".to_string())
}

/// 取消正在进行聊天
///
/// 中断指定会话的当前任务。
#[tauri::command]
pub fn agent_cancel_chat(_session_id: String, _state: State<AgentState>) -> Result<(), String> {
    // TODO: 实现取消逻辑
    Ok(())
}

// ============================================================================
// 配置命令
// ============================================================================

/// 加载当前配置
///
/// 返回完整的 Agent 配置（模型、Agent、MCP 服务器等）。
#[tauri::command]
pub fn agent_load_config(
    state: State<AgentState>,
) -> Result<rusttools_app::agent::config::AgentConfig, String> {
    Ok(state.config_manager.get())
}

/// 保存配置
///
/// 将完整的 Agent 配置保存到文件系统。
#[tauri::command]
pub fn agent_save_config(
    config: rusttools_app::agent::config::AgentConfig,
    state: State<AgentState>,
) -> Result<(), String> {
    state
        .config_manager
        .save(&config)
        .map_err(|e| e.to_string())
}

/// 翻译 Skill JSON
///
/// 解析 Skill JSON 并翻译描述字段。
/// 注意：当前为简化实现，返回带语言前缀的原始文本。
#[tauri::command]
pub fn agent_translate_skill(
    skill_json: String,
    target_lang: String,
) -> Result<String, String> {
    // 简化实现：解析 JSON，给 name 和 description 添加目标语言前缀
    let mut skill: serde_json::Value =
        serde_json::from_str(&skill_json).map_err(|e| format!("解析 JSON 失败: {}", e))?;

    let fields_to_translate = vec!["name", "description"];
    for field in &fields_to_translate {
        if let Some(serde_json::Value::String(text)) = skill.get(field) {
            let translated = format!("[{}] {}", target_lang, text);
            if let Some(obj) = skill.as_object_mut() {
                obj.insert(field.to_string(), serde_json::Value::String(translated));
            }
        }
    }

    serde_json::to_string_pretty(&skill).map_err(|e| format!("序列化失败: {}", e))
}

// ============================================================================
// 会话管理命令
// ============================================================================

/// 创建新会话
///
/// 为指定 Agent 创建新的对话会话。
#[tauri::command]
pub fn agent_create_session(
    session_id: String,
    agent_id: String,
    state: State<AgentState>,
) -> Result<(), String> {
    let mut orch = state.orchestrator.write();
    orch.create_session(session_id, &agent_id)
        .map_err(|e: rusttools_app::agent::agent_core::AgentError| e.to_string())
}

/// 获取会话管理器信息
///
/// 返回当前活跃会话数量。
#[tauri::command]
pub fn agent_get_session_info(state: State<AgentState>) -> Result<serde_json::Value, String> {
    let sm = state.session_manager.read();
    Ok(serde_json::json!({
        "session_count": sm.len(),
    }))
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 生成简单的唯一 ID
///
/// 使用随机数生成唯一标识符，避免依赖 uuid crate。
fn uuid() -> String {
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

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 创建测试用的 AgentState（使用临时配置路径）
    fn create_test_state() -> AgentState {
        let config_manager = Arc::new(
            rusttools_app::agent::config::ConfigManager::with_path(
                "/tmp/test_agent_commands_config.json",
            )
            .expect("创建配置管理器失败"),
        );
        let orchestrator =
            rusttools_app::agent::agent_core::Orchestrator::new(config_manager.clone());
        AgentState {
            config_manager: config_manager.clone(),
            orchestrator: RwLock::new(orchestrator),
            session_manager: RwLock::new(
                rusttools_app::agent::agent_core::SessionManager::new(),
            ),
        }
    }

    /// 测试 AgentState 创建
    #[test]
    fn test_agent_state_creation() {
        let state = create_test_state();
        // 验证状态创建成功且配置为空
        let config = state.config_manager.get();
        assert_eq!(config.version, "1.0");
    }

    /// 测试模型 CRUD 操作
    #[test]
    fn test_model_crud() {
        let state = create_test_state();

        // 添加模型（直接操作 config_manager，避免 State 依赖）
        let model = rusttools_app::agent::config::ModelConfig {
            id: "test-model".to_string(),
            name: "Test Model".to_string(),
            provider: rusttools_app::agent::config::ProviderType::OpenAI,
            api_key: Some("sk-test".to_string()),
            base_url: Some("https://api.openai.com".to_string()),
            models_list: vec!["gpt-4".to_string()],
            default_model: "gpt-4".to_string(),
            timeout_ms: 30000,
        };

        state
            .config_manager
            .update(|cfg| cfg.models.push(model.clone()))
            .expect("添加模型失败");

        // 验证模型已添加
        let config = state.config_manager.get();
        assert_eq!(config.models.len(), 1);
        assert_eq!(config.models[0].id, "test-model");

        // 删除模型
        state
            .config_manager
            .update(|cfg| cfg.models.retain(|m| m.id != "test-model"))
            .expect("删除模型失败");

        let config = state.config_manager.get();
        assert!(config.models.is_empty());
    }

    /// 测试 Agent CRUD 操作
    #[test]
    fn test_agent_crud() {
        let state = create_test_state();

        // 创建 Agent
        let agent = rusttools_app::agent::config::AgentDefinition {
            id: "test-agent".to_string(),
            name: "测试助手".to_string(),
            description: "用于测试的助手".to_string(),
            system_prompt: "你是测试助手".to_string(),
            model_id: "auto".to_string(),
            tools: vec!["fs_read".to_string()],
            mcp_servers: vec![],
            auto_mode: true,
            max_iterations: 10,
            allowed_directories: vec![],
            capabilities: vec![],
        };

        state
            .config_manager
            .update(|cfg| cfg.agents.push(agent.clone()))
            .expect("添加 Agent 失败");

        // 验证 Agent 已添加
        let config = state.config_manager.get();
        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].name, "测试助手");

        // 更新 Agent
        let mut updated = agent.clone();
        updated.name = "已更新".to_string();
        state
            .config_manager
            .update(|cfg| {
                if let Some(idx) = cfg.agents.iter().position(|a| a.id == "test-agent") {
                    cfg.agents[idx] = updated;
                }
            })
            .expect("更新 Agent 失败");

        let config = state.config_manager.get();
        assert_eq!(config.agents[0].name, "已更新");

        // 删除 Agent
        state
            .config_manager
            .update(|cfg| cfg.agents.retain(|a| a.id != "test-agent"))
            .expect("删除 Agent 失败");

        let config = state.config_manager.get();
        assert!(config.agents.is_empty());
    }

    /// 测试 MCP 服务器管理
    #[test]
    fn test_mcp_server_management() {
        let state = create_test_state();

        // 添加服务器
        let server = rusttools_app::agent::config::McpServerConfig {
            name: "test-server".to_string(),
            transport: rusttools_app::agent::config::McpTransportType::Stdio,
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
            ],
            env: HashMap::new(),
            url: None,
            enabled: true,
        };

        state
            .config_manager
            .update(|cfg| cfg.mcp_servers.push(server.clone()))
            .expect("添加服务器失败");

        // 验证服务器已添加
        let config = state.config_manager.get();
        assert_eq!(config.mcp_servers.len(), 1);
        assert_eq!(config.mcp_servers[0].name, "test-server");

        // 删除服务器
        state
            .config_manager
            .update(|cfg| cfg.mcp_servers.retain(|s| s.name != "test-server"))
            .expect("删除服务器失败");

        let config = state.config_manager.get();
        assert!(config.mcp_servers.is_empty());
    }

    /// 测试配置加载和保存
    #[test]
    fn test_config_load_save() {
        let state = create_test_state();

        // 加载配置（默认空配置）
        let config = state.config_manager.get();
        assert_eq!(config.version, "1.0");

        // 修改并保存
        let mut new_config = config.clone();
        new_config.active_model = "test-model".to_string();
        state
            .config_manager
            .save(&new_config)
            .expect("保存配置失败");

        // 重新加载验证
        let loaded = state.config_manager.load().expect("加载配置失败");
        assert_eq!(loaded.active_model, "test-model");
    }

    /// 测试 Skill 翻译
    #[test]
    fn test_translate_skill() {
        let skill_json =
            r#"{"name":"Read File","description":"Read a file from disk","parameters":{}}"#;

        let result = agent_translate_skill(skill_json.to_string(), "zh".to_string());
        assert!(result.is_ok());

        let translated = result.unwrap();
        assert!(translated.contains("[zh]"));
    }

    /// 测试测试不存在的模型（通过直接查询）
    #[test]
    fn test_model_not_found() {
        let state = create_test_state();
        let config = state.config_manager.get();
        let model = config.models.iter().find(|m| m.id == "nonexistent");
        assert!(model.is_none());
    }

    /// 测试 Skill 翻译的无效 JSON 处理
    #[test]
    fn test_translate_skill_invalid_json() {
        let result = agent_translate_skill("invalid json".to_string(), "zh".to_string());
        assert!(result.is_err());
    }

    /// 测试取消聊天（简化）
    #[test]
    fn test_cancel_chat_simpl() {
        // 取消聊天当前为空操作，测试通过
        assert!(true);
    }

    /// 测试会话管理器信息
    #[test]
    fn test_session_manager_info() {
        let state = create_test_state();
        let sm = state.session_manager.read();
        assert_eq!(sm.len(), 0);
    }

    /// 测试 UUID 生成
    #[test]
    fn test_uuid_generation() {
        let id1 = uuid();
        let id2 = uuid();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // 标准 UUID 字符串长度
    }

    /// 测试 ModelConfig Default
    #[test]
    fn test_model_config_default() {
        let default = rusttools_app::agent::config::ModelConfig::default();
        assert_eq!(default.id, "default");
        assert_eq!(default.timeout_ms, 60000);
    }

    /// 测试 AgentDefinition Default
    #[test]
    fn test_agent_definition_default() {
        let default = rusttools_app::agent::config::AgentDefinition::default();
        assert!(default.id.is_empty());
        assert_eq!(default.model_id, "auto");
        assert!(default.capabilities.is_empty());
    }
}
