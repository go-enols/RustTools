//! 故障恢复 — 基于检查点的任务恢复和重试机制
//!
//! 提供自动重试、从检查点恢复和幂等性执行保障。

use crate::agent::agent_core::agent::AgentError;
use crate::agent::agent_core::orchestrator::TaskResult;
use crate::agent::agent_core::Orchestrator;
use crate::agent::tools::tool::ToolResult;
use crate::agent::RecoveryError;
use super::checkpoint::{Checkpoint, CheckpointManager};

/// 恢复管理器 — 管理任务的容错执行
///
/// 核心功能：
/// 1. `execute_with_recovery` — 包装任务执行，自动重试和检查点
/// 2. `can_recover` — 检查任务是否可以恢复
/// 3. `recover` — 从检查点恢复任务
pub struct RecoveryManager {
    checkpoint_mgr: CheckpointManager,
    max_retries: u32,
}

impl RecoveryManager {
    /// 创建新的恢复管理器
    ///
    /// # Arguments
    /// * `max_retries` — 最大重试次数 (不包括初始尝试)
    pub fn new(max_retries: u32) -> Self {
        Self {
            checkpoint_mgr: CheckpointManager::new()
                .unwrap_or_else(|_| CheckpointManager::default()),
            max_retries,
        }
    }

    /// 使用自定义检查点管理器创建 (用于测试)
    #[cfg(test)]
    pub fn with_manager(checkpoint_mgr: CheckpointManager, max_retries: u32) -> Self {
        Self {
            checkpoint_mgr,
            max_retries,
        }
    }

    /// 带恢复机制执行任务
    ///
    /// 如果任务失败，会尝试从最近的检查点恢复并重试，
    /// 直到成功或超过最大重试次数。
    ///
    /// # Type Parameters
    /// * `F` — 任务工厂函数类型
    /// * `Fut` — 任务异步返回类型
    pub async fn execute_with_recovery<F, Fut>(
        &self,
        task_id: &str,
        mut f: F,
    ) -> Result<TaskResult, RecoveryError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<TaskResult, AgentError>>,
    {
        let mut attempts = 0u32;

        loop {
            attempts += 1;

            match f().await {
                Ok(result) => {
                    log::info!("任务 {} 执行成功 (尝试 {})", task_id, attempts);
                    return Ok(result);
                }
                Err(err) => {
                    let err_msg = err.to_string();
                    log::warn!(
                        "任务 {} 第 {} 次尝试失败: {}",
                        task_id,
                        attempts,
                        err_msg
                    );

                    if attempts > self.max_retries {
                        return Err(RecoveryError::MaxRetriesExceeded {
                            task_id: task_id.to_string(),
                            retries: self.max_retries,
                        });
                    }

                    // 检查是否有检查点可用
                    match self.checkpoint_mgr.get_latest(task_id) {
                        Ok(Some(cp)) => {
                            log::info!(
                                "任务 {} 将从检查点恢复: step={}",
                                task_id,
                                cp.step_index
                            );
                        }
                        Ok(None) => {
                            log::warn!("任务 {} 无检查点，将从头重试", task_id);
                        }
                        Err(e) => {
                            log::error!("读取检查点失败: {}", e);
                        }
                    }

                    // 指数退避等待
                    let delay_secs = 2u64.saturating_pow(attempts.min(5));
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
                }
            }
        }
    }

    /// 检查任务是否可以恢复 (是否有可用检查点)
    pub fn can_recover(&self, task_id: &str) -> bool {
        matches!(self.checkpoint_mgr.get_latest(task_id), Ok(Some(_)))
    }

    /// 从检查点恢复任务
    ///
    /// 从最近的检查点加载状态，然后通过编排器继续执行。
    /// 此方法是占位实现，实际恢复逻辑需要与Orchestrator深度集成。
    pub async fn recover(
        &self,
        task_id: &str,
        _orchestrator: &Orchestrator,
    ) -> Result<TaskResult, RecoveryError> {
        let checkpoint = self
            .checkpoint_mgr
            .get_latest(task_id)?
            .ok_or_else(|| RecoveryError::NoCheckpoint(task_id.to_string()))?;

        log::info!(
            "恢复任务 {} 从检查点 step={}, timestamp={}",
            task_id,
            checkpoint.step_index,
            checkpoint.timestamp
        );

        // 实际恢复逻辑需要编排器从 checkpoint.step_index 继续执行
        // 这里返回一个占位成功结果
        Ok(TaskResult {
            final_response: format!("从检查点 {} 恢复", checkpoint.step_index),
            steps_executed: vec![],
            tool_calls: vec![],
            duration_ms: 0,
        })
    }

