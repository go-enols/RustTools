//! 配置目录管理 —— 管理 `~/.config/rusttools/` 的目录结构和文件持久化
//!
//! 目录结构（对齐 Claude-CN）：
//! ```text
//! ~/.config/rusttools/
//! ├── config.json          # 核心配置（models, active_model, router_rules, skills）
//! ├── agents/              # Agent markdown 文件 (*.md)
//! ├── mcp/
//! │   └── servers.json     # MCP 服务器配置
//! ├── sessions/            # 会话存储
//! └── skills/              # 技能文件目录
//! ```

use super::models::*;
use super::ConfigError;
use std::path::{Path, PathBuf};

/// 配置目录管理器
pub struct ConfigDirectory {
    base_dir: PathBuf,
}

impl ConfigDirectory {
    /// 打开或创建默认配置目录（`~/.config/rusttools/`）
    pub fn new() -> Result<Self, ConfigError> {
        let base_dir = Self::default_base_dir()?;
        Self::with_path(base_dir)
    }

    /// 从指定路径创建
    pub fn with_path(base_dir: PathBuf) -> Result<Self, ConfigError> {
        let dir = Self { base_dir };
        dir.ensure_dirs()?;
        Ok(dir)
    }

    /// 默认配置目录：`~/.config/rusttools/`
    pub fn default_base_dir() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::config_dir().ok_or(ConfigError::InvalidPath)?;
        let base = config_dir.join("rusttools");
        std::fs::create_dir_all(&base)?;
        Ok(base)
    }

    /// 旧版配置路径：`~/.local/share/rusttools/agent_config.json`
    pub fn legacy_config_path() -> Result<PathBuf, ConfigError> {
        let data_dir = dirs::data_dir().ok_or(ConfigError::InvalidPath)?;
        Ok(data_dir.join("rusttools").join("agent_config.json"))
    }

    /// 确保所有子目录存在
    fn ensure_dirs(&self) -> Result<(), ConfigError> {
        for sub in ["agents", "mcp", "sessions", "skills"] {
            std::fs::create_dir_all(self.base_dir.join(sub))?;
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // 路径访问器
    // ------------------------------------------------------------------

    pub fn base(&self) -> &Path {
        &self.base_dir
    }

    pub fn config_json_path(&self) -> PathBuf {
        self.base_dir.join("config.json")
    }

    pub fn agents_dir(&self) -> PathBuf {
        self.base_dir.join("agents")
    }

    pub fn mcp_servers_path(&self) -> PathBuf {
        self.base_dir.join("mcp").join("servers.json")
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.base_dir.join("sessions")
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.base_dir.join("skills")
    }

    // ------------------------------------------------------------------
    // config.json 读写
    // ------------------------------------------------------------------

    /// 加载核心配置（config.json）
    pub fn load_config(&self) -> Result<AgentConfig, ConfigError> {
        let path = self.config_json_path();
        if !path.exists() {
            return Ok(AgentConfig::default());
        }
        let content = std::fs::read_to_string(&path)?;
        if content.trim().is_empty() {
            return Ok(AgentConfig::default());
        }
        let config: AgentConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 原子保存核心配置
    pub fn save_config(&self, config: &AgentConfig) -> Result<(), ConfigError> {
        let path = self.config_json_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(config)?;
        let temp = path.with_extension("tmp");
        std::fs::write(&temp, &json)?;
        std::fs::rename(&temp, &path)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // MCP servers.json 读写
    // ------------------------------------------------------------------

    /// 加载 MCP 服务器配置
    pub fn load_mcp_servers(&self) -> Result<Vec<McpServerConfig>, ConfigError> {
        let path = self.mcp_servers_path();
        if !path.exists() {
            return Ok(vec![]);
        }
        let content = std::fs::read_to_string(&path)?;
        if content.trim().is_empty() {
            return Ok(vec![]);
        }
        let servers: Vec<McpServerConfig> = serde_json::from_str(&content)?;
        Ok(servers)
    }

    /// 保存 MCP 服务器配置
    pub fn save_mcp_servers(&self, servers: &[McpServerConfig]) -> Result<(), ConfigError> {
        let path = self.mcp_servers_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(servers)?;
        let temp = path.with_extension("tmp");
        std::fs::write(&temp, &json)?;
        std::fs::rename(&temp, &path)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Agent markdown 文件
    // ------------------------------------------------------------------

    /// 扫描 agents 目录下的所有 .md 文件
    pub fn list_agent_files(&self) -> Result<Vec<PathBuf>, ConfigError> {
        let dir = self.agents_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                files.push(path);
            }
        }
        files.sort();
        Ok(files)
    }

    /// 获取指定 id 的 agent 文件路径
    pub fn agent_file_path(&self, id: &str) -> PathBuf {
        self.agents_dir().join(format!("{}.md", id))
    }

    // ------------------------------------------------------------------
    // 旧配置迁移
    // ------------------------------------------------------------------

    /// 检查是否需要从旧版配置迁移
    pub fn needs_migration(&self) -> bool {
        match Self::legacy_config_path() {
            Ok(legacy) => legacy.exists() && !self.config_json_path().exists(),
            Err(_) => false,
        }
    }

    /// 执行旧版配置迁移
    ///
    /// 将 `~/.local/share/rusttools/agent_config.json` 中的数据迁移到新目录结构：
    /// - agents 数组 → `agents/*.md` 文件
    /// - mcp_servers → `mcp/servers.json`
    /// - 其余字段 → `config.json`
    pub fn migrate_from_legacy(&self) -> Result<(), ConfigError> {
        let legacy_path = Self::legacy_config_path()?;
        if !legacy_path.exists() {
            return Ok(());
        }

        log::info!("检测到旧版配置，开始迁移: {}", legacy_path.display());

        let content = std::fs::read_to_string(&legacy_path)?;
        let old_config: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ConfigError::Json(e))?;

        // 1. 迁移 agents → agents/*.md
        if let Some(agents) = old_config.get("agents").and_then(|a| a.as_array()) {
            use super::agent_file::AgentFile;
            for agent_val in agents {
                if let Ok(def) = serde_json::from_value::<AgentDefinition>(agent_val.clone()) {
                    let path = self.agent_file_path(&def.id);
                    if let Err(e) = AgentFile::save(&path, &def) {
                        log::warn!("迁移 agent '{}' 失败: {}", def.id, e);
                    } else {
                        log::info!("已迁移 agent: {}", def.id);
                    }
                }
            }
        }

        // 2. 迁移 mcp_servers → mcp/servers.json
        if let Some(mcp_servers) = old_config.get("mcpServers").and_then(|a| a.as_array()) {
            let servers: Vec<McpServerConfig> = mcp_servers
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            self.save_mcp_servers(&servers)?;
            log::info!("已迁移 {} 个 MCP 服务器", servers.len());
        }

        // 3. 迁移核心配置 → config.json（去掉 agents 和 mcp_servers）
        let mut new_config = old_config.clone();
        new_config.as_object_mut().map(|obj| {
            obj.remove("agents");
            obj.remove("mcpServers");
        });
        // 重新序列化为 AgentConfig
        let config: AgentConfig = serde_json::from_value(new_config)
            .unwrap_or_else(|_| AgentConfig::default());
        self.save_config(&config)?;
        log::info!("已迁移核心配置");

        // 4. 备份旧文件
        let backup = legacy_path.with_extension("json.migrated");
        std::fs::rename(&legacy_path, &backup)?;
        log::info!("旧配置已备份至: {}", backup.display());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directory_paths() {
        let temp = tempfile::tempdir().unwrap();
        let dir = ConfigDirectory::with_path(temp.path().to_path_buf()).unwrap();

        assert!(dir.config_json_path().ends_with("config.json"));
        assert!(dir.agents_dir().ends_with("agents"));
        assert!(dir.mcp_servers_path().ends_with("servers.json"));
    }

    #[test]
    fn test_config_roundtrip() {
        let temp = tempfile::tempdir().unwrap();
        let dir = ConfigDirectory::with_path(temp.path().to_path_buf()).unwrap();

        let config = AgentConfig {
            version: "2.0".to_string(),
            active_model: "auto".to_string(),
            models: vec![],
            auto_router_rules: vec![],
            skills: vec![],
            workspace_path: Some("/tmp/test".to_string()),
        };

        dir.save_config(&config).unwrap();
        let loaded = dir.load_config().unwrap();
        assert_eq!(loaded.version, "2.0");
        assert_eq!(loaded.workspace_path, Some("/tmp/test".to_string()));
    }

    #[test]
    fn test_mcp_roundtrip() {
        let temp = tempfile::tempdir().unwrap();
        let dir = ConfigDirectory::with_path(temp.path().to_path_buf()).unwrap();

        let servers = vec![McpServerConfig {
            name: "test".to_string(),
            transport: McpTransportType::Stdio,
            command: Some("echo".to_string()),
            args: vec!["hello".to_string()],
            env: std::collections::HashMap::new(),
            url: None,
            enabled: true,
        }];

        dir.save_mcp_servers(&servers).unwrap();
        let loaded = dir.load_mcp_servers().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "test");
    }
}
