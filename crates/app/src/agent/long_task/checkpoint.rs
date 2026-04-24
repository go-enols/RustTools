//! 检查点机制 — 保存和恢复长任务的中间状态
//!
//! 检查点保存到用户数据目录: `dirs::data_dir()/rusttools/checkpoints`

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

use crate::agent::tools::tool::ToolResult;
use crate::agent::CheckpointError;

/// 任务检查点 — 记录任务在某一步的状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Checkpoint {
    /// 当前步骤索引
    pub step_index: usize,
    /// 序列化的会话上下文
    pub context: Value,
    /// 已执行工具的结果
    pub tool_outputs: Vec<ToolResult>,
    /// 时间戳 (UNIX epoch秒)
    pub timestamp: u64,
    /// 任务ID
    pub task_id: String,
}

/// 检查点管理器 — 负责检查点的持久化存储
pub struct CheckpointManager {
    /// 检查点存储目录
    storage_dir: PathBuf,
}

impl CheckpointManager {
    /// 创建新的检查点管理器
    ///
    /// 检查点存储位置: `dirs::data_dir()/rusttools/checkpoints`
    pub fn new() -> Result<Self, CheckpointError> {
        let data_dir = dirs::data_dir().ok_or_else(|| {
            CheckpointError::Io("无法获取数据目录".to_string())
        })?;
        let storage_dir = data_dir.join("rusttools").join("checkpoints");

        // 确保目录存在
        fs::create_dir_all(&storage_dir).map_err(|e| {
            CheckpointError::Io(format!("创建检查点目录失败: {}", e))
        })?;

        Ok(Self { storage_dir })
    }

    /// 保存检查点
    pub fn save(&self, checkpoint: &Checkpoint) -> Result<(), CheckpointError> {
        let task_dir = self.storage_dir.join(&checkpoint.task_id);
        fs::create_dir_all(&task_dir).map_err(|e| {
            CheckpointError::Io(format!("创建任务目录失败: {}", e))
        })?;

        let filename = format!("checkpoint_{}.json", checkpoint.timestamp);
        let path = task_dir.join(filename);

        let json = serde_json::to_string_pretty(checkpoint)
            .map_err(|e| CheckpointError::Serialization(e.to_string()))?;

        fs::write(&path, json).map_err(|e| {
            CheckpointError::Io(format!("写入检查点文件失败: {}", e))
        })?;

        log::info!(
            "检查点已保存: task={}, step={}, path={:?}",
            checkpoint.task_id,
            checkpoint.step_index,
            path
        );

        Ok(())
    }

    /// 加载指定任务的所有检查点 (按时间戳排序)
    pub fn load(&self, task_id: &str) -> Result<Vec<Checkpoint>, CheckpointError> {
        let task_dir = self.storage_dir.join(task_id);

        if !task_dir.exists() {
            return Ok(vec![]);
        }

        let mut checkpoints = Vec::new();
        let entries = fs::read_dir(&task_dir).map_err(|e| {
            CheckpointError::Io(format!("读取任务目录失败: {}", e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                CheckpointError::Io(format!("读取目录项失败: {}", e))
            })?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).map_err(|e| {
                    CheckpointError::Io(format!("读取检查点文件失败: {}", e))
                })?;
                let checkpoint: Checkpoint = serde_json::from_str(&content)
                    .map_err(|e| CheckpointError::Serialization(e.to_string()))?;
                checkpoints.push(checkpoint);
            }
        }

        // 按时间戳排序
        checkpoints.sort_by_key(|c| c.timestamp);