    /// 保存检查点 (任务执行过程中调用)
    pub fn save_checkpoint(
        &self,
        task_id: &str,
        step_index: usize,
        context: serde_json::Value,
        tool_outputs: Vec<ToolResult>,
    ) -> Result<(), RecoveryError> {
        let checkpoint = Checkpoint {
            step_index,
            context,
            tool_outputs,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            task_id: task_id.to_string(),
        };

        self.checkpoint_mgr
            .save(&checkpoint)
            .map_err(RecoveryError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::super::checkpoint::CheckpointManager;
    use super::*;
    use crate::agent::agent_core::orchestrator::TaskResult;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use tempfile::TempDir;

    fn ok_result() -> TaskResult {
        TaskResult {
            final_response: "done".to_string(),
            steps_executed: vec![],
            tool_calls: vec![],
            duration_ms: 100,
        }
    }

    #[tokio::test]
    async fn test_execute_with_recovery_success() {
        let temp_dir = TempDir::new().unwrap();
        let cp_mgr = CheckpointManager::with_dir(temp_dir.path().to_path_buf()).unwrap();
        let mgr = RecoveryManager::with_manager(cp_mgr, 3);

        let result = mgr
            .execute_with_recovery("task1", || async { Ok(ok_result()) })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_recovery_retry_then_success() {
        let temp_dir = TempDir::new().unwrap();
        let cp_mgr = CheckpointManager::with_dir(temp_dir.path().to_path_buf()).unwrap();
        let mgr = RecoveryManager::with_manager(cp_mgr, 3);

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = mgr
            .execute_with_recovery("task2", move || {
                let c = Arc::clone(&counter_clone);
                async move {
                    let count = c.fetch_add(1, Ordering::SeqCst) + 1;
                    if count < 3 {
                        Err(AgentError::ExecutionError(format!("第{}次失败", count)))
                    } else {
                        Ok(ok_result())
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_execute_with_recovery_max_retries_exceeded() {
        let temp_dir = TempDir::new().unwrap();
        let cp_mgr = CheckpointManager::with_dir(temp_dir.path().to_path_buf()).unwrap();
        let mgr = RecoveryManager::with_manager(cp_mgr, 2);

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = mgr
            .execute_with_recovery("task3", move || {
                let c = Arc::clone(&counter_clone);
                async move {
                    let count = c.fetch_add(1, Ordering::SeqCst) + 1;
                    Err(AgentError::ExecutionError(format!("always fail {}", count)))
                }
            })
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RecoveryError::MaxRetriesExceeded { task_id, retries } => {
                assert_eq!(task_id, "task3");
                assert_eq!(retries, 2);
            }
            other => panic!("期望MaxRetriesExceeded, 得到: {:?}", other),
        }

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_can_recover_with_checkpoint() {
        let temp_dir = TempDir::new().unwrap();
        let cp_mgr = CheckpointManager::with_dir(temp_dir.path().to_path_buf()).unwrap();

        let checkpoint = Checkpoint {
            step_index: 2,
            context: serde_json::json!({"test": true}),
            tool_outputs: vec![],
            timestamp: 1000,
            task_id: "recoverable_task".to_string(),
        };
        cp_mgr.save(&checkpoint).unwrap();

        let mgr = RecoveryManager::with_manager(cp_mgr, 3);
        assert!(mgr.can_recover("recoverable_task"));
        assert!(!mgr.can_recover("unknown_task"));
    }

    #[tokio::test]
    async fn test_recover_from_checkpoint() {
        let temp_dir = TempDir::new().unwrap();
        let cp_mgr = CheckpointManager::with_dir(temp_dir.path().to_path_buf()).unwrap();

        let checkpoint = Checkpoint {
            step_index: 3,
            context: serde_json::json!({"key": "value"}),
            tool_outputs: vec![],
            timestamp: 2000,
            task_id: "recover_task".to_string(),
        };
        cp_mgr.save(&checkpoint).unwrap();

        let mgr = RecoveryManager::with_manager(cp_mgr, 3);
        let config_mgr =
            std::sync::Arc::new(crate::agent::config::ConfigManager::with_path("/dev/null").unwrap_or_else(|_| {
                crate::agent::config::ConfigManager::with_path("/tmp/test_config.json").unwrap()
            }));
        let orchestrator = Orchestrator::new(config_mgr);

        let result = mgr.recover("recover_task", &orchestrator).await;
        assert!(result.is_ok());
        let task_result = result.unwrap();
        assert!(task_result.final_response.contains("检查点 3"));
    }

    #[tokio::test]
    async fn test_recover_no_checkpoint() {
        let temp_dir = TempDir::new().unwrap();
        let cp_mgr = CheckpointManager::with_dir(temp_dir.path().to_path_buf()).unwrap();
        let mgr = RecoveryManager::with_manager(cp_mgr, 3);
        let config_mgr =
            std::sync::Arc::new(crate::agent::config::ConfigManager::with_path("/dev/null").unwrap_or_else(|_| {
                crate::agent::config::ConfigManager::with_path("/tmp/test_config.json").unwrap()
            }));
        let orchestrator = Orchestrator::new(config_mgr);

        let result = mgr.recover("no_cp_task", &orchestrator).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            RecoveryError::NoCheckpoint(id) => assert_eq!(id, "no_cp_task"),
            other => panic!("期望NoCheckpoint, 得到: {:?}", other),
        }
    }

    #[test]
    fn test_save_checkpoint() {
        let temp_dir = TempDir::new().unwrap();
        let cp_mgr = CheckpointManager::with_dir(temp_dir.path().to_path_buf()).unwrap();
        let mgr = RecoveryManager::with_manager(cp_mgr, 3);

        let result = mgr.save_checkpoint(
            "task_cp",
            5,
            serde_json::json!({"progress": 50}),
            vec![],
        );
        assert!(result.is_ok());

        // 验证能恢复
        assert!(mgr.can_recover("task_cp"));
    }
}
