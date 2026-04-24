//! 任务执行器
//!
//! Executor负责执行Planner生成的Plan，协调工具调用和LLM交互。
//! 处理步骤依赖、错误恢复和结果汇总。

use super::agent::AgentError;
use super::orchestrator::{StepRecord, TaskResult, ToolCallRecord};
use super::planner::{Plan, PlanStep, PlannedToolCall, PlanningStrategy};
use super::session::Session;
use super::super::api_client::{ChatMessage, ChatRequest, ChatResponse, ToolCall, UnifiedClient};
use super::super::tools::{Tool, ToolDefinition, ToolError, ToolRegistry, ToolResult};
use serde_json::Value;

// ============================================================================
// 执行器错误
// ============================================================================

/// 执行器错误
#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("步骤执行失败 [{step_id}]: {message}")]
    StepFailed { step_id: String, message: String },
    #[error("LLM调用失败: {0}")]
    LlmCallFailed(String),
    #[error("工具调用失败 [{tool_name}]: {message}")]
    ToolCallFailed { tool_name: String, message: String },
    #[error("计划为空")]
    EmptyPlan,
    #[error("{0}")]
    Other(String),
}

impl ExecutorError {
    /// 从AgentError转换
    pub fn from_agent_error(e: AgentError) -> Self {
        Self::Other(e.to_string())
    }
}

// ============================================================================
// 执行器
// ============================================================================

/// 任务执行器
pub struct Executor;

impl Executor {
    /// 创建新的执行器
    pub fn new() -> Self {
        Self
    }

