//! AI Agent Tauri 命令集成层
//!
//! 提供前端与 Agent 核心模块交互的所有 Tauri 命令。
//! 本模块独立于 YOLO / Training 命令，保持架构边界清晰。

use rusttools_app::agent::agent_core::{AgentDefinition, Orchestrator, SessionManager};
use rusttools_app::agent::config::models::{AgentConfig, McpServerConfig, ModelConfig};
use rusttools_app::agent::config::ConfigManager;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Agent 全局状态 —— 包含编排器与会话管理器
pub struct AgentState {
    pub config_manager: Arc<ConfigManager>,
    pub orchestrator: Mutex<Orchestrator>,
    pub session_manager: Mutex<SessionManager>,
}

impl Default for AgentState {
    fn default() -> Self {
        let config_manager = Arc::new(ConfigManager::new().expect("Failed to create ConfigManager"));
        let orchestrator = Orchestrator::new(Arc::clone(&config_manager));
        Self {
            config_manager,
            orchestrator: Mutex::new(orchestrator),
            session_manager: Mutex::new(SessionManager::new()),
        }
    }
}

fn uuid() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let bytes: [u8; 16] = rng.random();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11],
        bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

// ============================================================================
// Model Commands
// ============================================================================

#[tauri::command]
pub fn agent_get_models(state: tauri::State<'_, AgentState>) -> Result<Vec<ModelConfig>, String> {
    let config = state.config_manager.get();
    Ok(config.models)
}

#[tauri::command]
pub fn agent_add_model(
    model: ModelConfig,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    state
        .config_manager
        .update(|cfg| {
            cfg.models.push(model);
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn agent_remove_model(
    model_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    state
        .config_manager
        .update(|cfg| {
            cfg.models.retain(|m| m.id != model_id);
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_test_model(
    model_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<bool, String> {
    let model = state
        .config_manager
        .get_model(&model_id)
        .ok_or_else(|| format!("Model not found: {}", model_id))?;

    let client = rusttools_app::agent::api_client::UnifiedClient::new(&[model.clone()])
        .map_err(|e| e.to_string())?;

    let request = rusttools_app::agent::api_client::ChatRequest {
        model: model.default_model.clone(),
        messages: vec![rusttools_app::agent::api_client::ChatMessage::User {
            content: "Hello".to_string().into(),
        }],
        tools: None,
        stream: false,
        temperature: None,
        max_tokens: Some(10),
    };

    match client.chat(&model.id, request).await {
        Ok(_) => Ok(true),
        Err(e) => Err(e.to_string()),
    }
}

// ============================================================================
// Agent Commands
// ============================================================================

#[tauri::command]
pub async fn agent_list_agents(
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<rusttools_app::agent::agent_core::AgentInfo>, String> {
    let orch = state.orchestrator.lock().await;
    Ok(orch.list_agents())
}

#[tauri::command]
pub async fn agent_create_agent(
    def: AgentDefinition,
    state: tauri::State<'_, AgentState>,
) -> Result<String, String> {
    let mut orch = state.orchestrator.lock().await;
    orch.create_agent(def).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_update_agent(
    id: String,
    def: AgentDefinition,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    let mut orch = state.orchestrator.lock().await;
    orch.update_agent(&id, def).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_delete_agent(
    id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    let mut orch = state.orchestrator.lock().await;
    orch.delete_agent(&id).map_err(|e| e.to_string())
}

// ============================================================================
// MCP Server Commands
// ============================================================================

#[tauri::command]
pub fn agent_get_mcp_servers(
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<rusttools_app::agent::McpServerInfo>, String> {
    let config = state.config_manager.get();
    let servers: Vec<rusttools_app::agent::McpServerInfo> = config
        .mcp_servers
        .into_iter()
        .map(|s| rusttools_app::agent::McpServerInfo {
            name: s.name.clone(),
            transport: format!("{:?}", s.transport).to_lowercase(),
            command: s.command.unwrap_or_default(),
            status: rusttools_app::agent::ServerStatus::Disconnected,
            tool_count: 0,
            resource_count: 0,
            last_error: None,
        })
        .collect();
    Ok(servers)
}

#[tauri::command]
pub fn agent_add_mcp_server(
    server: McpServerConfig,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    state
        .config_manager
        .update(|cfg| {
            cfg.mcp_servers.push(server);
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn agent_remove_mcp_server(
    name: String,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    state
        .config_manager
        .update(|cfg| {
            cfg.mcp_servers.retain(|s| s.name != name);
        })
        .map_err(|e| e.to_string())
}

// ============================================================================
// Chat / Session Commands
// ============================================================================

#[tauri::command]
pub async fn agent_send_message(
    session_id: String,
    message: String,
    state: tauri::State<'_, AgentState>,
) -> Result<String, String> {
    // 阶段 1：同步准备（持有锁）
    {
        let mut orch = state.orchestrator.lock().await;
        // 确保默认 agent 存在
        if orch.get_agent("default").is_none() {
            let def = AgentDefinition {
                name: "Default Agent".to_string(),
                description: "默认智能助手".to_string(),
                system_prompt: "You are a helpful assistant.".to_string(),
                ..Default::default()
            };
            let _ = orch.create_agent(def);
        }
        let _ = orch.create_session(&session_id, "default");
    } // 锁在这里释放

    // 阶段 2：异步执行（不持有锁）
    let model = state
        .config_manager
        .get()
        .models
        .first()
        .cloned()
        .unwrap_or_default();
    let client = rusttools_app::agent::api_client::UnifiedClient::new(&[model])
        .map_err(|e| e.to_string())?;

    {
        let orch = state.orchestrator.lock().await;
        let result = orch
            .execute_task(&session_id, &message, &client)
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.final_response)
    }
}

#[tauri::command]
pub fn agent_cancel_chat(_state: tauri::State<'_, AgentState>) -> Result<(), String> {
    Ok(())
}

// ============================================================================
// Config Commands
// ============================================================================

#[tauri::command]
pub fn agent_load_config(state: tauri::State<'_, AgentState>) -> Result<AgentConfig, String> {
    let cfg = state.config_manager.load().map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[tauri::command]
pub fn agent_save_config(
    config: AgentConfig,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    state.config_manager.save(&config).map_err(|e| e.to_string())
}

// ============================================================================
// Skill / Session Commands
// ============================================================================

#[tauri::command]
pub fn agent_translate_skill(
    description: String,
    target_lang: String,
) -> Result<String, String> {
    Ok(format!("[{}] {}", target_lang, description))
}

#[tauri::command]
pub async fn agent_create_session(
    agent_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<String, String> {
    let session_id = uuid();
    let orch = state.orchestrator.lock().await;
    orch.create_session(&session_id, &agent_id)
        .map_err(|e| e.to_string())?;
    Ok(session_id)
}

#[tauri::command]
pub async fn agent_get_session_info(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let sessions = state.session_manager.lock().await;
    let _session = sessions.get(&session_id).ok_or("Session not found")?;
    Ok(serde_json::json!({
        "id": session_id,
        "agent_id": "",
        "message_count": 0,
    }))
}
