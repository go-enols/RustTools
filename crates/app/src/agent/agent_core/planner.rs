//! 任务规划器
//!
//! Planner根据用户输入和Agent配置，生成执行计划。
//! 支持多种规划策略：单步、顺序、并行、层级、自适应。

use super::agent::{Agent, Capability};
use super::super::tools::ToolDefinition;
use serde::{Deserialize, Serialize};

// ============================================================================
// 规划策略
// ============================================================================

/// 规划策略枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanningStrategy {
    /// 单步执行 — 简单任务直接完成
    SingleStep,
    /// 顺序执行 — 按依赖顺序逐步执行
    Sequential,
    /// 并行执行 — 独立子任务并行执行
    Parallel,
    /// 层级执行 — 树形分解，逐层细化
    Hierarchical,
    /// 自适应执行 — 根据中间结果动态调整
    Adaptive,
}

impl Default for PlanningStrategy {
    fn default() -> Self {
        Self::SingleStep
    }
}

// ============================================================================
// 计划工具调用
// ============================================================================

/// 计划中的工具调用
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlannedToolCall {
    /// 工具名称
    pub tool_name: String,
    /// 预期参数（可以是部分参数）
    pub parameters: serde_json::Value,
    /// 调用目的说明
    pub purpose: String,
}

// ============================================================================
// 计划步骤
// ============================================================================

/// 计划步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// 步骤唯一ID
    pub id: String,
    /// 步骤描述
    pub description: String,
    /// 此步骤要调用的工具
    pub tool_calls: Vec<PlannedToolCall>,
    /// 依赖的其他步骤ID
    pub depends_on: Vec<String>,
}

impl PlanStep {
    /// 创建新的计划步骤
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            tool_calls: Vec::new(),
            depends_on: Vec::new(),
        }
    }

    /// 添加工具调用
    pub fn add_tool_call(&mut self, name: impl Into<String>, purpose: impl Into<String>) {
        self.tool_calls.push(PlannedToolCall {
            tool_name: name.into(),
            parameters: serde_json::Value::Null,
            purpose: purpose.into(),
        });
    }

    /// 添加依赖
    pub fn add_dependency(&mut self, step_id: impl Into<String>) {
        self.depends_on.push(step_id.into());
    }
}

// ============================================================================
// 计划
// ============================================================================

/// 执行计划
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Plan {
    /// 规划策略
    pub strategy: PlanningStrategy,
    /// 计划步骤列表
    pub steps: Vec<PlanStep>,
}

impl Plan {
    /// 创建新的计划
    pub fn new(strategy: PlanningStrategy) -> Self {
        Self {
            strategy,
            steps: Vec::new(),
        }
    }

    /// 添加步骤
    pub fn add_step(&mut self, step: PlanStep) {
        self.steps.push(step);
    }

    /// 获取可并行执行的步骤组
    pub fn parallel_groups(&self) -> Vec<Vec<&PlanStep>> {
        let mut groups: Vec<Vec<&PlanStep>> = Vec::new();
        let mut assigned: std::collections::HashSet<&str> = std::collections::HashSet::new();

        while assigned.len() < self.steps.len() {
            let mut group: Vec<&PlanStep> = Vec::new();
            for step in &self.steps {
                if assigned.contains(step.id.as_str()) {
                    continue;
                }
                // 检查所有依赖是否已分配
                let all_deps_met = step
                    .depends_on
                    .iter()
                    .all(|dep| assigned.contains(dep.as_str()));
                if all_deps_met {
                    group.push(step);
                }
            }
            if group.is_empty() {
                // 存在循环依赖，跳出
                break;
            }
            for step in &group {
                assigned.insert(&step.id);
            }
            groups.push(group);
        }

        groups
    }

    /// 检查计划是否为空
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// 获取步骤数量
    pub fn len(&self) -> usize {
        self.steps.len()
    }
}

// ============================================================================
// 规划器
// ============================================================================

