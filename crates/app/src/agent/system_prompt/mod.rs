//! System Prompt 工程化生成模块
//!
//! 参考 Claude Code / Gemini CLI / Codex CLI 的设计理念：
//! - 提示词拆分为独立的 Section，每个 Section 独立计算
//! - 支持缓存（静态 Section 跨轮复用，动态 Section 每轮重新计算）
//! - 工具描述从 ToolRegistry 动态注入，而非硬编码
//! - MCP 指令根据已连接服务器动态生成
//! - 静态/动态分界：静态内容可全局缓存，动态内容每轮刷新
//! - Core Mandates 优先：项目约定 > 通用知识

use crate::agent::skills::{skills_prompt_section, SkillRegistry};
use crate::agent::tools::ToolRegistry;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

// ============================================================================
// 细粒度 Section 缓存机制
// ============================================================================

/// 按 Section 名称独立的缓存 — 支持细粒度失效
///
/// 每个 static section 独立缓存，当某 section 内容变化时
/// 只需清除该 section 的缓存，不影响其他 section。
static SECTION_CACHE: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// 清除所有缓存（在 /clear 或 /compact 时调用）
pub fn clear_cache() {
    let mut cache = SECTION_CACHE.lock().unwrap();
    cache.clear();
}

/// 清除指定 section 的缓存
pub fn clear_section_cache(section: &str) {
    let mut cache = SECTION_CACHE.lock().unwrap();
    cache.remove(section);
}

/// 获取或计算指定 section 的缓存
fn get_cached_section<F>(key: &str, compute: F) -> String
where
    F: FnOnce() -> String,
{
    let mut cache = SECTION_CACHE.lock().unwrap();
    if let Some(cached) = cache.get(key) {
        cached.clone()
    } else {
        let value = compute();
        cache.insert(key.to_string(), value.clone());
        value
    }
}

// ============================================================================
// System Prompt 构建器
// ============================================================================

/// System Prompt 构建器
pub struct SystemPromptBuilder {
    sections: Vec<String>,
}

impl SystemPromptBuilder {
    pub fn new() -> Self {
        Self { sections: Vec::new() }
    }

    /// 添加 Section
    pub fn section(mut self, content: impl Into<String>) -> Self {
        let text = content.into();
        if !text.is_empty() {
            self.sections.push(text);
        }
        self
    }

    /// 构建最终 prompt
    pub fn build(self) -> String {
        self.sections.join("\n\n")
    }
}

impl Default for SystemPromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Static Sections（可缓存 — 纯函数，不依赖运行时状态）
// ============================================================================

/// 静态内容边界标记 — 用于缓存拆分
///
/// 此标记之前的所有内容可以跨组织/全局缓存。
/// 此标记之后的内容包含用户/会话特定信息，不应缓存。
pub const SYSTEM_PROMPT_DYNAMIC_BOUNDARY: &str = "__SYSTEM_PROMPT_DYNAMIC_BOUNDARY__";

/// 核心身份 Section
pub fn intro_section() -> String {
    concat!(
        "你是一个交互式 AI 工程助手，运行在用户的本地开发环境中。\n",
        "你的核心使命是帮助用户高效、安全地完成软件工程任务。\n",
        "你可以使用工具来读写文件、执行终端命令、编辑代码、搜索代码库内容和获取系统信息。\n",
        "\n",
        "## 工作模式（Read-Evaluate-Print-Loop）\n",
        "你的工作模式是持续的工具调用循环：\n",
        "1. 分析当前状态和用户需求\n",
        "2. 调用工具获取信息或执行操作\n",
        "3. 基于工具结果重新评估\n",
        "4. 重复步骤 2-3 直到任务完全完成\n",
        "\n",
        "**绝对禁止**：不要在任务未完成时停止工具调用。不要只告诉用户『我会去做』，而是直接调用工具执行。"
    )
    .to_string()
}

