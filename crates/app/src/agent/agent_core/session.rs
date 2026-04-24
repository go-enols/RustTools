//! 会话状态管理
//!
//! Session表示一次完整的对话会话，包含消息历史、元数据和状态。
//! SessionManager负责创建、查找和管理所有活跃会话。

use super::super::api_client::{ChatMessage, MessageContent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// 会话状态
// ============================================================================

/// 会话状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// 活跃状态 — 可以进行对话
    Active,
    /// 暂停状态 — 等待用户确认
    Paused,
    /// 已完成
    Completed,
    /// 出错状态
    Error(String),
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Active
    }
}

// ============================================================================
// 会话元数据
// ============================================================================

/// 会话元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionMetadata {
    /// 使用的总token数
    pub total_tokens_used: usize,
    /// 工具调用次数
    pub tool_call_count: u32,
    /// 会话状态
    pub status: SessionStatus,
}

// ============================================================================
// 会话
// ============================================================================

/// 对话会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// 会话唯一ID
    pub id: String,
    /// 使用的Agent ID
    pub agent_id: String,
    /// 消息历史
    pub messages: Vec<ChatMessage>,
    /// 创建时间戳（毫秒）
    pub created_at: u64,
    /// 更新时间戳（毫秒）
    pub updated_at: u64,
    /// 会话元数据
    pub metadata: SessionMetadata,
}

impl Session {
    /// 创建新会话
    pub fn new(id: impl Into<String>, agent_id: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        Self {
            id: id.into(),
            agent_id: agent_id.into(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: SessionMetadata::default(),
        }
    }

    /// 添加用户消息
    pub fn add_user_message(&mut self, content: impl Into<String>) {
        self.messages.push(ChatMessage::User {
            content: MessageContent::Text(content.into()),
        });
        self.update_timestamp();
    }

    /// 添加助手消息
    pub fn add_assistant_message(&mut self, content: impl Into<String>) {
        self.messages.push(ChatMessage::Assistant {
            content: Some(content.into()),
            tool_calls: None,
        });
        self.update_timestamp();
    }

    /// 添加系统消息
    pub fn add_system_message(&mut self, content: impl Into<String>) {
        self.messages.push(ChatMessage::System {
            content: content.into(),
        });
        self.update_timestamp();
    }

    /// 添加工具消息
    pub fn add_tool_message(&mut self, tool_call_id: impl Into<String>, content: impl Into<String>) {
        self.messages.push(ChatMessage::Tool {
            tool_call_id: tool_call_id.into(),
            content: content.into(),
        });
        self.update_timestamp();
    }

    /// 更新token使用量
    pub fn add_tokens_used(&mut self, count: usize) {
        self.metadata.total_tokens_used += count;
    }

    /// 增加工具调用计数
    pub fn increment_tool_call(&mut self) {
        self.metadata.tool_call_count += 1;
    }

    /// 设置会话状态
    pub fn set_status(&mut self, status: SessionStatus) {
        self.metadata.status = status;
        self.update_timestamp();
    }

    /// 获取最近N条消息
    pub fn recent_messages(&self, n: usize) -> Vec<&ChatMessage> {
        self.messages.iter().rev().take(n).rev().collect()
    }

    /// 更新最后活动时间
    fn update_timestamp(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp_millis() as u64;
    }
}

// ============================================================================
// 会话管理器
// ============================================================================

/// 会话管理器 — 管理所有活跃会话
pub struct SessionManager {
    sessions: HashMap<String, Session>,
}

impl SessionManager {
    /// 创建新的会话管理器
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// 创建新会话
    pub fn create(&mut self, id: impl Into<String>, agent_id: impl Into<String>) -> &mut Session {
        let session = Session::new(id, agent_id);
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        self.sessions.get_mut(&id).unwrap()
    }

    /// 获取会话（可变引用）
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    /// 获取会话（不可变引用）
    pub fn get(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    /// 删除会话
    pub fn remove(&mut self, id: &str) -> Option<Session> {
        self.sessions.remove(id)
    }

    /// 列出所有会话ID
    pub fn list_ids(&self) -> Vec<&String> {
        self.sessions.keys().collect()
    }

    /// 获取会话数量
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// 清理已完成或出错的会话
    pub fn cleanup_completed(&mut self) -> usize {
        let to_remove: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| {
                matches!(
                    s.metadata.status,
                    SessionStatus::Completed | SessionStatus::Error(_)
                )
            })
            .map(|(id, _)| id.clone())
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            self.sessions.remove(&id);
        }
        count
    }
}