/// 任务规划器
pub struct Planner;

impl Planner {
    /// 根据用户输入和Agent配置生成执行计划
    ///
    /// 当前实现使用基于规则的简单规划：
    /// - 代码相关任务 → 顺序/层级策略
    /// - 分析任务 → 单步策略
    /// - 多文件操作 → 顺序策略
    pub fn plan(
        user_input: &str,
        agent: &Agent,
        _available_tools: &[ToolDefinition],
    ) -> Plan {
        let input_lower = user_input.to_lowercase();

        // 根据任务类型选择策略
        let strategy = Self::select_strategy(&input_lower, agent);

        let mut plan = Plan::new(strategy);

        // 基于策略生成步骤
        match strategy {
            PlanningStrategy::SingleStep => {
                plan.add_step(PlanStep::new(
                    "step-1",
                    format!("直接处理: {}", &user_input[..user_input.len().min(50)]),
                ));
            }
            PlanningStrategy::Sequential => {
                Self::build_sequential_plan(&mut plan, user_input, agent);
            }
            PlanningStrategy::Parallel => {
                Self::build_parallel_plan(&mut plan, user_input, agent);
            }
            PlanningStrategy::Hierarchical => {
                Self::build_hierarchical_plan(&mut plan, user_input, agent);
            }
            PlanningStrategy::Adaptive => {
                // 自适应从单步开始，执行器会根据中间结果调整
                let mut step = PlanStep::new(
                    "step-1",
                    format!("开始自适应处理: {}", &user_input[..user_input.len().min(50)]),
                );
                step.add_tool_call("fs_read", "分析当前上下文");
                plan.add_step(step);
            }
        }

        plan
    }

    /// 选择规划策略
    fn select_strategy(input_lower: &str, agent: &Agent) -> PlanningStrategy {
        // 如果Agent有Planning能力，优先使用自适应策略
        if agent.has_capability(&Capability::Planning) {
            return PlanningStrategy::Adaptive;
        }

        // 简单分析类任务
        if input_lower.contains("分析")
            || input_lower.contains("解释")
            || input_lower.contains("总结")
            || input_lower.contains("review")
        {
            return PlanningStrategy::SingleStep;
        }

        // 代码生成任务
        if input_lower.contains("创建")
            || input_lower.contains("生成")
            || input_lower.contains("写")
            || input_lower.contains("build")
            || input_lower.contains("implement")
        {
            if input_lower.contains("多")
                || input_lower.contains("多个")
                || input_lower.contains("项目")
                || input_lower.contains("结构")
            {
                return PlanningStrategy::Hierarchical;
            }
            return PlanningStrategy::Sequential;
        }

        // 涉及多个独立任务
        if input_lower.contains("和") || input_lower.contains("并且") || input_lower.contains("同时") {
            return PlanningStrategy::Parallel;
        }

        // 默认顺序执行
        PlanningStrategy::Sequential
    }

    /// 构建顺序计划
    fn build_sequential_plan(plan: &mut Plan, user_input: &str, agent: &Agent) {
        let mut step1 = PlanStep::new("step-1", "分析需求和当前环境");
        if agent.tools.contains(&"fs_read".to_string()) {
            step1.add_tool_call("fs_read", "读取相关文件了解上下文");
        }
        if agent.tools.contains(&"fs_list".to_string()) {
            step1.add_tool_call("fs_list", "列出目录结构");
        }
        plan.add_step(step1);

        let mut step2 = PlanStep::new("step-2", format!("执行主要任务: {}", &user_input[..user_input.len().min(40)]));
        step2.add_dependency("step-1");
        if agent.tools.contains(&"terminal".to_string()) {
            step2.add_tool_call("terminal", "执行必要命令");
        }
        if agent.tools.contains(&"code_edit".to_string()) {
            step2.add_tool_call("code_edit", "修改代码");
        }
        plan.add_step(step2);

        let mut step3 = PlanStep::new("step-3", "验证结果");
        step3.add_dependency("step-2");
        if agent.tools.contains(&"fs_read".to_string()) {
            step3.add_tool_call("fs_read", "验证修改后的文件");
        }
        plan.add_step(step3);
    }

