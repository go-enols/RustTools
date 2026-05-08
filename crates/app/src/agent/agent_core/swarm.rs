//! Agent 蜂群（Swarm）协调器
//!
//! SwarmCoordinator 实现多 Agent 并行协作：
//! 1. 接收用户任务，分析已有 Agent 能力，动态选择最适合的 Agent 组合
//! 2. 为每个选中的 Agent 分配子任务（利用其已有能力/工具）
//! 3. 选中的 Agent 并行独立执行（复用 ReAct 循环）
//! 4. 汇总所有 Agent 结果，由 Coordinator 生成最终回复
//!
//! 参考 Claude Code 的 ULTRAPLAN 和 VERIFICATION_AGENT 理念。

use super::agent::{Agent, AgentError};
use super::executor::{Executor, ExecutionEvent};
use super::orchestrator::{TaskResult, ToolCallRecord};
use super::session::Session;
use super::super::api_client::{ChatMessage, ChatRequest, UnifiedClient};
use super::super::tools::ToolRegistry;
use futures_util::future::join_all;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;

// ============================================================================
// 子任务定义
// ============================================================================

/// 子任务 — 由 Coordinator 分解产生
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    /// 子任务ID
    pub id: String,
    /// 子任务描述
    pub description: String,
    /// 分配给 Worker 的提示词
    pub worker_prompt: String,
    /// 需要的工具列表
    pub required_tools: Vec<String>,
    /// 依赖的其他子任务ID
    pub depends_on: Vec<String>,
}

/// Agent 任务分配 — 由 Coordinator 动态调度产生
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentAssignment {
    /// 独立任务ID（用于依赖追踪，区别于 agent_id）
    task_id: String,
    /// 分配的已有 Agent ID
    agent_id: String,
    /// 子任务描述
    description: String,
    /// 依赖的其他任务ID列表
    depends_on: Vec<String>,
}

// ============================================================================
// Worker Agent 结果
// ============================================================================

/// Worker Agent 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    /// 子任务ID
    pub subtask_id: String,
    /// Worker Agent ID
    pub worker_id: String,
    /// 执行结果内容
    pub result: String,
    /// 是否成功
    pub success: bool,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
    /// 工具调用记录
    pub tool_calls: Vec<ToolCallRecord>,
}

// ============================================================================
// 蜂群配置
// ============================================================================

/// 蜂群执行配置
#[derive(Debug, Clone)]
pub struct SwarmConfig {
    /// 最大 Worker 数量
    pub max_workers: usize,
    /// 最大并发 Worker 数量
    pub max_concurrent_workers: usize,
    /// 是否启用任务分解
    pub enable_decomposition: bool,
    /// Coordinator 系统提示词
    pub coordinator_system_prompt: String,
    /// Worker 执行超时（毫秒）
    pub worker_timeout_ms: u64,
    /// Worker 失败最大重试次数
    pub max_retries: u32,
    /// 是否启用结果验证
    pub verification_enabled: bool,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            max_workers: 5,
            max_concurrent_workers: 3,
            enable_decomposition: true,
            coordinator_system_prompt: DEFAULT_COORDINATOR_PROMPT.to_string(),
            worker_timeout_ms: 120_000,
            max_retries: 1,
            verification_enabled: false,
        }
    }
}

const DEFAULT_COORDINATOR_PROMPT: &str = r#"你是蜂群协调专家（Swarm Coordinator）。你的职责是综合分析多个 Worker Agent 的并行执行结果，通过多阶段质量管控生成最终回复。

## 核心职责

### 阶段 1: 冲突检测与解决 (Conflict Resolution)
- 检查不同 Worker 的结果是否存在矛盾或不一致
- 如果发现冲突，分析可能的原因（工具调用参数不同、执行时机不同、数据版本不同）
- 基于工具调用记录判断哪个结果更可靠
- 明确说明冲突及你的判断依据

### 阶段 2: 质量门控 (Quality Gate)
- 验证工具调用结果的合理性（文件路径是否存在、命令输出是否符合预期）
- 检查是否有 Worker 遗漏了关键步骤
- 识别可能的误报或漏报
- 标记置信度低的结果

### 阶段 3: 审议与批判 (Deliberation & Critique)
- 评估每个 Worker 结果的完整性和准确性
- 识别结果之间的依赖关系和逻辑链条
- 检查是否有信息缺口需要补充
- 对不一致的结果进行交叉验证

### 阶段 4: 最终综合 (Final Synthesis)
- 将多个 Worker 的结果有机整合，避免简单拼接
- 去重：相同信息只保留一次
- 补全：利用多个 Worker 的结果互相补充，形成完整画面
- 失败处理：说明失败原因及影响，基于成功结果继续生成回复

## 回复格式

```
## 执行概况
- 成功: X 个 Worker | 失败: Y 个 Worker | 总计: Z 个
- [如有失败] 失败 Worker 及原因: ...

## 质量评估
- 结果一致性: [高/中/低]
- 置信度: [高/中/低]
- [如有] 冲突说明: ...

## 整合结果
[详细的最终答案，结构清晰]

## 注意事项
[如有冲突、不确定之处、遗漏或建议，在此说明]
```

