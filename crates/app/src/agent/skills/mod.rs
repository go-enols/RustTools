//! Skills 系统
//!
//! 参考 Claude Code (Claude-CN) 的 Skills 设计理念：
//! - Skills 是预置的专家能力包，通过 system prompt 注入扩展 Agent 行为
//! - 每个 Skill 可以贡献额外的 system prompt 指令和可选的工具
//! - Skills 在 AgentDefinition 级别配置，运行时注入到会话中
//!
//! 架构：
//! - `Skill` trait — 能力定义接口
//! - `SkillRegistry` — Skill 注册与发现中心
//! - `BundledSkill` — 内置 Skill 的通用实现
//! - `skills_prompt_section()` — 生成注入 system prompt 的 skills 描述段

pub mod builtin;

use crate::agent::tools::{Tool, ToolRegistry};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================================================
// Skill Trait
// ============================================================================

/// Skill 能力接口
///
/// 每个 Skill 代表一种专家能力，通过 system prompt 注入影响 Agent 行为。
/// Skill 可以贡献：
/// 1. system prompt 追加指令（`system_prompt_addon`）
/// 2. 额外的工具（`tools`）— 可选
pub trait Skill: Send + Sync {
    /// Skill 唯一标识符（如 "git", "web_search"）
    fn id(&self) -> &str;

    /// Skill 显示名称
    fn name(&self) -> &str;

    /// Skill 描述
    fn description(&self) -> &str;

    /// 注入到 system prompt 的追加指令
    ///
    /// 返回 None 表示此 Skill 不需要追加任何指令。
    fn system_prompt_addon(&self) -> Option<String>;

    /// 此 Skill 贡献的额外工具
    ///
    /// 默认返回空列表。需要工具时可覆盖此方法。
    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![]
    }
}

// ============================================================================
// Skill Registry
// ============================================================================

/// Skill 注册中心
///
/// 管理所有可用的 Skill，支持运行时注册和按 ID 列表获取启用的 Skill。
pub struct SkillRegistry {
    skills: RwLock<HashMap<String, Arc<dyn Skill>>>,
}

impl SkillRegistry {
    /// 创建空的 Skill Registry
    pub fn new() -> Self {
        Self {
            skills: RwLock::new(HashMap::new()),
        }
    }

    /// 注册一个 Skill
    pub fn register(&self, skill: Arc<dyn Skill>) {
        let id = skill.id().to_string();
        let mut skills = self.skills.write().unwrap();
        skills.insert(id, skill);
    }

    /// 注销一个 Skill
    pub fn unregister(&self, id: &str) -> bool {
        let mut skills = self.skills.write().unwrap();
        skills.remove(id).is_some()
    }

    /// 根据 ID 获取 Skill
    pub fn get(&self, id: &str) -> Option<Arc<dyn Skill>> {
        let skills = self.skills.read().unwrap();
        skills.get(id).cloned()
    }

    /// 获取所有已注册 Skill 的 ID 列表
    pub fn list_ids(&self) -> Vec<String> {
        let skills = self.skills.read().unwrap();
        skills.keys().cloned().collect()
    }

    /// 根据 ID 列表获取启用的 Skill
    pub fn get_enabled(&self, ids: &[String]) -> Vec<Arc<dyn Skill>> {
        let skills = self.skills.read().unwrap();
        ids.iter()
            .filter_map(|id| skills.get(id).cloned())
            .collect()
    }

    /// 收集所有启用 Skill 的工具并注册到 ToolRegistry
    pub fn register_tools_to(&self, ids: &[String], registry: &mut ToolRegistry) {
        for skill in self.get_enabled(ids) {
            for tool in skill.tools() {
                registry.register(tool);
            }
        }
    }

    /// 生成所有启用 Skill 的 system prompt 追加内容
    pub fn build_prompt_addon(&self, ids: &[String]) -> String {
        let mut addons: Vec<String> = Vec::new();
        for skill in self.get_enabled(ids) {
            if let Some(addon) = skill.system_prompt_addon() {
                addons.push(format!("## Skill: {} ({})", skill.name(), skill.id()));
                addons.push(addon);
            }
        }
        if addons.is_empty() {
            String::new()
        } else {
            addons.join("\n\n")
        }
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 内置 Skill 注册
// ============================================================================

/// 创建包含所有内置 Skill 的 Registry
pub fn create_builtin_registry() -> SkillRegistry {
    let registry = SkillRegistry::new();
    registry.register(Arc::new(builtin::GitSkill::new()));
    registry.register(Arc::new(builtin::WebSearchSkill::new()));
    registry.register(Arc::new(builtin::CodeReviewSkill::new()));
    registry
}

// ============================================================================
// System Prompt Section 生成
// ============================================================================

/// 生成 Skills 注入段，用于追加到 system prompt 末尾
pub fn skills_prompt_section(registry: &SkillRegistry, enabled_ids: &[String]) -> String {
    let addon = registry.build_prompt_addon(enabled_ids);
    if addon.is_empty() {
        String::new()
    } else {
        format!(
            "## 已启用的专家技能 (Skills)\n\n{}\n\n{}",
            "以下是你当前可用的专家技能。在相关场景中，自动激活对应技能的行为模式。",
            addon
        )
    }
}

// ============================================================================
// 模块测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSkill;

    impl Skill for TestSkill {
        fn id(&self) -> &str {
            "test"
        }
        fn name(&self) -> &str {
            "Test Skill"
        }
        fn description(&self) -> &str {
            "A test skill"
        }
        fn system_prompt_addon(&self) -> Option<String> {
            Some("Test addon content".to_string())
        }
    }

    #[test]
    fn test_skill_registry() {
        let registry = SkillRegistry::new();
        registry.register(Arc::new(TestSkill));

        let ids = registry.list_ids();
        assert_eq!(ids, vec!["test"]);

        let addon = registry.build_prompt_addon(&["test".to_string()]);
        assert!(addon.contains("Test Skill"));
        assert!(addon.contains("Test addon content"));
    }

    #[test]
    fn test_empty_skills() {
        let registry = SkillRegistry::new();
        let addon = registry.build_prompt_addon(&[]);
        assert!(addon.is_empty());
    }
}