impl Default for SessionManager {
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

    #[test]
    fn test_session_creation() {
        let session = Session::new("sess-1", "agent-1");
        assert_eq!(session.id, "sess-1");
        assert_eq!(session.agent_id, "agent-1");
        assert!(session.messages.is_empty());
        assert_eq!(session.metadata.status, SessionStatus::Active);
        assert_eq!(session.metadata.total_tokens_used, 0);
        assert_eq!(session.metadata.tool_call_count, 0);
    }

    #[test]
    fn test_session_add_messages() {
        let mut session = Session::new("sess-1", "agent-1");

        session.add_system_message("你是一个助手");
        session.add_user_message("你好");
        session.add_assistant_message("您好！有什么可以帮助的？");

        assert_eq!(session.messages.len(), 3);
        assert!(session.created_at > 0);
        assert!(session.updated_at >= session.created_at);
    }

    #[test]
    fn test_session_add_tool_message() {
        let mut session = Session::new("sess-1", "agent-1");
        session.add_tool_message("call-1", "工具结果");

        assert_eq!(session.messages.len(), 1);
        session.increment_tool_call();
        assert_eq!(session.metadata.tool_call_count, 1);
    }

    #[test]
    fn test_session_add_tokens() {
        let mut session = Session::new("sess-1", "agent-1");
        session.add_tokens_used(100);
        session.add_tokens_used(50);
        assert_eq!(session.metadata.total_tokens_used, 150);
    }

    #[test]
    fn test_session_set_status() {
        let mut session = Session::new("sess-1", "agent-1");
        assert_eq!(session.metadata.status, SessionStatus::Active);

        session.set_status(SessionStatus::Paused);
        assert_eq!(session.metadata.status, SessionStatus::Paused);

        session.set_status(SessionStatus::Completed);
        assert_eq!(session.metadata.status, SessionStatus::Completed);

        session.set_status(SessionStatus::Error("some error".to_string()));
        assert!(matches!(
            session.metadata.status,
            SessionStatus::Error(_)
        ));
    }

    #[test]
    fn test_session_recent_messages() {
        let mut session = Session::new("sess-1", "agent-1");
        for i in 0..10 {
            session.add_user_message(format!("msg-{i}"));
        }

        let recent = session.recent_messages(3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_session_manager_create_and_get() {
        let mut manager = SessionManager::new();
        let session = manager.create("sess-1", "agent-1");

        assert_eq!(session.id, "sess-1");
        assert_eq!(manager.len(), 1);

        let retrieved = manager.get("sess-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().agent_id, "agent-1");
    }

    #[test]
    fn test_session_manager_remove() {
        let mut manager = SessionManager::new();
        manager.create("sess-1", "agent-1");

        let removed = manager.remove("sess-1");
        assert!(removed.is_some());
        assert_eq!(manager.len(), 0);

        assert!(manager.get("sess-1").is_none());
    }

    #[test]
    fn test_session_manager_list_ids() {
        let mut manager = SessionManager::new();
        manager.create("sess-1", "agent-1");
        manager.create("sess-2", "agent-2");

        let ids = manager.list_ids();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn test_session_manager_cleanup() {
        let mut manager = SessionManager::new();
        manager.create("sess-active", "agent-1");
        manager.create("sess-completed", "agent-1");
        manager.create("sess-error", "agent-1");

        // 修改状态
        if let Some(s) = manager.get_mut("sess-completed") {
            s.set_status(SessionStatus::Completed);
        }
        if let Some(s) = manager.get_mut("sess-error") {
            s.set_status(SessionStatus::Error("err".to_string()));
        }

        let cleaned = manager.cleanup_completed();
        assert_eq!(cleaned, 2);
        assert_eq!(manager.len(), 1);
        assert!(manager.get("sess-active").is_some());
    }

    #[test]
    fn test_session_serialization() {
        let mut session = Session::new("test-id", "test-agent");
        session.add_user_message("hello");
        session.add_assistant_message("world");
        session.metadata.total_tokens_used = 42;
        session.metadata.tool_call_count = 1;

        // 序列化
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("test-agent"));

        // 反序列化
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test-id");
        assert_eq!(deserialized.agent_id, "test-agent");
        assert_eq!(deserialized.messages.len(), 2);
        assert_eq!(deserialized.metadata.total_tokens_used, 42);
        assert_eq!(deserialized.metadata.tool_call_count, 1);
    }
}
