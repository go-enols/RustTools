//! 会话持久化存储
//!
//! 参考 Claude Code 的持久化理念，将会话历史保存到本地文件系统，
//! 确保应用重启后会话不丢失。
//!
//! 存储策略：
//! - 每个会话保存为独立的 JSON 文件（按会话 ID 命名）
//! - 使用原子写入（临时文件 + rename）避免文件损坏
//! - 存储路径：`data_dir/rusttools/sessions/{session_id}.json`

use super::session::Session;
use std::path::{Path, PathBuf};

/// 会话存储错误
#[derive(Debug, thiserror::Error)]
pub enum SessionStoreError {
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON序列化错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("无效存储路径")]
    InvalidPath,
}

/// 会话持久化存储
///
/// 负责将 Session 对象序列化为 JSON 文件，并在需要时加载。
pub struct SessionStore {
    store_dir: PathBuf,
}

impl SessionStore {
    /// 创建默认路径的会话存储
    pub fn new() -> Result<Self, SessionStoreError> {
        Self::new_with_workspace(None)
    }

    /// 创建带工作区隔离的会话存储
    ///
    /// - `workspace_path` 为 `None` 时，使用全局默认路径
    /// - `workspace_path` 为 `Some(path)` 时，使用工作区哈希作为子目录名
    pub fn new_with_workspace(workspace_path: Option<&str>) -> Result<Self, SessionStoreError> {
        let data_dir = dirs::data_dir().ok_or(SessionStoreError::InvalidPath)?;
        let mut store_dir = data_dir.join("rusttools").join("sessions");

        if let Some(path) = workspace_path {
            // 使用路径的简单哈希作为子目录名，避免特殊字符问题
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            path.hash(&mut hasher);
            let hash = format!("{:x}", hasher.finish());
            store_dir = store_dir.join(hash);
        }

        std::fs::create_dir_all(&store_dir)?;
        log::info!("会话存储目录: {}", store_dir.display());
        Ok(Self { store_dir })
    }

    /// 从指定路径创建会话存储（主要用于测试）
    pub fn with_path(path: impl AsRef<Path>) -> Result<Self, SessionStoreError> {
        let store_dir = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&store_dir)?;
        Ok(Self { store_dir })
    }

    /// 保存会话（原子写入）
    pub fn save(&self, session: &Session) -> Result<(), SessionStoreError> {
        let path = self.session_path(&session.id);
        let json = serde_json::to_string_pretty(session)?;

        // 原子写入
        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, &json)?;
        std::fs::rename(&temp_path, &path)?;

        log::debug!("会话已保存: {} ({} 条消息)", session.id, session.messages.len());
        Ok(())
    }

    /// 加载指定会话
    pub fn load(&self, session_id: &str) -> Result<Option<Session>, SessionStoreError> {
        let path = self.session_path(session_id);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        let session: Session = serde_json::from_str(&content)?;
        Ok(Some(session))
    }

    /// 加载所有会话
    pub fn load_all(&self) -> Result<Vec<Session>, SessionStoreError> {
        let mut sessions = Vec::new();

        if !self.store_dir.exists() {
            return Ok(sessions);
        }

        for entry in std::fs::read_dir(&self.store_dir)? {
            let entry = entry?;
            let path = entry.path();

            // 只处理 .json 文件
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    if content.trim().is_empty() {
                        log::warn!("会话文件为空: {}", path.display());
                        continue;
                    }
                    match serde_json::from_str::<Session>(&content) {
                        Ok(session) => {
                            sessions.push(session);
                        }
                        Err(e) => {
                            log::error!("解析会话文件失败 ({}): {}", path.display(), e);
                            // 备份损坏的文件
                            let backup_path = path.with_extension("json.bak");
                            let _ = std::fs::rename(&path, &backup_path);
                        }
                    }
                }
                Err(e) => {
                    log::error!("读取会话文件失败 ({}): {}", path.display(), e);
                }
            }
        }

        log::info!("已加载 {} 个持久化会话", sessions.len());
        Ok(sessions)
    }

    /// 删除会话
    pub fn delete(&self, session_id: &str) -> Result<(), SessionStoreError> {
        let path = self.session_path(session_id);
        if path.exists() {
            std::fs::remove_file(&path)?;
            log::debug!("会话已删除: {}", session_id);
        }
        Ok(())
    }

    /// 获取会话文件路径
    fn session_path(&self, session_id: &str) -> PathBuf {
        self.store_dir.join(format!("{}.json", session_id))
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new().expect("创建会话存储失败")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::session::Session;

    #[test]
    fn test_session_store_save_and_load() {
        let temp_dir = tempfile::tempdir().expect("创建临时目录失败");
        let store = SessionStore::with_path(temp_dir.path()).expect("创建存储失败");

        let mut session = Session::new("test-sess-1", "agent-1");
        session.add_user_message("你好");
        session.add_assistant_message("您好！有什么可以帮助的？");

        store.save(&session).expect("保存会话失败");

        let loaded = store.load("test-sess-1").expect("加载会话失败");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.id, "test-sess-1");
        assert_eq!(loaded.agent_id, "agent-1");
        assert_eq!(loaded.messages.len(), 2);
    }

    #[test]
    fn test_session_store_load_all() {
        let temp_dir = tempfile::tempdir().expect("创建临时目录失败");
        let store = SessionStore::with_path(temp_dir.path()).expect("创建存储失败");

        let mut s1 = Session::new("sess-1", "agent-1");
        s1.add_user_message("消息1");
        store.save(&s1).unwrap();

        let mut s2 = Session::new("sess-2", "agent-2");
        s2.add_user_message("消息2");
        store.save(&s2).unwrap();

        let sessions = store.load_all().expect("加载所有会话失败");
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_session_store_delete() {
        let temp_dir = tempfile::tempdir().expect("创建临时目录失败");
        let store = SessionStore::with_path(temp_dir.path()).expect("创建存储失败");

        let session = Session::new("delete-me", "agent-1");
        store.save(&session).unwrap();

        assert!(store.load("delete-me").unwrap().is_some());

        store.delete("delete-me").unwrap();

        assert!(store.load("delete-me").unwrap().is_none());
    }
}