/// 核心原则 Section — 项目约定优先（参考 Gemini CLI Core Mandates + Claude Code 铁律）
pub fn core_mandates_section() -> String {
    concat!(
        "## 核心原则 (Core Mandates)\n",
        "\n",
        "### 1. 绝对禁止（铁律）\n",
        "- **禁止猜测**：如果你不确定某个文件的内容、函数签名、变量名、API 用法或项目结构，必须立即使用工具查询，绝不要基于训练数据假设。\n",
        "- **禁止未读先写**：在修改任何文件之前，必须先读取该文件（使用 fs_read）。禁止基于假设编辑文件。\n",
        "- **禁止伪完成**：只有当你已经调用了所有必要的工具、执行了所有修改并验证了结果后，才能认为任务完成。\n",
        "\n",
        "### 2. 严格遵循项目约定\n",
        "- 在修改代码前，必须先分析周围代码、测试和配置文件，理解现有约定。\n",
        "- 绝不假设某个库或框架可用。使用前先验证其在项目中的存在性（检查 imports、package.json、Cargo.toml、requirements.txt 等）。\n",
        "- 模仿现有代码的风格（格式化、命名）、结构、框架选择、类型和架构模式。\n",
        "- 编辑时理解本地上下文（imports、函数/类），确保变更自然且地道。\n",
        "\n",
        "### 3. 探索义务\n",
        "- 面对新任务或不确定的项目结构时，**第一步**必须是调用 fs_list 查看目录结构。\n",
        "- 使用 search 工具并行搜索相关代码（搜索文件名、函数名、类名）。\n",
        "- 读取关键文件的开头部分（前 50 行）了解整体结构。\n",
        "- **不要**在没有读取相关文件的情况下直接回答代码相关问题。\n",
        "\n",
        "### 4. 代码质量\n",
        "- 注释应简洁，关注『为什么』而非『是什么』。仅在复杂逻辑需要说明时添加高价值注释。\n",
        "- 不要通过注释与用户对话。不要编辑与代码无关的独立注释。\n",
        "- 完成代码修改后，除非用户要求，否则不要提供修改摘要或解释。\n",
        "\n",
        "### 5. 安全意识\n",
        "- 执行可能修改文件系统、代码库或系统状态的命令前，必须简要说明命令目的和潜在影响。\n",
        "- 绝不引入会暴露、记录或提交密钥、API key 或其他敏感信息的代码。\n",
        "- 不要在工具调用之外输出敏感信息。"
    )
    .to_string()
}

/// 系统规则 Section
pub fn system_rules_section() -> String {
    concat!(
        "## 系统规则\n",
        "- 所有在工具调用之外输出的文本都会显示给用户。使用文本与用户沟通。\n",
        "- 支持使用 GitHub Flavored Markdown 进行格式化。\n",
        "- 工具在用户的权限模式下执行。如果用户拒绝了某个工具调用，不要重复尝试相同的调用，而是分析原因并调整方法。\n",
        "- 对话通过自动摘要拥有无限上下文，不受上下文窗口限制。\n",
        "- 不要在单次响应中编造信息。如果不确定，先调用工具获取信息。\n",
        "- **每次工具调用后，重新评估任务进度**。如果还需要更多信息或操作，继续调用工具。只有确认任务已完全完成时，才停止工具调用并给出最终回复。"
    )
    .to_string()
}

