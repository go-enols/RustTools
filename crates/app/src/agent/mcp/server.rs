//! MCP服务器管理 — 管理多个MCP服务器实例
//!
//! 提供服务器CRUD、连接状态管理、工具聚合和Skills翻译功能。

use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::api_client::UnifiedClient;
use crate::agent::config::models::{McpServerConfig, McpTransportType};
use crate::agent::config::ConfigManager;
use crate::agent::tools::tool::ToolDefinition;
use crate::agent::{McpError, McpServerInfo, ServerStatus};
use super::client::McpClient;
use super::transport::StdioTransport;
use super::types::McpCallToolResult;

/// MCP服务器实例
#[derive(Debug)]
pub struct McpServerInstance {
    pub config: McpServerConfig,
    pub client: Option<McpClient>,
    pub status: ServerStatus,
    pub last_error: Option<String>,
}

impl McpServerInstance {
    fn new(config: McpServerConfig) -> Self {
        Self {
            status: ServerStatus::Disconnected,
            client: None,
            last_error: None,
            config,
        }
    }

    fn info(&self) -> McpServerInfo {
        let tool_count = self.client.as_ref().map(|c| c.tools.len()).unwrap_or(0);
        let resource_count = self.client.as_ref().map(|c| c.resources.len()).unwrap_or(0);
        McpServerInfo {
            name: self.config.name.clone(),
            transport: format!("{:?}", self.config.transport).to_lowercase(),
            command: self.config.command.clone().unwrap_or_default(),
            status: self.status.clone(),
            tool_count,
            resource_count,
            last_error: self.last_error.clone(),
        }
    }
}

/// MCP管理器 — 编排器中引用的简单包装
///
/// 实际功能由McpServerManager提供，此类型作为Orchestrator的占位。
pub struct McpManager;

impl McpManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}

/// MCP服务器管理器 — 统一管理多个MCP服务器
///
/// 线程安全，可在多任务环境中共享。
pub struct McpServerManager {
    servers: RwLock<HashMap<String, McpServerInstance>>,
    config_manager: Arc<ConfigManager>,
}

