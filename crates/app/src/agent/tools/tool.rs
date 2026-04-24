//! 工具trait定义与注册中心
//!
//! 提供Tool trait统一接口和ToolRegistry注册中心，
//! 所有AI可用工具必须实现Tool trait。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// 错误类型
// ============================================================================

/// 工具执行错误
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// 参数无效或缺失
    #[error("参数错误: {0}")]
    InvalidParameters(String),
    /// 文件系统操作失败
    #[error("文件系统错误: {0}")]
    Filesystem(String),
    /// 路径安全检查失败
    #[error("路径安全检查失败: {path} 不在允许的目录中")]
    PathNotAllowed { path: String },
    /// 命令执行失败
    #[error("命令执行错误: {0}")]
    CommandExecution(String),
    /// 命令执行超时
    #[error("命令执行超时 ({timeout}s): {command}")]
    Timeout { command: String, timeout: u64 },
    /// 未找到匹配内容
    #[error("未找到: {0}")]
    NotFound(String),
    /// IO错误
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),
    /// JSON解析错误
    #[error("JSON解析错误: {0}")]
    JsonParse(String),
    /// 其他错误
    #[error("{0}")]
    Other(String),
}

// ============================================================================
// 输出类型
// ============================================================================

/// 工具输出内容类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputType {
    /// 纯文本输出
    Text,
    /// JSON结构化数据
    Json,
    /// Markdown格式文本
    Markdown,
    /// Base64编码的图片
    ImageBase64,
    /// 错误信息
    Error,
}

impl Default for OutputType {
    fn default() -> Self {
        Self::Text
    }
}

// ============================================================================
// 工具结果
// ============================================================================

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResult {
    /// 是否执行成功
    pub success: bool,
    /// 输出内容
    pub content: String,
    /// 输出内容类型
    #[serde(default)]
    pub output_type: OutputType,
}

impl ToolResult {
    /// 创建成功的文本结果
    pub fn ok(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            output_type: OutputType::Text,
        }
    }

    /// 创建成功的JSON结果
    pub fn ok_json(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            output_type: OutputType::Json,
        }
    }

    /// 创建成功的Markdown结果
    pub fn ok_markdown(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            output_type: OutputType::Markdown,
        }
    }

    /// 创建失败的错误结果
    pub fn err(content: impl Into<String>) -> Self {
        Self {
            success: false,
            content: content.into(),
            output_type: OutputType::Error,
        }
    }
}

// ============================================================================
// 工具定义（供LLM使用）
// ============================================================================

/// 工具定义 — 用于向LLM描述可用工具
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

/// 函数定义详情
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

// ============================================================================
// Tool Trait
// ============================================================================

/// 工具trait — 所有AI可用工具必须实现此trait
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具唯一标识名
    fn name(&self) -> &str;
    /// 工具描述（供LLM理解用途）
    fn description(&self) -> &str;
    /// 参数JSON Schema定义
    fn parameters_schema(&self) -> Value;
    /// 执行工具
    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError>;

    /// 生成供LLM使用的工具定义
    fn to_definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: self.name().to_string(),
                description: self.description().to_string(),
                parameters: self.parameters_schema(),
            },
        }
    }
}

// ============================================================================
// 工具注册中心
// ============================================================================

/// 工具注册中心 — 管理所有可用工具
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    /// 创建空的工具注册中心
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// 注册一个工具
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    /// 根据名称获取工具
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// 列出所有已注册工具
    pub fn list(&self) -> Vec<&dyn Tool> {
        self.tools.values().map(|t| t.as_ref()).collect()
    }

    /// 生成所有工具的LLM定义列表
    pub fn definitions_for_llm(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.to_definition()).collect()
    }

    /// 检查工具是否已注册
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// 注销工具
    pub fn unregister(&mut self, name: &str) -> Option<Box<dyn Tool>> {
        self.tools.remove(name)
    }
}

impl Default for ToolRegistry {
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

    /// 测试用的Mock工具
    struct MockTool {
        name: String,
        desc: String,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.desc
        }

        fn parameters_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                }
            })
        }

        async fn execute(&self, _params: Value) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::ok("mock result"))
        }
    }

    #[test]
    fn test_tool_registry_register_and_get() {
        let mut registry = ToolRegistry::new();
        let tool = MockTool {
            name: "mock_tool".to_string(),
            desc: "A mock tool for testing".to_string(),
        };

        registry.register(Box::new(tool));

        assert!(registry.contains("mock_tool"));
        let retrieved = registry.get("mock_tool").unwrap();
        assert_eq!(retrieved.name(), "mock_tool");
        assert_eq!(retrieved.description(), "A mock tool for testing");
    }

    #[test]
    fn test_tool_registry_list() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockTool {
            name: "tool_a".to_string(),
            desc: "Tool A".to_string(),
        }));
        registry.register(Box::new(MockTool {
            name: "tool_b".to_string(),
            desc: "Tool B".to_string(),
        }));

        let tools = registry.list();
        assert_eq!(tools.len(), 2);
    }

    #[test]
    fn test_tool_registry_get_nonexistent() {
        let registry = ToolRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_tool_registry_unregister() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockTool {
            name: "temp_tool".to_string(),
            desc: "Temporary".to_string(),
        }));

        assert!(registry.contains("temp_tool"));
        registry.unregister("temp_tool");
        assert!(!registry.contains("temp_tool"));
    }

    #[test]
    fn test_definitions_for_llm() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockTool {
            name: "test_tool".to_string(),
            desc: "Test description".to_string(),
        }));

        let defs = registry.definitions_for_llm();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].function.name, "test_tool");
        assert_eq!(defs[0].function.description, "Test description");
    }

    #[test]
    fn test_tool_result_helpers() {
        let r1 = ToolResult::ok("success");
        assert!(r1.success);
        assert_eq!(r1.content, "success");
        assert_eq!(r1.output_type, OutputType::Text);

        let r2 = ToolResult::ok_json(r#"{"key":"val"}"#);
        assert!(r2.success);
        assert_eq!(r2.output_type, OutputType::Json);

        let r3 = ToolResult::err("failure");
        assert!(!r3.success);
        assert_eq!(r3.output_type, OutputType::Error);
    }

    #[test]
    fn test_tool_error_display() {
        let e1 = ToolError::InvalidParameters("missing field".to_string());
        assert_eq!(e1.to_string(), "参数错误: missing field");

        let e2 = ToolError::PathNotAllowed {
            path: "/etc/passwd".to_string(),
        };
        assert!(e2.to_string().contains("路径安全检查失败"));
    }
}