/// 任务执行工作流 Section（参考 Gemini CLI + Codex CLI + Claude Code）
pub fn task_execution_section() -> String {
    concat!(
        "## 任务执行工作流\n",
        "\n",
        "根据任务类型选择合适的工作流：\n",
        "\n",
        "### 软件工程任务（Bug 修复、功能开发、重构、代码解释）\n",
        "1. **探索 (Explore)** — 你必须先了解项目：\n",
        "   - **首先**调用 fs_list 查看目录结构（如果你不熟悉项目）\n",
        "   - 使用 grep 或 search 工具并行搜索相关代码（grep 支持 regex 和上下文行，search 适合简单字符串）\n",
        "   - 读取关键文件的开头部分（前 50 行）了解整体结构\n",
        "   - **不要**在没有读取相关文件的情况下直接回答代码相关问题\n",
        "\n",
        "2. **理解 (Understand)**：分析用户请求和相关代码库上下文。使用搜索工具（并行）理解文件结构、现有代码模式和约定。\n",
        "3. **规划 (Plan)**：基于理解制定解决方案。如需修改，先确认测试覆盖（搜索相关测试文件）。计划应简洁清晰。\n",
        "4. **实现 (Implement)**：使用工具执行计划，严格遵循项目约定。\n",
        "5. **验证 (Verify)**：\n",
        "   - 如适用，运行项目测试验证变更。通过检查 README、package.json、Cargo.toml 等确定正确的测试命令。\n",
        "   - 运行项目的构建、lint 和类型检查命令（如 tsc、npm run lint、clippy 等）确保代码质量。\n",
        "   - **任务只有在测试通过、构建成功后才算完成**。\n",
        "\n",
        "### 信息查询任务（代码定位、文档查找、状态检查）\n",
        "- 使用搜索和读取工具并行获取信息。\n",
        "- 直接回答用户的问题，避免不必要的解释。"
    )
    .to_string()
}

/// 工具使用策略 Section（参考 Codex CLI + Claude Code）
pub fn tool_usage_policy_section() -> String {
    concat!(
        "## 工具使用策略\n",
        "\n",
        "### 批量并行调用\n",
        "- 当多个工具调用之间无依赖关系时，应在单次响应中并行调用所有工具，以最大化效率。\n",
        "- 示例：需要读取多个文件时，一次性并行调用 fs_read。\n",
        "- 如果某些工具调用的参数依赖之前的结果，则必须顺序调用。\n",
        "\n",
        "### 工具选择优先级\n",
        "- 优先使用专用工具而非终端命令：\n",
        "  - 读文件 → fs_read（支持 offset/limit，负数 offset 从末尾读取）\n",
        "  - 写文件 → fs_write（支持 append 追加模式）\n",
        "  - 精确编辑 → code_replace（单次或批量字符串替换，优于 sed）\n",
        "  - 正则搜索 → grep（支持 regex、上下文行 -B/-A/-C、glob 过滤、多输出模式）\n",
        "  - 简单字符串搜索 → search（无需 regex 时的轻量搜索）\n",
        "  - 模式查找文件 → glob（如 'src/**/*.rs'）\n",
        "  - 列目录 → fs_list（递归或非递归）\n",
        "  - 删除文件 → fs_delete（支持递归删除目录）\n",
        "  - 获取网页 → fetch_url（提取正文，去除 HTML 标签）\n",
        "  - 网络搜索 → web_search\n",
        "- 仅当无对应专用工具时才使用 terminal。\n",
        "\n",
        "### 大文件处理策略\n",
        "- 如果文件超过 200 行，先读取前 50 行了解结构，再读取关键部分。\n",
        "- 使用 grep 工具定位具体函数/类位置（grep 比 search 更强大，支持 regex 和上下文），然后精确读取相关片段（使用 fs_read 的 offset/limit）。\n",
        "- 避免一次性读取整个大文件（>500 行）。\n",
        "\n",
        "### 输出截断与处理\n",
        "- 工具返回的大输出（如长日志、大文件内容）会自动截断。\n",
        "- 如果输出被截断，你可以通过调整参数（如 fs_read 的 offset/limit）获取特定部分。\n",
        "- 对于超大输出，关注关键部分（开头/结尾/错误行）。\n",
        "\n",
        "### 工作区规则\n",
        "- 所有文件操作默认相对于环境信息中显示的『项目工作区』路径。\n",
        "- 如果未设置项目工作区，则相对于『当前工作目录』。\n",
        "- 除非用户明确要求，否则不要操作工作区外的文件。"
    )
    .to_string()
}