impl McpServerManager {
    /// 创建新的服务器管理器
    pub fn new(config_manager: Arc<ConfigManager>) -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
            config_manager,
        }
    }

    /// 从配置加载所有MCP服务器
    pub fn load_from_config(&self) {
        let config = self.config_manager.get();
        let mut servers = self.servers.write();
        for mcp_config in &config.mcp_servers {
            if !servers.contains_key(&mcp_config.name) {
                servers.insert(
                    mcp_config.name.clone(),
                    McpServerInstance::new(mcp_config.clone()),
                );
            }
        }
    }

    /// 添加服务器配置
    pub async fn add_server(&self, config: McpServerConfig) -> Result<(), McpError> {
        let name = config.name.clone();

        // 创建实例
        let instance = McpServerInstance::new(config);
        self.servers.write().insert(name, instance);

        Ok(())
    }

    /// 移除服务器
    pub async fn remove_server(&self, name: &str) -> Result<(), McpError> {
        // 先断开连接
        let _ = self.disconnect_server(name).await;

        // 从管理器移除
        self.servers.write().remove(name);

        Ok(())
    }

    /// 连接指定服务器
    pub async fn connect_server(&self, name: &str) -> Result<(), McpError> {
        let mut servers = self.servers.write();
        let instance = servers
            .get_mut(name)
            .ok_or_else(|| McpError::server_not_found(name))?;

        if matches!(instance.status, ServerStatus::Connected | ServerStatus::Connecting) {
            return Ok(());
        }

        instance.status = ServerStatus::Connecting;
        instance.last_error = None;

        // 获取配置
        let config = instance.config.clone();
        drop(servers);

        // 目前只支持stdio传输
        let command = config
            .command
            .clone()
            .ok_or_else(|| McpError::transport("服务器配置缺少command"))?;

        let transport = StdioTransport::new(&command, &config.args, &config.env)
            .await
            .map_err(|e| {
                let mut servers = self.servers.write();
                if let Some(inst) = servers.get_mut(name) {
                    inst.status = ServerStatus::Error(e.message.clone());
                    inst.last_error = Some(e.message.clone());
                }
                e
            })?;

        // 创建客户端并初始化
        let mut client = McpClient::connect(Box::new(transport), name.to_string()).await?;

        let init_result = client.initialize().await.map_err(|e| {
            let mut servers = self.servers.write();
            if let Some(inst) = servers.get_mut(name) {
                inst.status = ServerStatus::Error(e.message.clone());
                inst.last_error = Some(e.message.clone());
            }
            e
        })?;

        // 获取工具列表
        let tools = client.list_tools().await.map_err(|e| {
            let mut servers = self.servers.write();
            if let Some(inst) = servers.get_mut(name) {
                inst.status = ServerStatus::Error(e.message.clone());
                inst.last_error = Some(e.message.clone());
            }
            e
        })?;

        // 更新实例状态
        let mut servers = self.servers.write();
        if let Some(inst) = servers.get_mut(name) {
            inst.client = Some(client);
            inst.status = ServerStatus::Connected;
            inst.last_error = None;
        }

        log::info!(
            "MCP服务器 '{}' 已连接，协议版本: {}，工具数: {}",
            name,
            init_result.protocol_version,
            tools.len()
        );

        Ok(())
    }

    /// 断开指定服务器
    pub async fn disconnect_server(&self, name: &str) -> Result<(), McpError> {
        let mut servers = self.servers.write();
        let instance = servers
            .get_mut(name)
            .ok_or_else(|| McpError::server_not_found(name))?;

        if let Some(mut client) = instance.client.take() {
            let _ = client.disconnect().await;
        }

        instance.status = ServerStatus::Disconnected;
        instance.last_error = None;

        Ok(())
    }

    /// 测试服务器连接
    pub async fn test_server(&self, name: &str) -> Result<ServerStatus, McpError> {
        match self.connect_server(name).await {
            Ok(()) => {
                let servers = self.servers.read();
                let status = servers
                    .get(name)
                    .map(|i| i.status.clone())
                    .unwrap_or(ServerStatus::Disconnected);
                Ok(status)
            }
            Err(e) => {
                let mut servers = self.servers.write();
                if let Some(inst) = servers.get_mut(name) {
                    inst.status = ServerStatus::Error(e.message.clone());
                    inst.last_error = Some(e.message.clone());
                }
                Ok(ServerStatus::Error(e.message))
            }
        }
    }

    /// 列出所有服务器信息
    pub fn list_servers(&self) -> Vec<McpServerInfo> {
        self.servers
            .read()
            .values()
            .map(|i| i.info())
            .collect()
    }

    /// 在指定服务器上调用工具
    pub async fn call_tool(
        &self,
        server: &str,
        tool: &str,
        args: Value,
    ) -> Result<McpCallToolResult, McpError> {
        let mut servers = self.servers.write();
        let instance = servers
            .get_mut(server)
            .ok_or_else(|| McpError::server_not_found(server))?;

        if !matches!(instance.status, ServerStatus::Connected) {
            return Err(McpError::transport(&format!("服务器 '{}' 未连接", server)));
        }

        let client = instance
            .client
            .as_mut()
            .ok_or_else(|| McpError::transport("客户端未初始化"))?;

        client.call_tool(tool, args).await
    }

    /// 聚合所有已连接服务器的工具定义
    pub fn get_all_tools(&self) -> Vec<ToolDefinition> {
        let servers = self.servers.read();
        let mut all_tools = Vec::new();

        for instance in servers.values() {
            if let Some(client) = &instance.client {
                let defs = client.get_tool_definitions();
                all_tools.extend(defs);
            }
        }

        all_tools
    }

    /// Skills一键翻译 — 解析skill JSON并翻译描述
    ///
    /// # Arguments
    /// * `skill_json` — 原始skill JSON字符串
    /// * `target_lang` — 目标语言 (如 "zh", "en")
    /// * `client` — 统一LLM客户端
    ///
    /// # Returns
    /// 翻译后的JSON字符串
    pub async fn translate_skill_description(
        &self,
        skill_json: &str,
        target_lang: &str,
        _client: &UnifiedClient,
    ) -> Result<String, McpError> {
        // 解析JSON
        let mut skill: Value = serde_json::from_str(skill_json)
            .map_err(|e| McpError::protocol(&format!("解析skill JSON失败: {}", e)))?;

        // 提取需要翻译的字段
        let fields_to_translate = vec!["name", "description"];
        let mut translations_needed = Vec::new();

        for field in &fields_to_translate {
            if let Some(val) = skill.get(field).and_then(|v| v.as_str()) {
                translations_needed.push((field.to_string(), val.to_string()));
            }
        }

        // 使用简单翻译（前缀标记）作为占位实现
        // 实际应调用LLM进行翻译
        let mut translated = HashMap::new();
        for (field, text) in &translations_needed {
            let result = format!("[{}] {}", target_lang, text);
            translated.insert(field.clone(), result);
        }

        // 应用翻译
        for (field, text) in translated {
            if let Some(v) = skill.get_mut(&field) {
                *v = Value::String(text);
            }
        }

        // 返回翻译后的JSON
        serde_json::to_string_pretty(&skill)
            .map_err(|e| McpError::protocol(&format!("序列化翻译结果失败: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::config::models::McpTransportType;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_manager() -> McpServerManager {
        // 使用内存配置
        let config_mgr = Arc::new(ConfigManager::with_path("/dev/null").unwrap_or_else(|_| {
            ConfigManager::with_path("/tmp/test_config.json").unwrap()
        }));
        McpServerManager::new(config_mgr)
    }

    fn test_config(name: &str) -> McpServerConfig {
        McpServerConfig {
            name: name.to_string(),
            transport: McpTransportType::Stdio,
            command: Some("echo".to_string()),
            args: vec![],
            env: HashMap::new(),
            url: None,
            enabled: true,
        }
    }

    #[tokio::test]
    async fn test_add_and_remove_server() {
        let manager = create_manager();
        let config = test_config("test");

        manager.add_server(config).await.unwrap();
        let servers = manager.list_servers();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "test");

        manager.remove_server("test").await.unwrap();
        assert!(manager.list_servers().is_empty());
    }

    #[tokio::test]
    async fn test_list_servers_empty() {
        let manager = create_manager();
        let servers = manager.list_servers();
        assert!(servers.is_empty());
    }

    #[tokio::test]
    async fn test_server_not_found() {
        let manager = create_manager();
        let result = manager.connect_server("nonexistent").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "SERVER_NOT_FOUND");
    }

    #[tokio::test]
    async fn test_disconnect_not_connected() {
        let manager = create_manager();
        let config = test_config("test");
        manager.add_server(config).await.unwrap();

        let result = manager.disconnect_server("test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_call_tool_server_not_connected() {
        let manager = create_manager();
        let config = test_config("test");
        manager.add_server(config).await.unwrap();

        let result = manager.call_tool("test", "tool1", json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_translate_skill_description() {
        let manager = create_manager();

        // UnifiedClient需要配置，测试时传入空配置创建的客户端
        let client = UnifiedClient::new(&[]).unwrap();

        let skill_json = r#"{
            "name": "Read File",
            "description": "Read a file from disk",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path to read"
                    }
                }
            }
        }"#;

        let result = manager
            .translate_skill_description(skill_json, "zh", &client)
            .await;
        assert!(result.is_ok());

        let translated = result.unwrap();
        let parsed: Value = serde_json::from_str(&translated).unwrap();
        assert!(parsed.get("name").is_some());
        assert!(parsed.get("description").is_some());
    }

    #[tokio::test]
    async fn test_translate_invalid_json() {
        let manager = create_manager();
        let client = UnifiedClient::new(&[]).unwrap();

        let result = manager
            .translate_skill_description("not json", "zh", &client)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_all_tools_empty() {
        let manager = create_manager();
        let tools = manager.get_all_tools();
        assert!(tools.is_empty());
    }

    #[test]
    fn test_mcp_manager_new() {
        let mgr = McpManager::new();
        // 简单验证构造成功
        let _ = mgr;
    }
}
