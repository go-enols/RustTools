//! 内置 Skills 实现
//!
//! 提供常用的专家能力：
//! - `git` — Git 仓库操作专家
//! - `web_search` — 网络搜索专家

use super::Skill;

// ============================================================================
// Git Skill
// ============================================================================

/// Git 操作专家 Skill
///
/// 启用后，Agent 在处理代码相关任务时会：
/// - 主动检查 git 状态
/// - 遵循 git 最佳实践
/// - 在修改文件前确认分支状态
pub struct GitSkill;

impl GitSkill {
    pub fn new() -> Self {
        Self
    }
}

impl Skill for GitSkill {
    fn id(&self) -> &str {
        "git"
    }

    fn name(&self) -> &str {
        "Git 专家"
    }

    fn description(&self) -> &str {
        "Git 仓库操作专家，提供版本控制相关的最佳实践和状态检查"
    }

    fn system_prompt_addon(&self) -> Option<String> {
        Some(
            r#"你是一位 Git 版本控制专家。在处理代码任务时：

1. **修改前检查状态**：在执行任何文件修改前，先通过 `git status` 了解当前工作区状态
2. **分支意识**：确认当前分支，避免在 main/master 直接修改。如有需要，建议创建 feature 分支
3. **原子提交**：将相关修改组织为逻辑清晰的提交，每次提交只做一件事
4. **提交信息规范**：使用 conventional commits 格式（如 `feat:`, `fix:`, `docs:`, `refactor:`）
5. **冲突处理**：遇到合并冲突时，先分析冲突原因，再谨慎解决
6. **安全检查**：执行 `git reset --hard` 或强制推送前必须二次确认

可用命令：
- `git status` — 查看工作区状态
- `git log --oneline -n 10` — 查看最近提交历史
- `git diff` — 查看未暂存改动
- `git branch` — 查看分支列表"#
            .to_string(),
        )
    }
}

// ============================================================================
// Web Search Skill
// ============================================================================

/// 网络搜索专家 Skill
///
/// 启用后，Agent 在遇到知识截止、需要最新信息或验证事实时会：
/// - 主动使用网络搜索工具
/// - 综合多个来源的信息
/// - 标注信息来源和时间
pub struct WebSearchSkill;

impl WebSearchSkill {
    pub fn new() -> Self {
        Self
    }
}

impl Skill for WebSearchSkill {
    fn id(&self) -> &str {
        "web_search"
    }

    fn name(&self) -> &str {
        "网络搜索"
    }

    fn description(&self) -> &str {
        "网络搜索专家，在需要最新信息或事实验证时主动搜索网络"
    }

    fn system_prompt_addon(&self) -> Option<String> {
        Some(
            r#"你配备了网络搜索能力。在以下场景中主动使用搜索：

1. **知识截止**：当被询问的事件、技术或数据可能晚于你的知识截止日期时
2. **事实验证**：对不确定的事实、版本号、API 变更等进行验证
3. **最佳实践**：查询当前最新的框架用法、安全建议或社区共识
4. **错误排查**：遇到陌生错误信息时，搜索相关解决方案

搜索原则：
- 优先搜索官方文档和权威来源
- 对比多个来源交叉验证
- 在回答中标注信息来源和获取时间
- 如果搜索结果相互矛盾，说明分歧并给出你的判断

注意：网络搜索可能有延迟，优先使用本地工具和已有知识。"#
            .to_string(),
        )
    }
}

// ============================================================================
// Code Review Skill
// ============================================================================

/// 代码审查专家 Skill
///
/// 启用后，Agent 在生成或修改代码时会：
/// - 自动进行代码质量检查
/// - 遵循安全编码规范
/// - 关注性能影响
pub struct CodeReviewSkill;

impl CodeReviewSkill {
    pub fn new() -> Self {
        Self
    }
}

impl Skill for CodeReviewSkill {
    fn id(&self) -> &str {
        "code_review"
    }

    fn name(&self) -> &str {
        "代码审查"
    }

    fn description(&self) -> &str {
        "代码审查专家，在生成代码时自动检查质量、安全和性能"
    }

    fn system_prompt_addon(&self) -> Option<String> {
        Some(
            r#"你是一位严格的代码审查专家。在生成或修改代码时，自动执行以下检查：

**质量检查**：
- 变量/函数命名是否清晰、符合语言惯例
- 代码是否遵循 DRY 原则，避免重复
- 错误处理是否完整（不要忽略 Result/Error）
- 边界条件和空值检查

**安全检查**：
- 用户输入是否经过验证和清理
- 敏感操作是否有权限检查
- 是否存在注入、XSS、路径遍历等风险
- 硬编码密钥或密码

**性能检查**：
- 算法复杂度是否合理
- 是否存在不必要的内存分配或拷贝
- I/O 操作是否高效（批量而非逐条）
- 锁的使用是否合理（避免死锁和长时间持有）

如果发现问题，在输出代码前先列出问题清单和修复建议。"#
            .to_string(),
        )
    }
}