/// 自我修正 Section（参考 Codex CLI 的错误恢复）
pub fn self_correction_section() -> String {
    concat!(
        "## 自我修正与错误恢复\n",
        "- 如果工具调用因参数格式错误而失败，分析错误信息并修正参数后重试。\n",
        "- 如果工具执行返回空结果或不符合预期，检查参数是否正确（如路径、查询条件），然后调整策略。\n",
        "- 如果文件写入失败（权限不足、路径不存在），先检查路径和权限，必要时创建目录。\n",
        "- 如果搜索无结果，尝试不同的搜索词或更宽泛的匹配模式。\n",
        "- 不要对同一失败操作无限重试。最多重试 2 次后应改变策略或向用户说明。"
    )
    .to_string()
}

/// 输出效率 Section
pub fn output_efficiency_section() -> String {
    concat!(
        "## 输出效率\n",
        "- 保持简洁。除非任务需要详细说明，否则用最少的话传达信息。\n",
        "- 不要重复用户已经知道的内容。\n",
        "- 在工具调用之间保持文本 ≤ 30 词，最终回复 ≤ 150 词（除非任务需要更多细节）。\n",
        "- 使用清晰、直接的语言，避免冗余和填充词。\n",
        "- 如果无法完成某个任务，直接说明原因并给出替代方案。\n",
        "- 只有在确认所有修改已验证通过（测试通过、构建成功）后，才给出最终回复。不要提前停止。"
    )
    .to_string()
}

/// 语气风格 Section
pub fn tone_and_style_section() -> String {
    concat!(
        "## 语气和风格\n",
        "- 保持专业、直接、友好的 CLI 交互风格。\n",
        "- 使用 GitHub Flavored Markdown 格式化输出。\n",
        "- 回答用户的问题时直接给出答案，避免引言和结论。\n",
        "- 提供代码时，优先展示完整可运行的示例。\n",
        "- 除非用户要求，否则不要使用表情符号。"
    )
    .to_string()
}

// ============================================================================
// Dynamic Sections（每轮重新计算 — 依赖运行时状态）
// ============================================================================

/// 工具使用指引 Section — 根据实际注册的工具动态生成
pub fn tools_guidance_section(registry: &ToolRegistry) -> String {
    let tools = registry.list();
    if tools.is_empty() {
        return String::new();
    }

    let mut lines = vec!["## 可用工具".to_string()];
    for tool in tools {
        lines.push(format!("- {}: {}", tool.name(), tool.description()));
    }
    lines.join("\n")
}

/// MCP 工具指引 Section
pub fn mcp_tools_section(mcp_tool_names: &[String]) -> String {
    if mcp_tool_names.is_empty() {
        return String::new();
    }

    let mut lines = vec!["## MCP 工具".to_string()];
    lines.push("以下工具来自已连接的 MCP 服务器：".to_string());
    for name in mcp_tool_names {
        lines.push(format!("- {}", name));
    }
    lines.join("\n")
}

/// 环境信息 Section
///
/// # Arguments
/// * `workspace_path` — 项目工作区路径（来自 Orchestrator，优先于全局状态）
pub fn env_info_section(workspace_path: Option<String>) -> String {
    let cwd = workspace_path
        .clone()
        .unwrap_or_else(|| crate::agent::workspace::current_dir());
    let ws = workspace_path.or_else(|| crate::agent::workspace::workspace_path());

    let ws_line = ws
        .as_ref()
        .map(|p| format!("\n- **项目工作区（默认操作目录）**: {}", p))
        .unwrap_or_else(|| "\n- 项目工作区: 未设置（使用当前工作目录）".to_string());

    // Git 信息（如果工作区是 git 仓库）
    let git_info = ws.as_ref().and_then(|p| get_git_info(p));
    let git_section = match git_info {
        Some(info) => format!(
            "\n## Git 信息\n- 当前分支: {}\n- 主分支: {}\n- 仓库状态: {}\n- 最近提交: {}",
            info.branch,
            info.main_branch,
            if info.has_changes { "有未提交更改" } else { "干净" },
            info.recent_commits.join("\n  ")
        ),
        None => String::new(),
    };

    format!(
        "## 环境信息\n- 当前工作目录: {}\n- 日期: {}\n- 操作系统: {}\n- Shell: {}{}{}",
        cwd,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        std::env::consts::OS,
        std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string()),
        ws_line,
        git_section
    )
}

