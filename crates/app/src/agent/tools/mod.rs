//! AI工具封装层
//!
//! 提供统一的Tool trait和所有内置工具实现。
//! 包括文件系统工具、终端工具、代码编辑工具和搜索工具。

pub mod tool;
pub mod filesystem;
pub mod terminal;
pub mod code_edit;
pub mod search;

// 统一导出常用类型
pub use tool::{OutputType, Tool, ToolDefinition, ToolError, ToolRegistry, ToolResult};

pub use filesystem::{FileListTool, FileReadTool, FileWriteTool};
pub use terminal::TerminalTool;
pub use code_edit::{CodeEditTool, CodeReplaceTool};
pub use search::SearchTool;

use serde_json::Value;

/// 创建默认工具集
///
/// 注册所有内置工具到ToolRegistry，方便直接使用。
/// allowed_directories用于文件系统相关工具的目录安全检查。
pub fn create_default_registry(allowed_directories: Vec<String>) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    registry.register(Box::new(FileReadTool::new(allowed_directories.clone())));
    registry.register(Box::new(FileWriteTool::new(allowed_directories.clone())));
    registry.register(Box::new(FileListTool::new(allowed_directories.clone())));
    registry.register(Box::new(TerminalTool::new()));
    registry.register(Box::new(CodeEditTool::new(allowed_directories.clone())));
    registry.register(Box::new(CodeReplaceTool::new(allowed_directories.clone())));
    registry.register(Box::new(SearchTool::new(allowed_directories)));

    registry
}

/// 创建允许所有目录的默认工具集（仅用于测试）
#[cfg(test)]
pub fn create_test_registry() -> ToolRegistry {
    create_default_registry(vec![])
}

// ============================================================================
// 模块测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_registry() {
        let registry = create_default_registry(vec![]);

        // 验证所有工具都已注册
        assert!(registry.contains("fs_read"));
        assert!(registry.contains("fs_write"));
        assert!(registry.contains("fs_list"));
        assert!(registry.contains("terminal"));
        assert!(registry.contains("code_edit"));
        assert!(registry.contains("code_replace"));
    }

    #[test]
    fn test_default_registry_llm_definitions() {
        let registry = create_default_registry(vec![]);
        let defs = registry.definitions_for_llm();

        assert!(!defs.is_empty());

        // 每个定义应该有name和description
        for def in &defs {
            assert!(!def.function.name.is_empty());
            assert!(!def.function.description.is_empty());
            assert!(!def.function.parameters.is_null());
        }
    }
}
