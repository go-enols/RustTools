//! 工作区状态管理
//!
//! 提供全局工作区路径状态，确保 Agent 使用用户打开的文件夹
//! 作为工作目录，而非软件安装/启动目录。

use parking_lot::RwLock;

/// 工作区状态
#[derive(Debug, Clone, Default)]
pub struct WorkspaceState {
    /// 用户打开的项目/文件夹路径
    pub path: Option<String>,
}

impl WorkspaceState {
    /// 设置工作区路径
    pub fn set_path(&mut self, path: impl Into<String>) {
        let path = path.into();
        log::info!("设置 Agent 工作区: {}", path);
        self.path = Some(path);
    }

    /// 清除工作区路径
    pub fn clear(&mut self) {
        log::info!("清除 Agent 工作区");
        self.path = None;
    }

    /// 获取当前工作目录
    ///
    /// 优先返回用户设置的工作区路径，如果没有则回退到进程当前目录。
    pub fn current_dir(&self) -> String {
        self.path
            .as_ref()
            .cloned()
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            })
            .unwrap_or_default()
    }

    /// 获取当前工作区路径（可能为 None）
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }
}

/// 全局工作区状态
///
/// 所有 Agent 组件共享此状态：
/// - 文件系统工具解析相对路径时以此为准
/// - System Prompt 环境信息以此显示
/// - system_info 工具返回此路径作为项目目录
pub static WORKSPACE: once_cell::sync::Lazy<RwLock<WorkspaceState>> =
    once_cell::sync::Lazy::new(|| RwLock::new(WorkspaceState::default()));

/// 设置全局工作区路径
pub fn set_workspace(path: impl Into<String>) {
    WORKSPACE.write().set_path(path);
}

/// 清除全局工作区路径
pub fn clear_workspace() {
    WORKSPACE.write().clear();
}

/// 获取当前工作目录（便捷函数）
pub fn current_dir() -> String {
    WORKSPACE.read().current_dir()
}

/// 获取当前工作区路径（便捷函数）
pub fn workspace_path() -> Option<String> {
    WORKSPACE.read().path.clone()
}
