//! 任务队列 — 管理Agent任务的入队、执行和完成
//!
//! 支持优先级队列、心跳检测和超时任务识别。

use std::collections::{HashMap, VecDeque};

use crate::agent::agent_core::orchestrator::TaskResult;

/// 排队中的任务
#[derive(Debug, Clone, PartialEq)]
pub struct QueuedTask {
    pub id: String,
    pub priority: u32,
    pub description: String,
    pub agent_id: String,
    pub created_at: u64,
}

/// 运行中的任务
#[derive(Debug, Clone, PartialEq)]
pub struct RunningTask {
    pub task: QueuedTask,
    pub started_at: u64,
    pub heartbeat_at: u64,
}

/// 已完成的任务
#[derive(Debug, Clone, PartialEq)]
pub struct CompletedTask {
    pub id: String,
    pub result: TaskResult,
    pub completed_at: u64,
}

/// 失败的任务
#[derive(Debug, Clone, PartialEq)]
pub struct FailedTask {
    pub id: String,
    pub error: String,
    pub failed_at: u64,
}

/// 任务队列 — 管理任务全生命周期
///
/// 状态流转:
/// pending -> running -> completed / failed
pub struct TaskQueue {
    /// 等待执行的任务队列 (按优先级排序)
    pending: VecDeque<QueuedTask>,
    /// 正在执行的任务
    running: HashMap<String, RunningTask>,
    /// 已完成的任务
    completed: Vec<CompletedTask>,
    /// 失败的任务
    failed: Vec<FailedTask>,
}

impl TaskQueue {
    /// 创建新的空任务队列
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            running: HashMap::new(),
            completed: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// 添加任务到队列
    ///
    /// 按优先级降序排列 (priority值越大越优先)
    pub fn enqueue(&mut self, task: QueuedTask) {
        let pos = self
            .pending
            .iter()
            .position(|t| t.priority < task.priority);
        match pos {
            Some(idx) => self.pending.insert(idx, task),
            None => self.pending.push_back(task),
        }
    }

    /// 启动下一个任务
    ///
    /// 从队列头部取出最高优先级任务，移入running状态。
    /// 返回任务ID。
    pub fn start_next(&mut self) -> Option<String> {
        let task = self.pending.pop_front()?;
        let now = current_timestamp();
        let id = task.id.clone();
        let running = RunningTask {
            task,
            started_at: now,
            heartbeat_at: now,
        };
        self.running.insert(id.clone(), running);
        Some(id)
    }

    /// 标记任务完成
    pub fn complete(&mut self, id: &str, result: TaskResult) {
        if let Some(running) = self.running.remove(id) {
            let completed = CompletedTask {
                id: running.task.id,
                result,
                completed_at: current_timestamp(),
            };
            self.completed.push(completed);
        }
    }

    /// 标记任务失败
    pub fn fail(&mut self, id: &str, error: String) {
        if let Some(running) = self.running.remove(id) {
            let failed = FailedTask {
                id: running.task.id,
                error,
                failed_at: current_timestamp(),
            };
            self.failed.push(failed);
        }
    }

    /// 更新任务心跳时间
    ///
    /// 返回 `true` 如果任务存在并更新成功
    pub fn heartbeat(&mut self, id: &str) -> bool {
        if let Some(task) = self.running.get_mut(id) {
            task.heartbeat_at = current_timestamp();
            true
        } else {
            false
        }
    }

    /// 获取超时(僵死)任务列表
    ///
    /// 返回超过 `timeout_secs` 秒没有心跳的任务ID列表
    pub fn get_stalled_tasks(&self, timeout_secs: u64) -> Vec<String> {
        let now = current_timestamp();
        self.running
            .iter()
            .filter(|(_, task)| {
                let elapsed = now.saturating_sub(task.heartbeat_at);
                elapsed > timeout_secs
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// 获取指定运行任务的可变引用
    pub fn get_running(&mut self, id: &str) -> Option<&mut RunningTask> {
        self.running.get_mut(id)
    }

    /// 查看下一个待处理任务 (不取出)
    pub fn peek_next(&self) -> Option<&QueuedTask> {
        self.pending.front()
    }

    /// 取消待处理的任务
    pub fn cancel_pending(&mut self, id: &str) -> bool {
        if let Some(pos) = self.pending.iter().position(|t| t.id == id) {
            self.pending.remove(pos);
            true
        } else {
            false
        }
    }

    /// 获取队列统计
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            pending: self.pending.len(),
            running: self.running.len(),
            completed: self.completed.len(),
            failed: self.failed.len(),
        }
    }

    /// 获取所有运行中任务ID
    pub fn running_ids(&self) -> Vec<String> {
        self.running.keys().cloned().collect()
    }

    /// 获取已完成任务 (最近N个)
    pub fn recent_completed(&self, n: usize) -> Vec<&CompletedTask> {
        self.completed.iter().rev().take(n).collect()
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// 队列统计信息
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueueStats {
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
}

/// 获取当前UNIX时间戳
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::agent_core::orchestrator::TaskResult;

    fn make_task(id: &str, priority: u32) -> QueuedTask {
        QueuedTask {
            id: id.to_string(),
            priority,
            description: format!("Task {}", id),
            agent_id: "agent1".to_string(),
            created_at: current_timestamp(),
        }
    }

    fn make_result() -> TaskResult {
        TaskResult {
            final_response: "done".to_string(),
            steps_executed: vec![],
            tool_calls: vec![],
            duration_ms: 100,
        }
    }

    #[test]
    fn test_enqueue_and_priority_ordering() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task("low", 1));
        queue.enqueue(make_task("high", 10));
        queue.enqueue(make_task("mid", 5));

        let next = queue.peek_next().unwrap();
        assert_eq!(next.id, "high");
    }

