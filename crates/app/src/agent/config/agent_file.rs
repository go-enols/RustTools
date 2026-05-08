//! Agent Markdown 文件解析 —— YAML frontmatter + Markdown body
//!
//! 文件格式：
//! ```markdown
//! ---
//! id: code-reviewer
//! name: 代码审查员
//! model_id: claude-sonnet
//! tools:
//!   - fs_read
//! ---
//!
//! # System Prompt
//!
//! You are a senior code reviewer...
//! ```

use super::models::AgentDefinition;
use super::ConfigError;
use std::path::Path;

/// 用于 YAML frontmatter 解析的中间结构（snake_case，所有字段可选）
#[derive(Debug, serde::Deserialize, serde::Serialize, Default)]
struct AgentFrontmatter {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default, alias = "modelId")]
    model_id: String,
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default, alias = "mcpServers")]
    mcp_servers: Vec<String>,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default, alias = "autoMode")]
    auto_mode: Option<bool>,
    #[serde(default, alias = "maxIterations")]
    max_iterations: Option<u32>,
    #[serde(default, alias = "allowedDirectories")]
    allowed_directories: Vec<String>,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default, alias = "systemPrompt")]
    system_prompt: Option<String>,
}

impl AgentFrontmatter {
    fn into_definition(self, body: String) -> AgentDefinition {
        AgentDefinition {
            id: self.id,
            name: self.name,
            description: self.description,
            system_prompt: self.system_prompt.filter(|s| !s.is_empty()).unwrap_or(body),
            model_id: if self.model_id.is_empty() { "auto".to_string() } else { self.model_id },
            tools: self.tools,
            mcp_servers: self.mcp_servers,
            skills: self.skills,
            auto_mode: self.auto_mode.unwrap_or(true),
            max_iterations: self.max_iterations.unwrap_or(50),
            allowed_directories: self.allowed_directories,
            capabilities: self.capabilities,
        }
    }
}

/// Agent Markdown 文件
pub struct AgentFile;

