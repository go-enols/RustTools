//! Tauri 命令模块注册中心
//!
//! 将 YOLO / Project / Environment 相关命令集中到 `commands` 子模块，
//! 与独立的 `agent_commands` 模块形成清晰边界：
//!
//! - `commands/` — 工具类命令（YOLO、训练、标注、项目、环境等）
//! - `agent_commands/` — AI Agent 独立模块命令

pub mod project;
pub mod yolo;