## 重要原则
- 不要编造 Worker 未提供的信息
- 对不确定的内容明确标注"不确定"或"需进一步验证"
- 优先使用有工具调用记录支持的结果
- 保持客观，不要过度推断
- 如果关键 Worker 失败导致结果不完整，明确告知用户影响范围"#;

// ============================================================================
// Worker System Prompt 构建
// ============================================================================

/// 构建 Swarm Worker 的 system prompt
///
/// 核心要求：必须包含工作区路径信息，否则 Worker 不知道应该在哪个目录操作。
/// - 如果 agent 有自定义 system_prompt，在其后附加环境信息
/// - 如果 agent 没有自定义 prompt，构建一个包含基本指令和环境信息的 prompt
fn build_worker_system_prompt(
    agent: &Agent,
    registry: &ToolRegistry,
    workspace_path: Option<String>,
) -> String {
    let env_info = format!(
        "## 环境信息\n- 项目工作区（默认操作目录）: {}\n- 当前工作目录: {}\n- 日期: {}\n- 所有文件操作都相对于上述工作区路径。",
        workspace_path.as_deref().unwrap_or("未设置"),
        workspace_path.clone().unwrap_or_else(|| crate::agent::workspace::current_dir()),
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    // 工具列表
    let tools_info = if !agent.tools.is_empty() {
        let mut lines = vec!["## 可用工具".to_string()];
        for tool_id in &agent.tools {
            if let Some(tool) = registry.get(tool_id) {
                lines.push(format!("- {}: {}", tool.name(), tool.description()));
            }
        }
        lines.join("\n")
    } else {
        String::new()
    };

    if agent.system_prompt.is_empty() {
        // Agent 没有自定义 prompt，构建一个完整的默认 prompt
        format!(
            "你是一个 AI 工程助手，负责执行分配给你的子任务。\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}",
            crate::agent::system_prompt::task_execution_section(),
            crate::agent::system_prompt::tool_usage_policy_section(),
            crate::agent::system_prompt::output_efficiency_section(),
            env_info,
            tools_info
        )
    } else {
        // Agent 有自定义 prompt，附加环境信息和工具列表
        format!(
            "{}\n\n{}\n\n{}",
            agent.system_prompt,
            env_info,
            tools_info
        )
    }
}

// ============================================================================
// Swarm 协调器
// ============================================================================

/// Agent 蜂群协调器
pub struct SwarmCoordinator {
    config: SwarmConfig,
    executor: Executor,
}

impl SwarmCoordinator {
    /// 创建新的蜂群协调器
    pub fn new() -> Self {
        Self {
            config: SwarmConfig::default(),
            executor: Executor::new(),
        }
    }

    /// 使用自定义配置创建
    pub fn with_config(config: SwarmConfig) -> Self {
        Self {
            config,
            executor: Executor::new(),
        }
    }

    /// 检测任务依赖图中的循环依赖
    /// 使用 DFS 拓扑排序检测，返回第一个发现的循环路径
    fn detect_cycles(assignments: &[AgentAssignment]) -> Result<(), String> {
        let mut graph: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        let mut in_degree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        
        // 初始化所有节点
        for a in assignments {
            in_degree.entry(a.task_id.clone()).or_insert(0);
            graph.entry(a.task_id.clone()).or_default();
        }
        
        // 构建图
        for a in assignments {
            for dep in &a.depends_on {
                graph.entry(dep.clone()).or_default().push(a.task_id.clone());
                *in_degree.entry(a.task_id.clone()).or_insert(0) += 1;
                
                // 检查依赖指向的任务是否存在
                if !assignments.iter().any(|x| x.task_id == *dep) {
                    return Err(format!(
                        "任务 '{}' 依赖不存在的任务 '{}'",
                        a.task_id, dep
                    ));
                }
            }
        }
        
        // Kahn 算法拓扑排序
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(k, _)| k.clone())
            .collect();
        let mut visited = 0usize;
        
        while let Some(node) = queue.pop() {
            visited += 1;
            if let Some(neighbors) = graph.get(&node) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
        }
        
        if visited == assignments.len() {
            Ok(())
        } else {
            // 找出参与循环的节点
            let cycle_nodes: Vec<String> = in_degree
                .iter()
                .filter(|(_, &d)| d > 0)
                .map(|(k, _)| k.clone())
                .collect();
            Err(format!("循环涉及任务: {:?}", cycle_nodes))
        }
    }

    /// 执行蜂群任务
    ///
    /// 完整流程：
    /// 1. 分析已有 Agent 能力，动态选择最适合的 Agent 组合（如果启用）
    /// 2. 为每个选中的已有 Agent 分配子任务，并行执行
    /// 3. 汇总结果并生成最终回复
    pub async fn execute_swarm<F, C>(
        &self,
        user_input: &str,
        base_agent: &Agent,
        available_agents: &[Agent],
        client: &UnifiedClient,
        registry: &ToolRegistry,
        model_id: &str,
        workspace_path: Option<String>,
        mut on_event: F,
        check_cancel: C,
    ) -> Result<TaskResult, AgentError>
    where
        F: FnMut(SwarmEvent) + Send + Clone,
        C: Fn() -> bool + Send + Clone,
    {
        let start_time = std::time::Instant::now();

        on_event(SwarmEvent::CoordinatorThinking(
            "正在分析任务并调度合适的 Agent...".to_string(),
        ));

        // 步骤 1：动态调度 — 从已有 Agent 中选择并分配任务
        let (assignments, fallback_reason): (Vec<AgentAssignment>, Option<String>) = if self.config.enable_decomposition && available_agents.len() > 1 {
            match self.plan_with_agents(user_input, available_agents, client, model_id).await {
                Ok(plans) if plans.len() > 1 => (plans, None),
                Ok(plans) if plans.len() == 1 => {
                    (plans, Some("任务较简单，使用单个 Agent 执行更高效".to_string()))
                }
                Err(e) => {
                    (vec![AgentAssignment {
                        task_id: format!("task-{}", uuid::Uuid::new_v4()),
                        agent_id: base_agent.id.clone(),
                        description: user_input.to_string(),
                        depends_on: vec![],
                    }], Some(format!("Agent 调度失败，回退到单 Agent 模式: {}", e)))
                }
                _ => {
                    (vec![AgentAssignment {
                        task_id: format!("task-{}", uuid::Uuid::new_v4()),
                        agent_id: base_agent.id.clone(),
                        description: user_input.to_string(),
                        depends_on: vec![],
                    }], Some("任务无需分解，使用单 Agent 执行".to_string()))
                }
            }
        } else {
            (vec![AgentAssignment {
                task_id: format!("task-{}", uuid::Uuid::new_v4()),
                agent_id: base_agent.id.clone(),
                description: user_input.to_string(),
                depends_on: vec![],
            }], None)
        };

        // 通知用户回退原因（如果发生了回退）
        if let Some(reason) = fallback_reason {
            on_event(SwarmEvent::CoordinatorThinking(
                format!("【模式切换】{}", reason)
            ));
        }

        // 循环依赖检测
        if let Err(cycle) = Self::detect_cycles(&assignments) {
            on_event(SwarmEvent::CoordinatorError(format!(
                "检测到循环依赖: {}", cycle
            )));
            return Err(AgentError::ExecutionError(format!(
                "任务调度失败: 存在循环依赖 — {}", cycle
            )));
        }

        // 构建子任务列表（用于事件通知）
        let subtask_descriptions: Vec<String> = assignments
            .iter()
            .map(|a| {
                let agent = available_agents
                    .iter()
                    .find(|ag| ag.id == a.agent_id)
                    .or(Some(base_agent));
                format!(
                    "[{}] {}",
                    agent.map(|ag| ag.name.as_str()).unwrap_or("Agent"),
                    a.description
                )
            })
            .collect();

        on_event(SwarmEvent::TaskDecomposed {
            subtask_count: assignments.len(),
            descriptions: subtask_descriptions,
        });

        // 步骤 2：按依赖组并行执行
        let mut all_results: Vec<WorkerResult> = Vec::new();
        let mut completed_ids: Vec<String> = Vec::new();
        let all_original_assignments = assignments.clone(); // 保留原始任务用于重试
        let mut remaining: Vec<AgentAssignment> = assignments;
        let mut retry_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

        // 进度追踪原子计数器
        let total_workers = all_original_assignments.len();
        let completed_count = Arc::new(AtomicUsize::new(0));
        let failed_count = Arc::new(AtomicUsize::new(0));
        let running_count = Arc::new(AtomicUsize::new(0));

        let _emit_progress = |on_event: &mut F, completed: usize, failed: usize, running: usize| {
            on_event(SwarmEvent::TaskProgress {
                completed,
                total: total_workers,
                failed,
                running,
            });
        };

        // 并发控制信号量
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_workers));

        while !remaining.is_empty() {
            if check_cancel() {
                on_event(SwarmEvent::CoordinatorError("任务已取消".to_string()));
                return Err(AgentError::ExecutionError("任务已取消".to_string()));
            }

            // 找出当前可执行的分配（无依赖或依赖已完成）
            let executable: Vec<AgentAssignment> = remaining
                .iter()
                .filter(|a| {
                    a.depends_on.is_empty()
                        || a.depends_on.iter().all(|dep| completed_ids.contains(dep))
                })
                .cloned()
                .collect();

            if executable.is_empty() {
                let remaining_ids: Vec<String> = remaining.iter().map(|a| a.task_id.clone()).collect();
                on_event(SwarmEvent::CoordinatorError(format!(
                    "循环依赖或无法解析的依赖关系，剩余任务: {:?}", remaining_ids
                )));
                return Err(AgentError::ExecutionError(
                    "任务执行失败: 存在循环依赖或无法解析的依赖".to_string()
                ));
            }

            // 为因依赖未满足而等待的任务发出事件
            for waiting_task in &remaining {
                if !waiting_task.depends_on.is_empty() {
                    let unmet_deps: Vec<String> = waiting_task.depends_on.iter()
                        .filter(|dep| !completed_ids.contains(dep))
                        .cloned()
                        .collect();
                    if !unmet_deps.is_empty() {
                        let waiting_agent = available_agents.iter()
                            .find(|a| a.id == waiting_task.agent_id)
                            .or(Some(base_agent));
                        on_event(SwarmEvent::DependencyWaiting {
                            worker_id: waiting_agent.map(|a| a.id.clone()).unwrap_or_else(|| waiting_task.agent_id.clone()),
                            task_id: waiting_task.task_id.clone(),
                            waiting_for: unmet_deps,
                        });
                    }
                }
            }

            // 从 remaining 中移除即将执行的（使用 task_id 匹配）
            let executable_task_ids: std::collections::HashSet<String> =
                executable.iter().map(|e| e.task_id.clone()).collect();
            remaining.retain(|a| !executable_task_ids.contains(&a.task_id));

            // 并行执行当前组的子任务（受信号量限制）
            let ws_path = workspace_path.clone();
            let worker_futures = executable.into_iter().map(|assignment| {
                let sem = Arc::clone(&semaphore);
                let mut on_event_worker = on_event.clone();
                let _assignment_task_id = assignment.task_id.clone();
                let timeout_ms = self.config.worker_timeout_ms;
                let max_retries = self.config.max_retries;
                let retry_count = *retry_counts.get(&assignment.task_id).unwrap_or(&0);
                let completed_count_w = Arc::clone(&completed_count);
                let failed_count_w = Arc::clone(&failed_count);
                let running_count_w = Arc::clone(&running_count);
                let total_workers_w = total_workers;
                let ws_path_worker = ws_path.clone();
                let check_cancel_worker = check_cancel.clone();

                async move {
                    // 获取信号量许可
                    let _permit = sem.acquire().await;

                    // 从已有 Agent 中找到对应的 Agent，找不到则回退到 base_agent
                    let agent = available_agents
                        .iter()
                        .find(|a| a.id == assignment.agent_id)
                        .cloned()
                        .unwrap_or_else(|| base_agent.clone());

                    let mut worker_session = Session::new(
                        format!("swarm-{}", assignment.task_id),
                        &assignment.agent_id,
                    );

                    // 构建 Worker 的 system prompt：优先使用 agent 自定义 prompt，
                    // 但必须注入工作区路径等环境信息，否则 Worker 不知道操作哪个目录
                    let worker_system_prompt = build_worker_system_prompt(
                        &agent,
                        registry,
                        ws_path_worker,
                    );
                    worker_session.add_system_message(worker_system_prompt);
                    worker_session.add_user_message(assignment.description.clone());

                    let executor = Executor::new();
                    let assignment_desc = assignment.description.clone();
                    let task_id_for_result = assignment.task_id.clone();

                    let running = running_count_w.fetch_add(1, Ordering::Relaxed) + 1;
                    let completed = completed_count_w.load(Ordering::Relaxed);
                    let failed = failed_count_w.load(Ordering::Relaxed);
                    on_event_worker(SwarmEvent::TaskProgress {
                        completed,
                        total: total_workers_w,
                        failed,
                        running,
                    });

                    on_event_worker(SwarmEvent::WorkerStarted {
                        worker_id: agent.id.clone(),
                        subtask_id: task_id_for_result.clone(),
                        description: assignment_desc,
                    });

                    if retry_count > 0 {
                        on_event_worker(SwarmEvent::WorkerRetry {
                            worker_id: agent.id.clone(),
                            task_id: task_id_for_result.clone(),
                            attempt: retry_count + 1,
                            max_attempts: max_retries + 1,
                        });
                    }

                    let worker_start = std::time::Instant::now();

                    // 使用超时包装 execute_react
                    let result = tokio::time::timeout(
                        std::time::Duration::from_millis(timeout_ms),
                        executor.execute_react(
                            &mut worker_session,
                            client,
                            registry,
                            &agent,
                            model_id,
                            |event| {
                                let swarm_event = match event {
                                    ExecutionEvent::Thinking(msg) => {
                                        SwarmEvent::WorkerThinking {
                                            worker_id: agent.id.clone(),
                                            task_id: task_id_for_result.clone(),
                                            message: msg,
                                        }
                                    }
                                    ExecutionEvent::ContentChunk(chunk) => {
                                        SwarmEvent::WorkerResponseChunk {
                                            worker_id: agent.id.clone(),
                                            task_id: task_id_for_result.clone(),
                                            chunk,
                                        }
                                    }
                                    ExecutionEvent::ToolCall { id, name, params } => {
                                        SwarmEvent::WorkerToolCall {
                                            worker_id: agent.id.clone(),
                                            task_id: task_id_for_result.clone(),
                                            id,
                                            name,
                                            params,
                                        }
                                    }
                                    ExecutionEvent::ToolResult {
                                        id,
                                        name,
                                        result,
                                        success,
                                    } => SwarmEvent::WorkerToolResult {
                                        worker_id: agent.id.clone(),
                                        task_id: task_id_for_result.clone(),
                                        id,
                                        name,
                                        result,
                                        success,
                                    },
                                    ExecutionEvent::ToolCacheHit { id: _, name } => {
                                        SwarmEvent::ToolCacheHit {
                                            worker_id: agent.id.clone(),
                                            task_id: task_id_for_result.clone(),
                                            tool_name: name,
                                        }
                                    }
                                    ExecutionEvent::TokenUsage { .. } => {
                                        return; // Worker token 使用量不单独传递
                                    }
                                    ExecutionEvent::Complete => {
                                        return;
                                    }
                                    ExecutionEvent::Error(msg) => SwarmEvent::WorkerError {
                                        worker_id: agent.id.clone(),
                                        task_id: task_id_for_result.clone(),
                                        error: msg,
                                    },
                                };
                                on_event_worker(swarm_event);
                            },
                            check_cancel_worker.clone(),
                        )
                    ).await;

                    let duration_ms = worker_start.elapsed().as_millis() as u64;

                    match result {
                        Ok(Ok(r)) => {
                            on_event_worker(SwarmEvent::WorkerCompleted {
                                worker_id: agent.id.clone(),
                                task_id: task_id_for_result.clone(),
                                result: r.final_response.clone(),
                            });
                            let running = running_count_w.fetch_sub(1, Ordering::Relaxed) - 1;
                            let completed = completed_count_w.fetch_add(1, Ordering::Relaxed) + 1;
                            let failed = failed_count_w.load(Ordering::Relaxed);
                            on_event_worker(SwarmEvent::TaskProgress {
                                completed,
                                total: total_workers_w,
                                failed,
                                running,
                            });
                            WorkerResult {
                                subtask_id: task_id_for_result,
                                worker_id: agent.id,
                                result: r.final_response,
                                success: true,
                                duration_ms,
                                tool_calls: r.tool_calls,
                            }
                        }
                        Ok(Err(e)) => {
                            on_event_worker(SwarmEvent::WorkerError {
                                worker_id: agent.id.clone(),
                                task_id: task_id_for_result.clone(),
                                error: e.to_string(),
                            });
                            let running = running_count_w.fetch_sub(1, Ordering::Relaxed) - 1;
                            let _ = failed_count_w.fetch_add(1, Ordering::Relaxed) + 1;
                            let completed = completed_count_w.load(Ordering::Relaxed);
                            let failed = failed_count_w.load(Ordering::Relaxed);
                            on_event_worker(SwarmEvent::TaskProgress {
                                completed,
                                total: total_workers_w,
                                failed,
                                running,
                            });
                            WorkerResult {
                                subtask_id: task_id_for_result,
                                worker_id: agent.id,
                                result: e.to_string(),
                                success: false,
                                duration_ms,
                                tool_calls: vec![],
                            }
                        }
                        Err(_) => {
                            let err_msg = format!("Worker 执行超时（{}ms）", timeout_ms);
                            on_event_worker(SwarmEvent::WorkerError {
                                worker_id: agent.id.clone(),
                                task_id: task_id_for_result.clone(),
                                error: err_msg.clone(),
                            });
                            let running = running_count_w.fetch_sub(1, Ordering::Relaxed) - 1;
                            let _ = failed_count_w.fetch_add(1, Ordering::Relaxed) + 1;
                            let completed = completed_count_w.load(Ordering::Relaxed);
                            let failed = failed_count_w.load(Ordering::Relaxed);
                            on_event_worker(SwarmEvent::TaskProgress {
                                completed,
                                total: total_workers_w,
                                failed,
                                running,
                            });
                            WorkerResult {
                                subtask_id: task_id_for_result,
                                worker_id: agent.id,
                                result: err_msg,
                                success: false,
                                duration_ms,
                                tool_calls: vec![],
                            }
                        }
                    }
                }
            });

            let group_results = join_all(worker_futures).await;

            // 收集失败且需要重试的任务，待本轮结束后重新加入 remaining
            let mut tasks_to_retry: Vec<AgentAssignment> = Vec::new();
            
            for wr in &group_results {
                if !wr.success {
                    let retries = retry_counts.entry(wr.subtask_id.clone()).or_insert(0);
                    if *retries < self.config.max_retries {
                        *retries += 1;
                        // 从原始 assignments 中找到对应任务
                        if let Some(original) = all_original_assignments.iter().find(|a| a.task_id == wr.subtask_id) {
                            tasks_to_retry.push(original.clone());
                        }
                        // 需要重试的任务不加入 completed_ids，避免依赖它的任务提前执行
                        continue;
                    } else {
                        on_event(SwarmEvent::WorkerFailedPermanently {
                            worker_id: wr.worker_id.clone(),
                            task_id: wr.subtask_id.clone(),
                            error: wr.result.clone(),
                        });
                    }
                }
                completed_ids.push(wr.subtask_id.clone());
            }
            all_results.extend(group_results);
            
            // 将重试任务重新加入 remaining 队列
            remaining.extend(tasks_to_retry);
        }

        on_event(SwarmEvent::AllWorkersComplete);

        // 步骤 3：Coordinator 汇总 Agent 结果生成最终回复
        let mut coordinator_session = Session::new("swarm-coordinator", base_agent.id.clone());
        coordinator_session.add_system_message(self.config.coordinator_system_prompt.clone());

        let mut summary_prompt = format!(
            "用户原始任务：{}\n\n## Worker 执行结果\n",
            user_input
        );

        // 按成功/失败分组，先展示成功结果
        let successful: Vec<&WorkerResult> = all_results.iter().filter(|wr| wr.success).collect();
        let failed: Vec<&WorkerResult> = all_results.iter().filter(|wr| !wr.success).collect();

        if !successful.is_empty() {
            summary_prompt.push_str("\n### 成功执行的 Worker\n");
            for wr in successful {
                summary_prompt.push_str(&format!(
                    "\n**Worker**: {} | **任务**: {} | **耗时**: {}ms\n",
                    wr.worker_id, wr.subtask_id, wr.duration_ms
                ));
                // 结果摘要（截断避免过长）
                let result_summary = if wr.result.len() > 800 {
                    format!("{}... [省略 {} 字符]", &wr.result[..800], wr.result.len() - 800)
                } else {
                    wr.result.clone()
                };
                summary_prompt.push_str(&format!("**结果**: {}\n", result_summary));
                // 工具调用记录（结构化、压缩）
                if !wr.tool_calls.is_empty() {
                    summary_prompt.push_str("**工具调用**: ");
                    let tool_summaries: Vec<String> = wr.tool_calls.iter().map(|tc| {
                        let param_summary = tc.parameters.to_string();
                        let param_short = if param_summary.len() > 80 {
                            format!("{}...", &param_summary[..80])
                        } else {
                            param_summary
                        };
                        format!("{}({})[{}]", tc.tool_name, param_short, if tc.success { "✓" } else { "✗" })
                    }).collect();
                    summary_prompt.push_str(&tool_summaries.join(", "));
                    summary_prompt.push('\n');
                }
            }
        }

        if !failed.is_empty() {
            summary_prompt.push_str("\n### 执行失败的 Worker\n");
            for wr in failed {
                summary_prompt.push_str(&format!(
                    "\n**Worker**: {} | **任务**: {}\n**失败原因**: {}\n",
                    wr.worker_id, wr.subtask_id, wr.result
                ));
                if !wr.tool_calls.is_empty() {
                    summary_prompt.push_str("**失败前已执行工具**: ");
                    let tool_list: Vec<String> = wr.tool_calls.iter()
                        .map(|tc| format!("{}[{}]", tc.tool_name, if tc.success { "✓" } else { "✗" }))
                        .collect();
                    summary_prompt.push_str(&tool_list.join(", "));
                    summary_prompt.push('\n');
                }
            }
            summary_prompt.push_str("\n> 注意：上述 Worker 执行失败，请在最终回复中评估失败对整体结果的影响。\n");
        }

        summary_prompt.push_str("\n请按照 Coordinator 系统提示中的四阶段流程（冲突检测→质量门控→审议批判→最终综合）生成最终回复。");
        coordinator_session.add_user_message(summary_prompt);

        let executor = Executor::new();
        let coordinator_result = executor
            .execute_react(
                &mut coordinator_session,
                client,
                registry,
                base_agent,
                model_id,
                |event| {
                    let swarm_event = match event {
                        ExecutionEvent::Thinking(msg) => {
                            SwarmEvent::CoordinatorThinking(msg)
                        }
                        ExecutionEvent::ContentChunk(chunk) => {
                            SwarmEvent::CoordinatorResponseChunk(chunk)
                        }
                        ExecutionEvent::ToolCall { id, name, params } => {
                            SwarmEvent::WorkerToolCall {
                                worker_id: "coordinator".to_string(),
                                task_id: "coordinator".to_string(),
                                id,
                                name,
                                params,
                            }
                        }
                        ExecutionEvent::ToolResult {
                            id,
                            name,
                            result,
                            success,
                        } => SwarmEvent::WorkerToolResult {
                            worker_id: "coordinator".to_string(),
                            task_id: "coordinator".to_string(),
                            id,
                            name,
                            result,
                            success,
                        },
                        ExecutionEvent::TokenUsage { .. } => {
                            // Coordinator 的 token 使用量不单独传递（已计入 coordinator_session）
                            return;
                        }
                        ExecutionEvent::ToolCacheHit { .. } => {
                            // Coordinator 的缓存命中不单独发送事件
                            return;
                        }
                        ExecutionEvent::Complete => SwarmEvent::AllWorkersComplete,
                        ExecutionEvent::Error(msg) => SwarmEvent::CoordinatorError(msg),
                    };
                    on_event(swarm_event);
                },
                check_cancel.clone(),
            )
            .await
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(TaskResult {
            final_response: coordinator_result.final_response,
            steps_executed: coordinator_result.steps_executed,
            tool_calls: coordinator_result.tool_calls,
            duration_ms,
        })
    }

    /// 用 LLM 动态调度已有 Agent：分析任务并选择最适合的 Agent 组合
    async fn plan_with_agents(
        &self,
        user_input: &str,
        available_agents: &[Agent],
        client: &UnifiedClient,
        model_id: &str,
    ) -> Result<Vec<AgentAssignment>, AgentError> {
        // 构建可用 Agent 描述
        let agent_descriptions: Vec<String> = available_agents
            .iter()
            .map(|a| {
                let caps = a.capabilities.iter()
                    .map(|c| c.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "- ID: {} | 名称: {} | 描述: {} | 能力: [{}] | 工具: [{}]",
                    a.id,
                    a.name,
                    a.description,
                    if caps.is_empty() { "通用" } else { &caps },
                    a.tools.join(", ")
                )
            })
            .collect();

        let planner_prompt = format!(
            r#"你是任务调度专家。请分析以下用户任务，并从已有 Agent 列表中选择最适合的 Agent 来协作完成。

## 用户任务
{}

## 可用 Agent
{}

## 任务分解原则
- 每个子任务应该是**独立、可验证的**，有明确的完成标准
- 避免过度分解：单个函数的小修改不应单独成为一个子任务
- 优先并行：无依赖的子任务应标记为并行（depends_on 为空）
- 利用 Agent 特长：根据每个 Agent 的 capabilities 和 tools 分配最适合的任务
- 对于代码修改类任务，建议分解为：分析/探索 → 修改实现 → 验证测试
- 对于分析类任务，可分解为并行探索不同模块/文件

## 输出要求
请严格按以下 JSON 格式返回调度计划。**只返回 JSON 数组，不要包含 markdown 代码块标记、不要添加任何解释文字**：
[
  {{
    "task_id": "task-1",
    "agent_id": "agent-id-1",
    "description": "分配给该 Agent 的具体子任务描述，应包含明确的完成标准",
    "depends_on": []
  }},
  {{
    "task_id": "task-2",
    "agent_id": "agent-id-2",
    "description": "另一个子任务描述",
    "depends_on": ["task-1"]
  }}
]

注意：
- task_id 必须是全局唯一标识符（如 task-1, task-2），用于依赖追踪
- depends_on 中引用的是 task_id，不是 agent_id
- 如果任务简单，只需 1 个 Agent（返回单元素数组）
- Agent 不存在时分配最接近的 Agent，执行阶段会自动回退
"#,
            user_input,
            agent_descriptions.join("\n")
        );

        let request = ChatRequest {
            model: String::new(),
            messages: vec![ChatMessage::User {
                content: planner_prompt.into(),
            }],
            temperature: Some(0.2),
            max_tokens: Some(3000),
            tools: None,
            stream: false,
        };

        let response = client
            .chat(model_id, request)
            .await
            .map_err(|e| AgentError::ExecutionError(format!("Agent 调度失败: {}", e)))?;

        // 解析 JSON 响应
        let content = response.content.trim();
        let json_str = Self::extract_json_array(content);

        let mut assignments: Vec<AgentAssignment> = serde_json::from_str(&json_str)
            .map_err(|e| AgentError::ExecutionError(format!("解析调度计划失败: {}", e)))?;

        // 为缺失 task_id 的 assignment 自动生成 UUID
        for (i, a) in assignments.iter_mut().enumerate() {
            if a.task_id.is_empty() {
                a.task_id = format!("task-{}-{}", i, uuid::Uuid::new_v4());
            }
        }

        // 对于 LLM 推荐但不在可用列表中的 agent，保留其任务分解，
        // 执行阶段会自动回退到 base_agent
        Ok(assignments)
    }

    /// 从文本中提取 JSON 数组
    fn extract_json_array(text: &str) -> String {
        let trimmed = text.trim();

        // 尝试直接解析
        if serde_json::from_str::<Vec<serde_json::Value>>(trimmed).is_ok() {
            return trimmed.to_string();
        }

        // 尝试提取 ```json ... ``` 块（支持多行内容）
        if let Some(start) = trimmed.find("```json") {
            let after_tag = &trimmed[start + 7..];
            if let Some(end) = after_tag.find("```") {
                let inner = after_tag[..end].trim();
                if serde_json::from_str::<Vec<serde_json::Value>>(inner).is_ok() {
                    return inner.to_string();
                }
            }
        }

        // 尝试提取 ``` ... ``` 块（无语言标记）
        if let Some(start) = trimmed.find("```") {
            let after_tag = &trimmed[start + 3..];
            if let Some(end) = after_tag.find("```") {
                let inner = after_tag[..end].trim();
                if serde_json::from_str::<Vec<serde_json::Value>>(inner).is_ok() {
                    return inner.to_string();
                }
            }
        }

        // 尝试提取最外层 [ ... ] 匹配对
        if let Some(start) = trimmed.find('[') {
            // 找与第一个 '[' 匹配的最后一个 ']'（考虑嵌套）
            let mut depth = 0;
            let mut end_pos = None;
            for (i, c) in trimmed[start..].char_indices() {
                match c {
                    '[' => depth += 1,
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            end_pos = Some(start + i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(end) = end_pos {
                let candidate = &trimmed[start..=end];
                if serde_json::from_str::<Vec<serde_json::Value>>(candidate).is_ok() {
                    return candidate.to_string();
                }
            }
        }

        // 兜底：返回原始文本
        trimmed.to_string()
    }
}

impl Default for SwarmCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Swarm 事件
// ============================================================================

/// 蜂群执行过程中的事件
#[derive(Debug, Clone)]
pub enum SwarmEvent {
    /// Coordinator 正在思考
    CoordinatorThinking(String),
    /// Coordinator 回复内容块
    CoordinatorResponseChunk(String),
    /// 任务已分解
    TaskDecomposed {
        subtask_count: usize,
        descriptions: Vec<String>,
    },
    /// Worker 开始执行子任务
    WorkerStarted {
        worker_id: String,
        subtask_id: String,
        description: String,
    },
    /// Worker 调用工具
    WorkerToolCall { worker_id: String, task_id: String, id: String, name: String, params: serde_json::Value },
    /// Worker 工具执行结果
    WorkerToolResult { worker_id: String, task_id: String, id: String, name: String, result: String, success: bool },
    /// Worker 正在思考
    WorkerThinking { worker_id: String, task_id: String, message: String },
    /// Worker 回复内容块
    WorkerResponseChunk { worker_id: String, task_id: String, chunk: String },
    /// Worker 完成
    WorkerCompleted { worker_id: String, task_id: String, result: String },
    /// Worker 执行错误
    WorkerError { worker_id: String, task_id: String, error: String },
    /// 工具缓存命中
    ToolCacheHit { worker_id: String, task_id: String, tool_name: String },
    /// Worker 重试
    WorkerRetry { worker_id: String, task_id: String, attempt: u32, max_attempts: u32 },
    /// Worker 永久失败（超过重试次数）
    WorkerFailedPermanently { worker_id: String, task_id: String, error: String },
    /// 任务进度更新
    TaskProgress { completed: usize, total: usize, failed: usize, running: usize },
    /// Worker 等待依赖
    DependencyWaiting { worker_id: String, task_id: String, waiting_for: Vec<String> },
    /// 所有 Worker 完成
    AllWorkersComplete,
    /// Coordinator 生成最终回复
    FinalResponse(String),
    /// 错误
    CoordinatorError(String),
}

// ============================================================================
// 内部类型
// ============================================================================

/// LLM 返回的分解任务结构
#[derive(Debug, Clone, Deserialize)]
struct DecomposedTask {
    id: String,
    description: String,
    #[serde(default)]
    required_tools: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::agent_core::Capability;
    use crate::agent::tools::create_test_registry;

    #[test]
    fn test_swarm_config_default() {
        let config = SwarmConfig::default();
        assert_eq!(config.max_workers, 5);
        assert!(config.enable_decomposition);
    }

    #[test]
    fn test_swarm_event_variants() {
        let ev1 = SwarmEvent::CoordinatorThinking("思考中".to_string());
        let ev2 = SwarmEvent::TaskDecomposed {
            subtask_count: 2,
            descriptions: vec!["任务A".to_string(), "任务B".to_string()],
        };
        let ev3 = SwarmEvent::WorkerStarted {
            worker_id: "w1".to_string(),
            subtask_id: "s1".to_string(),
            description: "测试".to_string(),
        };
        let ev4 = SwarmEvent::AllWorkersComplete;
        let ev5 = SwarmEvent::FinalResponse("结果".to_string());
        let ev6 = SwarmEvent::CoordinatorError("错误".to_string());

        drop(ev1);
        drop(ev2);
        drop(ev3);
        drop(ev4);
        drop(ev5);
        drop(ev6);
    }

    #[test]
    fn test_extract_json_array_direct() {
        let text = r#"[{"id":"1","description":"test"}]"#;
        let result = SwarmCoordinator::extract_json_array(text);
        assert_eq!(result, text);
    }

    #[test]
    fn test_extract_json_array_from_markdown() {
        let text = r#"```json
[{"id":"1","description":"test"}]
```"#;
        let result = SwarmCoordinator::extract_json_array(text);
        assert!(result.contains("[{"));
        assert!(result.contains("test"));
    }

    #[test]
    fn test_extract_json_array_from_brackets() {
        let text = r#"some text before [{"id":"1"}] after"#;
        let result = SwarmCoordinator::extract_json_array(text);
        assert!(result.starts_with("["));
        assert!(result.ends_with("]"));
    }

    #[test]
    fn test_swarm_coordinator_creation() {
        let coordinator = SwarmCoordinator::new();
        drop(coordinator);
    }

    #[test]
    fn test_detect_cycles_no_cycle() {
        let assignments = vec![
            AgentAssignment {
                task_id: "task-1".to_string(),
                agent_id: "a1".to_string(),
                description: "任务1".to_string(),
                depends_on: vec![],
            },
            AgentAssignment {
                task_id: "task-2".to_string(),
                agent_id: "a2".to_string(),
                description: "任务2".to_string(),
                depends_on: vec!["task-1".to_string()],
            },
            AgentAssignment {
                task_id: "task-3".to_string(),
                agent_id: "a3".to_string(),
                description: "任务3".to_string(),
                depends_on: vec!["task-1".to_string()],
            },
        ];
        assert!(SwarmCoordinator::detect_cycles(&assignments).is_ok());
    }

    #[test]
    fn test_detect_cycles_with_cycle() {
        let assignments = vec![
            AgentAssignment {
                task_id: "task-1".to_string(),
                agent_id: "a1".to_string(),
                description: "任务1".to_string(),
                depends_on: vec!["task-3".to_string()],
            },
            AgentAssignment {
                task_id: "task-2".to_string(),
                agent_id: "a2".to_string(),
                description: "任务2".to_string(),
                depends_on: vec!["task-1".to_string()],
            },
            AgentAssignment {
                task_id: "task-3".to_string(),
                agent_id: "a3".to_string(),
                description: "任务3".to_string(),
                depends_on: vec!["task-2".to_string()],
            },
        ];
        let result = SwarmCoordinator::detect_cycles(&assignments);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("task-1") || err.contains("task-2") || err.contains("task-3"));
    }

    #[test]
    fn test_detect_cycles_missing_dependency() {
        let assignments = vec![
            AgentAssignment {
                task_id: "task-1".to_string(),
                agent_id: "a1".to_string(),
                description: "任务1".to_string(),
                depends_on: vec!["non-existent".to_string()],
            },
        ];
        let result = SwarmCoordinator::detect_cycles(&assignments);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non-existent"));
    }
}
