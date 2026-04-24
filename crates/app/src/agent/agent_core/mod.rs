//! Agent编排引擎
//!
//! 核心模块，负责Agent管理、任务规划、执行编排和会话管理。
//! 参考Claude Code设计理念，支持多Agent协同和自适应规划。

pub mod agent;
pub mod orchestrator;
pub mod planner;
pub mod executor;
pub mod session;

// 统一导出公共API
pub use agent::{Agent, AgentDefinition, AgentError, AgentInfo, Capability};
pub use orchestrator::{Orchestrator, StepRecord, TaskResult, ToolCallRecord};
pub use planner::{Plan, PlanStep, PlannedToolCall, Planner, PlanningStrategy};
pub use executor::Executor;
pub use session::{Session, SessionManager, SessionMetadata, SessionStatus};
