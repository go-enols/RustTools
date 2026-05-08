//! 系统信息工具
//!
//! 让 AI 能够获取当前系统的硬件和软件环境信息，
//! 包括操作系统、CPU、内存、GPU、Python 环境等。

use super::tool::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

/// 系统信息查询工具
pub struct SystemInfoTool;

impl SystemInfoTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SystemInfoTool {
    fn name(&self) -> &str {
        "system_info"
    }

    fn description(&self) -> &str {
        "获取当前系统的环境信息，包括操作系统、CPU、内存、GPU、Python环境等。\
         当用户询问系统配置、硬件信息或环境状态时调用此工具。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "category": {
                    "type": "string",
                    "enum": ["all", "os", "cpu", "memory", "gpu", "python", "project"],
                    "description": "信息类别，all 返回全部信息"
                }
            }
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let category = params
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        let info = gather_system_info(category)?;
        Ok(ToolResult::ok_json(info))
    }
}

/// 收集系统信息
fn gather_system_info(category: &str) -> Result<String, ToolError> {
    use crate::services::env::{detect_system, detect_cuda, generate_env_report};

    match category {
        "all" => {
            let sys = detect_system();
            let cuda = detect_cuda();
            let report = generate_env_report();
            let cwd = crate::agent::workspace::current_dir();
            let ws = crate::agent::workspace::workspace_path();

            let info = serde_json::json!({
                "os": format!("{} {:?} {}", sys.os, sys.os_version.as_deref().unwrap_or(""), sys.arch),
                "cpu_cores": sys.cpu_cores,
                "memory_mb": sys.total_memory_mb,
                "cuda_available": cuda.available,
                "gpus": cuda.gpus.iter().map(|g| {
                    serde_json::json!({
                        "name": g.name,
                        "memory_mb": g.memory_mb,
                        "driver": g.driver_version
                    })
                }).collect::<Vec<_>>(),
                "python": {
                    "installed": report.python_installed,
                    "version": report.python_version,
                    "venv_exists": report.venv_exists,
                    "torch_available": report.torch_available,
                    "torch_cuda": report.torch_cuda,
                    "ort_available": report.ort_available,
                    "ort_cuda": report.ort_cuda
                },
                "current_working_directory": cwd,
                "workspace_path": ws,
            });
            serde_json::to_string_pretty(&info)
                .map_err(|e| ToolError::JsonParse(e.to_string()))
        }
        "os" => {
            let sys = detect_system();
            let info = serde_json::json!({
                "os": format!("{:?}", sys.os),
                "version": sys.os_version,
                "arch": sys.arch,
            });
            serde_json::to_string_pretty(&info)
                .map_err(|e| ToolError::JsonParse(e.to_string()))
        }
        "cpu" => {
            let sys = detect_system();
            let info = serde_json::json!({
                "cpu_cores": sys.cpu_cores,
            });
            serde_json::to_string_pretty(&info)
                .map_err(|e| ToolError::JsonParse(e.to_string()))
        }
        "memory" => {
            let sys = detect_system();
            let info = serde_json::json!({
                "total_memory_mb": sys.total_memory_mb,
            });
            serde_json::to_string_pretty(&info)
                .map_err(|e| ToolError::JsonParse(e.to_string()))
        }
        "gpu" => {
            let cuda = detect_cuda();
            let info = serde_json::json!({
                "cuda_available": cuda.available,
                "driver_version": cuda.driver_version,
                "runtime_version": cuda.runtime_version,
                "gpus": cuda.gpus,
            });
            serde_json::to_string_pretty(&info)
                .map_err(|e| ToolError::JsonParse(e.to_string()))
        }
        "python" => {
            let report = generate_env_report();
            let info = serde_json::json!({
                "installed": report.python_installed,
                "version": report.python_version,
                "venv_exists": report.venv_exists,
                "torch_available": report.torch_available,
                "torch_cuda": report.torch_cuda,
                "ort_available": report.ort_available,
                "ort_cuda": report.ort_cuda,
                "uv_installed": report.uv_installed,
                "uv_version": report.uv_version,
            });
            serde_json::to_string_pretty(&info)
                .map_err(|e| ToolError::JsonParse(e.to_string()))
        }
        "project" => {
            let cwd = crate::agent::workspace::current_dir();
            let ws = crate::agent::workspace::workspace_path();
            let info = serde_json::json!({
                "current_working_directory": cwd,
                "workspace_path": ws,
            });
            serde_json::to_string_pretty(&info)
                .map_err(|e| ToolError::JsonParse(e.to_string()))
        }
        _ => Err(ToolError::InvalidParameters(format!(
            "未知类别: {}，可选: all, os, cpu, memory, gpu, python, project",
            category
        ))),
    }
}