    #[test]
    fn test_start_next() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task("t1", 1));

        let id = queue.start_next().unwrap();
        assert_eq!(id, "t1");
        assert!(queue.peek_next().is_none());
        assert_eq!(queue.running.len(), 1);
    }

    #[test]
    fn test_complete() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task("t1", 1));
        queue.start_next();

        queue.complete("t1", make_result());

        assert!(queue.running.is_empty());
        assert_eq!(queue.completed.len(), 1);
    }

    #[test]
    fn test_fail() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task("t1", 1));
        queue.start_next();

        queue.fail("t1", "something went wrong".to_string());

        assert!(queue.running.is_empty());
        assert_eq!(queue.failed.len(), 1);
        assert_eq!(queue.failed[0].error, "something went wrong");
    }

    #[test]
    fn test_heartbeat() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task("t1", 1));
        queue.start_next();

        let hb1 = queue.running.get("t1").unwrap().heartbeat_at;

        std::thread::sleep(std::time::Duration::from_millis(50));

        assert!(queue.heartbeat("t1"));
        let hb2 = queue.running.get("t1").unwrap().heartbeat_at;
        assert!(hb2 > hb1);

        assert!(!queue.heartbeat("nonexistent"));
    }

    #[test]
    fn test_get_stalled_tasks() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task("t1", 1));
        queue.start_next();

        let stalled = queue.get_stalled_tasks(0);
        assert!(stalled.contains(&"t1".to_string()));

        queue.heartbeat("t1");
        let stalled = queue.get_stalled_tasks(5);
        assert!(stalled.is_empty());
    }

    #[test]
    fn test_cancel_pending() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task("t1", 1));
        queue.enqueue(make_task("t2", 2));

        assert!(queue.cancel_pending("t1"));
        assert_eq!(queue.pending.len(), 1);
        assert_eq!(queue.peek_next().unwrap().id, "t2");

        assert!(!queue.cancel_pending("nonexistent"));
    }

    #[test]
    fn test_stats() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task("p1", 1));
        queue.enqueue(make_task("p2", 2));
        queue.start_next();

        let stats = queue.stats();
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.running, 1);
        assert_eq!(stats.completed, 0);
        assert_eq!(stats.failed, 0);
    }

    #[test]
    fn test_multiple_start_and_complete() {
        let mut queue = TaskQueue::new();
        for i in 0..5 {
            queue.enqueue(make_task(&format!("t{}", i), i));
        }

        let id1 = queue.start_next().unwrap();
        let id2 = queue.start_next().unwrap();
        let id3 = queue.start_next().unwrap();

        assert_eq!(id1, "t4");
        assert_eq!(id2, "t3");
        assert_eq!(id3, "t2");
        assert_eq!(queue.pending.len(), 2);

        queue.complete("t4", make_result());
        queue.fail("t3", "error".to_string());

        assert_eq!(queue.running.len(), 1);
        assert_eq!(queue.completed.len(), 1);
        assert_eq!(queue.failed.len(), 1);
    }

    #[test]
    fn test_recent_completed() {
        let mut queue = TaskQueue::new();
        for i in 0..5 {
            queue.enqueue(make_task(&format!("t{}", i), 1));
            let id = queue.start_next().unwrap();
            queue.complete(
                &id,
                TaskResult {
                    final_response: format!("result {}", i),
                    steps_executed: vec![],
                    tool_calls: vec![],
                    duration_ms: i as u64 * 100,
                },
            );
        }

        let recent = queue.recent_completed(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].id, "t4");
        assert_eq!(recent[1].id, "t3");
        assert_eq!(recent[2].id, "t2");
    }

    #[test]
    fn test_running_ids() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task("a", 1));
        queue.enqueue(make_task("b", 2));
        queue.start_next();
        queue.start_next();

        let ids = queue.running_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"a".to_string()));
        assert!(ids.contains(&"b".to_string()));
    }
}