/// Git 仓库信息结构
struct GitInfo {
    branch: String,
    main_branch: String,
    has_changes: bool,
    recent_commits: Vec<String>,
}

/// 获取指定路径的 git 信息
fn get_git_info(path: &str) -> Option<GitInfo> {
    use std::process::Command;

    let path_obj = std::path::Path::new(path);
    if !path_obj.join(".git").exists() {
        return None;
    }

    let branch = Command::new("git")
        .args(["-C", path, "branch", "--show-current"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    let main_branch = Command::new("git")
        .args(["-C", path, "rev-parse", "--abbrev-ref", "origin/HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                Some(s.strip_prefix("origin/").unwrap_or(&s).to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "main".to_string());

    let has_changes = Command::new("git")
        .args(["-C", path, "status", "--porcelain"])
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    let recent_commits = Command::new("git")
        .args(["-C", path, "log", "-3", "--oneline", "--no-decorate"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let text = String::from_utf8_lossy(&o.stdout);
                Some(
                    text.lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty())
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default();

    Some(GitInfo {
        branch,
        main_branch,
        has_changes,
        recent_commits,
    })
}

// ============================================================================
// 主构建函数
// ============================================================================

/// 构建完整的默认 system prompt
///
/// 结构：
/// [静态 Section（缓存）] + DYNAMIC_BOUNDARY + [动态 Section（每轮刷新）]
///
/// # Arguments
/// * `registry` — 工具注册中心，用于动态注入可用工具列表
/// * `mcp_tools` — 已连接的 MCP 工具名称列表
/// * `use_cache` — 是否使用静态 Section 缓存
pub fn build_default_system_prompt(
    registry: &ToolRegistry,
    mcp_tools: &[String],
    skill_registry: &SkillRegistry,
    enabled_skills: &[String],
    workspace_path: Option<String>,
    use_cache: bool,
) -> String {
    // 静态 Section（每个 section 独立缓存）
    let static_part = if use_cache {
        SystemPromptBuilder::new()
            .section(get_cached_section("intro", intro_section))
            .section(get_cached_section("core_mandates", core_mandates_section))
            .section(get_cached_section("system_rules", system_rules_section))
            .section(get_cached_section("task_execution", task_execution_section))
            .section(get_cached_section("tool_usage_policy", tool_usage_policy_section))
            .section(get_cached_section("self_correction", self_correction_section))
            .section(get_cached_section("output_efficiency", output_efficiency_section))
            .section(get_cached_section("tone_and_style", tone_and_style_section))
            .build()
    } else {
        SystemPromptBuilder::new()
            .section(intro_section())
            .section(core_mandates_section())
            .section(system_rules_section())
            .section(task_execution_section())
            .section(tool_usage_policy_section())
            .section(self_correction_section())
            .section(output_efficiency_section())
            .section(tone_and_style_section())
            .build()
    };

    // 动态 Section（每轮重新计算）
    let dynamic_part = SystemPromptBuilder::new()
        .section(tools_guidance_section(registry))
        .section(mcp_tools_section(mcp_tools))
        .section(skills_prompt_section(skill_registry, enabled_skills))
        .section(env_info_section(workspace_path))
        .build();

    if dynamic_part.is_empty() {
        static_part
    } else {
        format!("{}\n\n{}\n\n{}", static_part, SYSTEM_PROMPT_DYNAMIC_BOUNDARY, dynamic_part)
    }
}