impl AgentFile {
    /// 从 markdown 文件加载 Agent 定义
    pub fn load(path: &Path) -> Result<AgentDefinition, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// 将 Agent 定义保存为 markdown 文件
    pub fn save(path: &Path, def: &AgentDefinition) -> Result<(), ConfigError> {
        let content = Self::serialize(def);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, content)?;
        std::fs::rename(&temp_path, path)?;
        Ok(())
    }

    /// 解析 markdown 内容
    ///
    /// 支持标准 YAML frontmatter（`---` 包裹）和纯 JSON（无 frontmatter）。
    pub fn parse(content: &str) -> Result<AgentDefinition, ConfigError> {
        let trimmed = content.trim_start();

        // 检查是否有 YAML frontmatter
        if trimmed.starts_with("---") {
            if let Some(end_idx) = trimmed[3..].find("\n---") {
                let frontmatter = &trimmed[3..3 + end_idx].trim();
                let body = trimmed[3 + end_idx + 4..].trim();

                let front: AgentFrontmatter = serde_yaml::from_str(frontmatter)
                    .map_err(|e| ConfigError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("YAML frontmatter 解析错误: {}", e),
                    )))?;

                Ok(front.into_definition(body.to_string()))
            } else {
                Err(ConfigError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "YAML frontmatter 未闭合",
                )))
            }
        } else {
            // 无 frontmatter：将整个内容作为 system_prompt，返回默认结构
            let def = AgentDefinition {
                system_prompt: content.to_string(),
                ..AgentDefinition::default()
            };
            Ok(def)
        }
    }

    /// 序列化为 markdown 内容
    pub fn serialize(def: &AgentDefinition) -> String {
        // 构建 YAML frontmatter
        let mut yaml = serde_yaml::Mapping::new();

        yaml.insert(
            serde_yaml::Value::String("id".to_string()),
            serde_yaml::Value::String(def.id.clone()),
        );
        yaml.insert(
            serde_yaml::Value::String("name".to_string()),
            serde_yaml::Value::String(def.name.clone()),
        );
        yaml.insert(
            serde_yaml::Value::String("description".to_string()),
            serde_yaml::Value::String(def.description.clone()),
        );
        yaml.insert(
            serde_yaml::Value::String("model_id".to_string()),
            serde_yaml::Value::String(def.model_id.clone()),
        );

        if !def.tools.is_empty() {
            yaml.insert(
                serde_yaml::Value::String("tools".to_string()),
                serde_yaml::Value::Sequence(
                    def.tools.iter().map(|s| serde_yaml::Value::String(s.clone())).collect(),
                ),
            );
        }

        if !def.mcp_servers.is_empty() {
            yaml.insert(
                serde_yaml::Value::String("mcp_servers".to_string()),
                serde_yaml::Value::Sequence(
                    def.mcp_servers.iter().map(|s| serde_yaml::Value::String(s.clone())).collect(),
                ),
            );
        }

        if !def.skills.is_empty() {
            yaml.insert(
                serde_yaml::Value::String("skills".to_string()),
                serde_yaml::Value::Sequence(
                    def.skills.iter().map(|s| serde_yaml::Value::String(s.clone())).collect(),
                ),
            );
        }

        yaml.insert(
            serde_yaml::Value::String("auto_mode".to_string()),
            serde_yaml::Value::Bool(def.auto_mode),
        );
        yaml.insert(
            serde_yaml::Value::String("max_iterations".to_string()),
            serde_yaml::Value::Number(def.max_iterations.into()),
        );

        if !def.allowed_directories.is_empty() {
            yaml.insert(
                serde_yaml::Value::String("allowed_directories".to_string()),
                serde_yaml::Value::Sequence(
                    def.allowed_directories.iter().map(|s| serde_yaml::Value::String(s.clone())).collect(),
                ),
            );
        }

        if !def.capabilities.is_empty() {
            yaml.insert(
                serde_yaml::Value::String("capabilities".to_string()),
                serde_yaml::Value::Sequence(
                    def.capabilities.iter().map(|s| serde_yaml::Value::String(s.clone())).collect(),
                ),
            );
        }

        let frontmatter = serde_yaml::to_string(&yaml).unwrap_or_default();
        let body = if def.system_prompt.is_empty() {
            String::new()
        } else {
            format!("\n{}", def.system_prompt)
        };

        format!("---\n{}---\n{}", frontmatter, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_frontmatter() {
        let content = r#"---
id: code-reviewer
name: 代码审查员
description: 审查代码
model_id: claude-sonnet
tools:
  - fs_read
  - fs_grep
auto_mode: true
max_iterations: 10
capabilities:
  - CodeReview
---

# System Prompt

You are a senior code reviewer.
"#;

        let def = AgentFile::parse(content).unwrap();
        assert_eq!(def.id, "code-reviewer");
        assert_eq!(def.name, "代码审查员");
        assert_eq!(def.model_id, "claude-sonnet");
        assert_eq!(def.tools, vec!["fs_read", "fs_grep"]);
        assert_eq!(def.system_prompt, "# System Prompt\n\nYou are a senior code reviewer.");
        assert!(def.auto_mode);
        assert_eq!(def.max_iterations, 10);
        assert_eq!(def.capabilities, vec!["CodeReview"]);
    }

    #[test]
    fn test_parse_without_frontmatter() {
        let content = "You are a helpful assistant.";
        let def = AgentFile::parse(content).unwrap();
        assert_eq!(def.system_prompt, "You are a helpful assistant.");
        assert!(def.id.is_empty());
    }

    #[test]
    fn test_serialize_roundtrip() {
        let def = AgentDefinition {
            id: "test-agent".to_string(),
            name: "测试 Agent".to_string(),
            description: "用于测试".to_string(),
            system_prompt: "你是测试助手。".to_string(),
            model_id: "gpt-4".to_string(),
            tools: vec!["fs_read".to_string()],
            mcp_servers: vec![],
            skills: vec![],
            auto_mode: false,
            max_iterations: 5,
            allowed_directories: vec![],
            capabilities: vec!["Testing".to_string()],
        };

        let serialized = AgentFile::serialize(&def);
        let parsed = AgentFile::parse(&serialized).unwrap();

        assert_eq!(parsed.id, def.id);
        assert_eq!(parsed.name, def.name);
        assert_eq!(parsed.model_id, def.model_id);
        assert_eq!(parsed.tools, def.tools);
        assert_eq!(parsed.system_prompt, def.system_prompt);
        assert_eq!(parsed.auto_mode, def.auto_mode);
        assert_eq!(parsed.max_iterations, def.max_iterations);
        assert_eq!(parsed.capabilities, def.capabilities);
    }

    #[test]
    fn test_save_and_load() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("test.md");

        let def = AgentDefinition {
            id: "my-agent".to_string(),
            name: "My Agent".to_string(),
            description: "desc".to_string(),
            system_prompt: "prompt".to_string(),
            model_id: "auto".to_string(),
            tools: vec![],
            mcp_servers: vec![],
            skills: vec![],
            auto_mode: true,
            max_iterations: 50,
            allowed_directories: vec![],
            capabilities: vec![],
        };

        AgentFile::save(&path, &def).unwrap();
        let loaded = AgentFile::load(&path).unwrap();
        assert_eq!(loaded.id, "my-agent");
        assert_eq!(loaded.system_prompt, "prompt");
    }
}
