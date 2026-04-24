//! 长任务稳定性模块 — 检查点、故障恢复和任务队列
//!
//! 为AI Agent的长时运行任务提供：
//! - 定期保存检查点，崩溃后恢复
//! - 自动重试机制
//! - 任务队列管理与心跳检测

pub mod checkpoint;
pub mod recovery;
pub mod task_queue;

// 公共API重导出
pub use checkpoint::{Checkpoint, CheckpointManager};
pub use recovery::RecoveryManager;
pub use task_queue::{QueuedTask, RunningTask, CompletedTask, TaskQueue, QueueStats};