        Ok(checkpoints)
    }

    /// 获取指定任务的最新检查点
    pub fn get_latest(&self, task_id: &str) -> Result<Option<Checkpoint>, CheckpointError> {
        let mut checkpoints = self.load(task_id)?;
        Ok(checkpoints.pop())
    }

    /// 删除旧检查点，只保留最近N个
    pub fn delete_older_than(
        &self,
        task_id: &str,
        keep_count: usize,
    ) -> Result<(), CheckpointError> {
        let task_dir = self.storage_dir.join(task_id);

        if !task_dir.exists() {
            return Ok(());
        }

        let mut files = Vec::new();
        let entries = fs::read_dir(&task_dir).map_err(|e| {
            CheckpointError::Io(format!("读取目录失败: {}", e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                CheckpointError::Io(format!("读取目录项失败: {}", e))
            })?;
            let path = entry.path();
            let metadata = entry.metadata().map_err(|e| {
                CheckpointError::Io(format!("获取文件元数据失败: {}", e))
            })?;
            let modified = metadata.modified().map_err(|e| {
                CheckpointError::Io(format!("获取修改时间失败: {}", e))
            })?;
            files.push((path, modified));
        }

        // 按修改时间从新到旧排序
        files.sort_by(|a, b| b.1.cmp(&a.1));

        // 删除旧的
        if files.len() > keep_count {
            for (path, _) in files.iter().skip(keep_count) {
                if let Err(e) = fs::remove_file(path) {
                    log::warn!("删除旧检查点失败: {:?}, 错误: {}", path, e);
                }
            }
        }

        Ok(())
    }

    /// 获取存储目录路径 (用于测试)
    #[cfg(test)]
    pub fn storage_dir(&self) -> &PathBuf {
        &self.storage_dir
    }

    /// 从自定义路径创建 (用于测试)
    #[cfg(test)]
    pub fn with_dir(dir: PathBuf) -> Result<Self, CheckpointError> {
        fs::create_dir_all(&dir).map_err(|e| {
            CheckpointError::Io(format!("创建检查点目录失败: {}", e))
        })?;
        Ok(Self { storage_dir: dir })
    }
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            storage_dir: PathBuf::from("./checkpoints"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::tools::tool::{OutputType, ToolResult};
    use serde_json::json;
    use tempfile::TempDir;

    fn create_test_checkpoint(task_id: &str, step: usize, ts: u64) -> Checkpoint {
        Checkpoint {
            step_index: step,
            context: json!({"key": "value"}),
            tool_outputs: vec![ToolResult {
                success: true,
                content: "result".to_string(),
                output_type: OutputType::Text,
            }],
            timestamp: ts,
            task_id: task_id.to_string(),
        }
    }

    #[test]
    fn test_checkpoint_manager_new() {
        let mgr = CheckpointManager::new();
        assert!(mgr.is_ok());
        let mgr = mgr.unwrap();
        assert!(mgr.storage_dir().to_string_lossy().contains("rusttools"));
        assert!(mgr.storage_dir().to_string_lossy().contains("checkpoints"));
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let mgr = CheckpointManager::with_dir(temp_dir.path().to_path_buf()).unwrap();

        let cp = create_test_checkpoint("task1", 0, 1000);
        mgr.save(&cp).unwrap();

        let loaded = mgr.load("task1").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], cp);
    }

    #[test]
    fn test_load_empty() {
        let temp_dir = TempDir::new().unwrap();
        let mgr = CheckpointManager::with_dir(temp_dir.path().to_path_buf()).unwrap();
        let loaded = mgr.load("nonexistent").unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_get_latest() {
        let temp_dir = TempDir::new().unwrap();
        let mgr = CheckpointManager::with_dir(temp_dir.path().to_path_buf()).unwrap();

        let cp1 = create_test_checkpoint("task1", 0, 1000);
        let cp2 = create_test_checkpoint("task1", 1, 2000);
        let cp3 = create_test_checkpoint("task1", 2, 1500);

        mgr.save(&cp1).unwrap();
        mgr.save(&cp2).unwrap();
        mgr.save(&cp3).unwrap();

        let latest = mgr.get_latest("task1").unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().timestamp, 2000); // 最新的
    }

    #[test]
    fn test_delete_older_than() {
        let temp_dir = TempDir::new().unwrap();
        let mgr = CheckpointManager::with_dir(temp_dir.path().to_path_buf()).unwrap();

        // 保存5个检查点
        for i in 0..5 {
            let cp = create_test_checkpoint("task1", i, 1000 + i as u64);
            mgr.save(&cp).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // 保留最近2个
        mgr.delete_older_than("task1", 2).unwrap();

        let remaining = mgr.load("task1").unwrap();
        assert_eq!(remaining.len(), 2);
    }

    #[test]
    fn test_checkpoint_serialization() {
        let cp = Checkpoint {
            step_index: 3,
            context: json!({"messages": ["hello"]}),
            tool_outputs: vec![],
            timestamp: 1234567890,
            task_id: "task-abc".to_string(),
        };

        let json = serde_json::to_string(&cp).unwrap();
        let deserialized: Checkpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, cp);
    }

    #[test]
    fn test_multiple_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let mgr = CheckpointManager::with_dir(temp_dir.path().to_path_buf()).unwrap();

        let cp_a = create_test_checkpoint("task_a", 0, 1000);
        let cp_b = create_test_checkpoint("task_b", 0, 2000);

        mgr.save(&cp_a).unwrap();
        mgr.save(&cp_b).unwrap();

        let loaded_a = mgr.load("task_a").unwrap();
        let loaded_b = mgr.load("task_b").unwrap();

        assert_eq!(loaded_a.len(), 1);
        assert_eq!(loaded_b.len(), 1);
        assert_eq!(loaded_a[0].task_id, "task_a");
        assert_eq!(loaded_b[0].task_id, "task_b");
    }
}