    /// 构建并行计划
    fn build_parallel_plan(plan: &mut Plan, _user_input: &str, agent: &Agent) {
        let mut step1 = PlanStep::new("step-1", "并行分析多个方面");
        if agent.tools.contains(&"fs_search".to_string()) {
            step1.add_tool_call("fs_search", "搜索相关代码");
        }
        plan.add_step(step1);

        let mut step2a = PlanStep::new("step-2a", "执行任务A");
        step2a.add_dependency("step-1");
        plan.add_step(step2a);

        let mut step2b = PlanStep::new("step-2b", "执行任务B");
        step2b.add_dependency("step-1");
        plan.add_step(step2b);

        let mut step3 = PlanStep::new("step-3", "汇总结果");
        step3.add_dependency("step-2a");
        step3.add_dependency("step-2b");
        plan.add_step(step3);
    }

    /// 构建层级计划
    fn build_hierarchical_plan(plan: &mut Plan, user_input: &str, agent: &Agent) {
        // 高层规划
        let mut step1 = PlanStep::new("step-1", "项目结构分析");
        if agent.tools.contains(&"fs_list".to_string()) {
            step1.add_tool_call("fs_list", "分析项目目录结构");
        }
        plan.add_step(step1);

        // 设计阶段
        let mut step2 = PlanStep::new("step-2", "设计实现方案");
        step2.add_dependency("step-1");
        plan.add_step(step2);

        // 实施阶段
        let mut step3 = PlanStep::new(
            "step-3",
            format!("实施: {}", &user_input[..user_input.len().min(30)]),
        );
        step3.add_dependency("step-2");
        if agent.tools.contains(&"code_edit".to_string()) {
            step3.add_tool_call("code_edit", "编写代码");
        }
        if agent.tools.contains(&"fs_write".to_string()) {
            step3.add_tool_call("fs_write", "创建新文件");
        }
        plan.add_step(step3);

        // 验证阶段
        let mut step4 = PlanStep::new("step-4", "验证和测试");
        step4.add_dependency("step-3");
        if agent.tools.contains(&"terminal".to_string()) {
            step4.add_tool_call("terminal", "运行测试");
        }
        plan.add_step(step4);
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::agent::{Agent, AgentDefinition, Capability};
    use super::super::super::config::ModelConfig;

    fn create_test_agent(tools: Vec<&str>, capabilities: Vec<Capability>) -> Agent {
        let def = AgentDefinition {
            name: "测试Agent".to_string(),
            description: "用于测试".to_string(),
            system_prompt: "测试".to_string(),
            model_config: ModelConfig::default(),
            tools: tools.into_iter().map(String::from).collect(),
            capabilities,
            ..Default::default()
        };
        Agent::new("test-agent", def)
    }

    #[test]
    fn test_plan_single_step() {
        let agent = create_test_agent(vec![], vec![]);
        let tools: Vec<ToolDefinition> = vec![];
        let plan = Planner::plan("总结一下这段代码", &agent, &tools);

        assert_eq!(plan.strategy, PlanningStrategy::SingleStep);
        assert!(!plan.is_empty());
    }

    #[test]
    fn test_plan_sequential() {
        let agent = create_test_agent(
            vec!["fs_read", "code_edit"],
            vec![],
        );
        let tools: Vec<ToolDefinition> = vec![];
        let plan = Planner::plan("帮我修改这个函数的实现", &agent, &tools);

        assert_eq!(plan.strategy, PlanningStrategy::Sequential);
        assert!(plan.len() >= 2);

        // 检查依赖关系
        for (i, step) in plan.steps.iter().enumerate() {
            if i > 0 {
                assert!(!step.depends_on.is_empty() || step.id == "step-1");
            }
        }
    }

    #[test]
    fn test_plan_hierarchical() {
        let agent = create_test_agent(
            vec!["fs_write", "code_edit", "terminal"],
            vec![],
        );
        let tools: Vec<ToolDefinition> = vec![];
        let plan = Planner::plan("帮我创建多个文件和完整的项目结构", &agent, &tools);

        assert_eq!(plan.strategy, PlanningStrategy::Hierarchical);
        assert!(plan.len() >= 3);
    }

    #[test]
    fn test_plan_adaptive_with_planning_capability() {
        let agent = create_test_agent(vec![], vec![Capability::Planning]);
        let tools: Vec<ToolDefinition> = vec![];
        let plan = Planner::plan("随便什么任务", &agent, &tools);

        assert_eq!(plan.strategy, PlanningStrategy::Adaptive);
    }

    #[test]
    fn test_plan_parallel() {
        let agent = create_test_agent(vec![], vec![]);
        let tools: Vec<ToolDefinition> = vec![];
        let plan = Planner::plan("同时分析A和B两个模块", &agent, &tools);

        assert_eq!(plan.strategy, PlanningStrategy::Parallel);
        assert!(plan.len() >= 2);
    }

    #[test]
    fn test_plan_parallel_groups() {
        let mut plan = Plan::new(PlanningStrategy::Sequential);
        plan.add_step(PlanStep::new("step-1", "第一步"));

        let mut step2 = PlanStep::new("step-2", "第二步");
        step2.add_dependency("step-1");
        plan.add_step(step2);

        let mut step3 = PlanStep::new("step-3", "第三步");
        step3.add_dependency("step-2");
        plan.add_step(step3);

        let groups = plan.parallel_groups();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].len(), 1);
        assert_eq!(groups[1].len(), 1);
        assert_eq!(groups[2].len(), 1);
    }

    #[test]
    fn test_plan_parallel_groups_with_parallel_steps() {
        let mut plan = Plan::new(PlanningStrategy::Parallel);
        plan.add_step(PlanStep::new("step-1", "第一步"));

        let mut step2a = PlanStep::new("step-2a", "任务A");
        step2a.add_dependency("step-1");
        plan.add_step(step2a);

        let mut step2b = PlanStep::new("step-2b", "任务B");
        step2b.add_dependency("step-1");
        plan.add_step(step2b);

        let mut step3 = PlanStep::new("step-3", "汇总");
        step3.add_dependency("step-2a");
        step3.add_dependency("step-2b");
        plan.add_step(step3);

        let groups = plan.parallel_groups();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].len(), 1); // step-1
        assert_eq!(groups[1].len(), 2); // step-2a, step-2b (并行)
        assert_eq!(groups[2].len(), 1); // step-3
    }

    #[test]
    fn test_plan_step_add_tool_call() {
        let mut step = PlanStep::new("test", "测试步骤");
        step.add_tool_call("fs_read", "读取文件");
        step.add_tool_call("code_edit", "编辑代码");

        assert_eq!(step.tool_calls.len(), 2);
        assert_eq!(step.tool_calls[0].tool_name, "fs_read");
        assert_eq!(step.tool_calls[0].purpose, "读取文件");
    }

    #[test]
    fn test_plan_step_add_dependency() {
        let mut step = PlanStep::new("step-2", "第二步");
        step.add_dependency("step-1");
        assert_eq!(step.depends_on, vec!["step-1"]);
    }

    #[test]
    fn test_plan_serialization() {
        let mut plan = Plan::new(PlanningStrategy::Sequential);
        let mut step = PlanStep::new("step-1", "测试步骤");
        step.add_tool_call("fs_read", "读取");
        plan.add_step(step);

        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("Sequential"));
        assert!(json.contains("step-1"));

        let deserialized: Plan = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.strategy, PlanningStrategy::Sequential);
        assert_eq!(deserialized.len(), 1);
    }
}