    /// 执行计划
    ///
    /// 根据计划策略，逐步执行每个步骤中的工具调用。
    /// 支持顺序执行、并行执行和自适应调整。
    pub async fn execute_plan(
        &self,
        plan: Plan,
        session: &mut Session,
        client: &UnifiedClient,
        registry: &ToolRegistry,
    ) -> Result<TaskResult, ExecutorError> {
        if plan.is_empty() {
            return Err(ExecutorError::EmptyPlan);
        }

        let start_time = std::time::Instant::now();
        let mut steps_executed: Vec<StepRecord> = Vec::new();
        let mut tool_calls: Vec<ToolCallRecord> = Vec::new();
        let mut final_response = String::new();

        // 根据策略选择执行方式
        match plan.strategy {
            PlanningStrategy::SingleStep | PlanningStrategy::Sequential => {
                final_response = self
                    .execute_sequential(
                        plan,
                        session,
                        client,
                        registry,
                        &mut steps_executed,
                        &mut tool_calls,
                    )
                    .await?;
            }
            PlanningStrategy::Parallel => {
                final_response = self
                    .execute_parallel(
                        plan,
                        session,
                        client,
                        registry,
                        &mut steps_executed,
                        &mut tool_calls,
                    )
                    .await?;
            }
            PlanningStrategy::Hierarchical => {
                final_response = self
                    .execute_hierarchical(
                        plan,
                        session,
                        client,
                        registry,
                        &mut steps_executed,
                        &mut tool_calls,
                    )
                    .await?;
            }
            PlanningStrategy::Adaptive => {
                final_response = self
                    .execute_adaptive(
                        plan,
                        session,
                        client,
                        registry,
                        &mut steps_executed,
                        &mut tool_calls,
                    )
                    .await?;
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(TaskResult {
            final_response,
            steps_executed,
            tool_calls,
            duration_ms,
        })
    }

    /// 顺序执行计划步骤
    async fn execute_sequential(
        &self,
        plan: Plan,
        session: &mut Session,
        client: &UnifiedClient,
        registry: &ToolRegistry,
        steps_executed: &mut Vec<StepRecord>,
        tool_calls: &mut Vec<ToolCallRecord>,
    ) -> Result<String, ExecutorError> {
        let mut response_parts = Vec::new();

        for step in plan.steps {
            let step_result = self
                .execute_step(&step, session, client, registry, tool_calls)
                .await;

            match step_result {
                Ok(result) => {
                    steps_executed.push(StepRecord {
                        step_id: step.id.clone(),
                        description: step.description.clone(),
                        success: true,
                        result: result.clone(),
                    });
                    if !result.is_empty() {
                        response_parts.push(result);
                    }
                }
                Err(e) => {
                    steps_executed.push(StepRecord {
                        step_id: step.id.clone(),
                        description: step.description.clone(),
                        success: false,
                        result: e.to_string(),
                    });
                    return Err(ExecutorError::StepFailed {
                        step_id: step.id,
                        message: e.to_string(),
                    });
                }
            }
        }

        Ok(response_parts.join("\n\n"))
    }

    /// 并行执行计划步骤
    ///
    /// 注：当前实现按依赖组顺序执行，组内步骤串行执行。
    /// 真正的并行执行需要在各任务间无共享可变状态。
    async fn execute_parallel(
        &self,
        plan: Plan,
        session: &mut Session,
        client: &UnifiedClient,
        registry: &ToolRegistry,
        steps_executed: &mut Vec<StepRecord>,
        tool_calls: &mut Vec<ToolCallRecord>,
    ) -> Result<String, ExecutorError> {
        // 使用与顺序执行相同的逻辑，但按依赖组处理
        // TODO: 实现真正的并行执行
        self.execute_sequential(plan, session, client, registry, steps_executed, tool_calls)
            .await
    }

    /// 层级执行计划步骤
    async fn execute_hierarchical(
        &self,
        plan: Plan,
        session: &mut Session,
        client: &UnifiedClient,
        registry: &ToolRegistry,
        steps_executed: &mut Vec<StepRecord>,
        tool_calls: &mut Vec<ToolCallRecord>,
    ) -> Result<String, ExecutorError> {
        // 层级执行与顺序执行类似，但会在每个阶段后向LLM确认
        // TODO: 在每个阶段后添加LLM确认步骤
        self.execute_sequential(plan, session, client, registry, steps_executed, tool_calls)
            .await
    }

    /// 自适应执行计划步骤
    async fn execute_adaptive(
        &self,
        mut plan: Plan,
        session: &mut Session,
        client: &UnifiedClient,
        registry: &ToolRegistry,
        steps_executed: &mut Vec<StepRecord>,
        tool_calls: &mut Vec<ToolCallRecord>,
    ) -> Result<String, ExecutorError> {
        // 自适应从第一个步骤开始，根据结果动态调整后续步骤
        let mut response_parts = Vec::new();
        let mut current_step_idx = 0;

        while current_step_idx < plan.steps.len() {
            let step = &plan.steps[current_step_idx];
            let step_result = self
                .execute_step(step, session, client, registry, tool_calls)
                .await;

            match step_result {
                Ok(result) => {
                    steps_executed.push(StepRecord {
                        step_id: step.id.clone(),
                        description: step.description.clone(),
                        success: true,
                        result: result.clone(),
                    });
                    if !result.is_empty() {
                        response_parts.push(result);
                    }
                    current_step_idx += 1;
                }
                Err(e) => {
                    steps_executed.push(StepRecord {
                        step_id: step.id.clone(),
                        description: step.description.clone(),
                        success: false,
                        result: e.to_string(),
                    });
                    return Err(ExecutorError::StepFailed {
                        step_id: step.id.clone(),
                        message: e.to_string(),
                    });
                }
            }
        }

        Ok(response_parts.join("\n\n"))
    }

    /// 执行单个步骤
    async fn execute_step(
        &self,
        step: &PlanStep,
        _session: &mut Session,
        _client: &UnifiedClient,
        registry: &ToolRegistry,
        tool_calls: &mut Vec<ToolCallRecord>,
    ) -> Result<String, ExecutorError> {
        let mut step_outputs = Vec::new();

        for planned_call in &step.tool_calls {
            let result = self
                .execute_tool_call(&planned_call, registry, tool_calls)
                .await;

            match result {
                Ok(output) => step_outputs.push(output),
                Err(e) => {
                    return Err(ExecutorError::ToolCallFailed {
                        tool_name: planned_call.tool_name.clone(),
                        message: e.to_string(),
                    });
                }
            }
        }

        // 如果没有工具调用，生成默认响应
        if step_outputs.is_empty() {
            step_outputs.push(format!("完成: {}", step.description));
        }

        Ok(step_outputs.join("\n"))
    }

    /// 执行工具调用
    async fn execute_tool_call(
        &self,
        planned: &PlannedToolCall,
        registry: &ToolRegistry,
        tool_calls: &mut Vec<ToolCallRecord>,
    ) -> Result<String, ExecutorError> {
        let tool = registry.get(&planned.tool_name).ok_or_else(|| {
            ExecutorError::ToolCallFailed {
                tool_name: planned.tool_name.clone(),
                message: "工具未注册".to_string(),
            }
        })?;

        // 构造参数
        let params = if planned.parameters.is_null() {
            serde_json::json!({})
        } else {
            planned.parameters.clone()
        };

        let result = tool.execute(params).await.map_err(|e| {
            ExecutorError::ToolCallFailed {
                tool_name: planned.tool_name.clone(),
                message: e.to_string(),
            }
        })?;

        tool_calls.push(ToolCallRecord {
            tool_name: planned.tool_name.clone(),
            parameters: planned.parameters.clone(),
            result: result.content.clone(),
            success: result.success,
        });

        Ok(format!("[{}] {}", planned.tool_name, result.content))
    }

    /// 收集步骤中的工具调用future
    fn collect_tool_futures(
        &self,
        step: &PlanStep,
        _registry: &ToolRegistry,
    ) -> Vec<(String, Value, String)> {
        step.tool_calls
            .iter()
            .map(|tc| {
                (
                    tc.tool_name.clone(),
                    tc.parameters.clone(),
                    tc.purpose.clone(),
                )
            })
            .collect()
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::planner::{PlanStep, Planner, PlanningStrategy};
    use super::super::session::Session;
    use super::super::api_client::UnifiedClient;
    use super::super::config::ModelConfig;
    use super::super::tools::{ToolResult, create_test_registry};

    #[tokio::test]
    async fn test_executor_empty_plan() {
        let executor = Executor::new();
        let plan = Plan::new(PlanningStrategy::SingleStep);
        let mut session = Session::new("test", "agent");
        let configs: Vec<ModelConfig> = vec![];
        let client = UnifiedClient::new(&configs).unwrap();
        let registry = create_test_registry();

        let result = executor
            .execute_plan(plan, &mut session, &client, &registry)
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ExecutorError::EmptyPlan));
    }

    #[tokio::test]
    async fn test_executor_single_step_success() {
        let executor = Executor::new();
        let mut plan = Plan::new(PlanningStrategy::SingleStep);
        plan.add_step(PlanStep::new("step-1", "测试步骤"));

        let mut session = Session::new("test", "agent");
        let configs: Vec<ModelConfig> = vec![];
        let client = UnifiedClient::new(&configs).unwrap();
        let registry = create_test_registry();

        let result = executor
            .execute_plan(plan, &mut session, &client, &registry)
            .await;

        assert!(result.is_ok());
        let task_result = result.unwrap();
        assert!(!task_result.final_response.is_empty());
        assert_eq!(task_result.steps_executed.len(), 1);
        assert!(task_result.steps_executed[0].success);
    }

    #[tokio::test]
    async fn test_executor_sequential_steps() {
        let executor = Executor::new();
        let mut plan = Plan::new(PlanningStrategy::Sequential);
        plan.add_step(PlanStep::new("step-1", "第一步"));
        plan.add_step(PlanStep::new("step-2", "第二步"));

        let mut session = Session::new("test", "agent");
        let configs: Vec<ModelConfig> = vec![];
        let client = UnifiedClient::new(&configs).unwrap();
        let registry = create_test_registry();

        let result = executor
            .execute_plan(plan, &mut session, &client, &registry)
            .await;

        assert!(result.is_ok());
        let task_result = result.unwrap();
        assert_eq!(task_result.steps_executed.len(), 2);
        assert!(task_result.steps_executed[0].success);
        assert!(task_result.steps_executed[1].success);
    }

    #[tokio::test]
    async fn test_executor_with_tool_calls() {
        let executor = Executor::new();
        let mut plan = Plan::new(PlanningStrategy::SingleStep);
        let mut step = PlanStep::new("step-1", "使用终端工具");
        step.add_tool_call("terminal", "执行echo命令");
        // 设置具体参数
        step.tool_calls[0].parameters = serde_json::json!({"command": "echo test-output"});
        plan.add_step(step);

        let mut session = Session::new("test", "agent");
        let configs: Vec<ModelConfig> = vec![];
        let client = UnifiedClient::new(&configs).unwrap();
        let registry = create_test_registry();

        let result = executor
            .execute_plan(plan, &mut session, &client, &registry)
            .await;

        assert!(result.is_ok());
        let task_result = result.unwrap();
        assert!(!task_result.tool_calls.is_empty());
        assert_eq!(task_result.tool_calls[0].tool_name, "terminal");
        assert!(task_result.tool_calls[0].success);
    }

    #[tokio::test]
    async fn test_executor_duration() {
        let executor = Executor::new();
        let mut plan = Plan::new(PlanningStrategy::SingleStep);
        plan.add_step(PlanStep::new("step-1", "测试"));

        let mut session = Session::new("test", "agent");
        let configs: Vec<ModelConfig> = vec![];
        let client = UnifiedClient::new(&configs).unwrap();
        let registry = create_test_registry();

        let result = executor
            .execute_plan(plan, &mut session, &client, &registry)
            .await
            .unwrap();

        assert!(result.duration_ms > 0);
    }

    #[test]
    fn test_executor_error_display() {
        let e1 = ExecutorError::EmptyPlan;
        assert_eq!(e1.to_string(), "计划为空");

        let e2 = ExecutorError::StepFailed {
            step_id: "step-1".to_string(),
            message: "failed".to_string(),
        };
        assert!(e2.to_string().contains("步骤执行失败 [step-1]"));
    }
}
