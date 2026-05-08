//! 进程列表工具
//!
//! 让 AI 能够查看当前系统运行的进程信息。

use super::tool::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

/// 进程列表查询工具
pub struct ListProcessesTool;

impl ListProcessesTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ListProcessesTool {
    fn name(&self) -> &str {
        "list_processes"
    }

    fn description(&self) -> &str {
        "获取当前系统运行的进程列表。\
         在需要诊断系统资源占用、查找特定程序进程或排查端口占用时调用此工具。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "filter": {
                    "type": "string",
                    "description": "按进程名过滤，不区分大小写"
                },
                "limit": {
                    "type": "integer",
                    "description": "返回的最大进程数，默认 50"
                }
            }
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let filter = params.get("filter").and_then(|v| v.as_str());
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|l| (l as usize).clamp(1, 200))
            .unwrap_or(50);

        let processes = gather_processes(filter, limit)?;

        let result = serde_json::json!({
            "count": processes.len(),
            "filter": filter,
            "processes": processes,
        });

        let json_str = serde_json::to_string_pretty(&result)
            .map_err(|e| ToolError::JsonParse(e.to_string()))?;

        Ok(ToolResult::ok_json(json_str))
    }
}

/// 收集进程信息
fn gather_processes(filter: Option<&str>, limit: usize) -> Result<Vec<Value>, ToolError> {
    let mut processes = Vec::new();

    #[cfg(target_os = "linux")]
    {
        use std::fs;

        let entries = fs::read_dir("/proc")
            .map_err(|e| ToolError::Io(e))?;

        for entry in entries.flatten() {
            if processes.len() >= limit {
                break;
            }

            let name = entry.file_name();
            let pid_str = name.to_string_lossy();
            if pid_str.parse::<u32>().is_err() {
                continue;
            }

            let status_path = entry.path().join("status");
            let cmdline_path = entry.path().join("cmdline");

            if let Ok(status) = fs::read_to_string(&status_path) {
                let mut proc_name = String::new();
                let mut proc_pid = pid_str.to_string();
                let mut proc_ppid = String::new();
                let mut proc_mem_kb = 0u64;

                for line in status.lines() {
                    if line.starts_with("Name:") {
                        proc_name = line[5..].trim().to_string();
                    } else if line.starts_with("Pid:") {
                        proc_pid = line[4..].trim().to_string();
                    } else if line.starts_with("PPid:") {
                        proc_ppid = line[5..].trim().to_string();
                    } else if line.starts_with("VmRSS:") {
                        let mem_str = line[6..].trim().trim_end_matches(" kB").trim();
                        proc_mem_kb = mem_str.parse().unwrap_or(0);
                    }
                }

                // 应用过滤
                if let Some(f) = filter {
                    if !proc_name.to_lowercase().contains(&f.to_lowercase()) {
                        continue;
                    }
                }

                // 读取命令行
                let cmdline = fs::read_to_string(&cmdline_path)
                    .unwrap_or_default()
                    .replace('\0', " ");

                processes.push(serde_json::json!({
                    "pid": proc_pid,
                    "ppid": proc_ppid,
                    "name": proc_name,
                    "memory_kb": proc_mem_kb,
                    "cmdline": cmdline.trim(),
                }));
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("ps")
            .args(["-axo", "pid,ppid,comm,rss,args"])
            .output()
            .map_err(|e| ToolError::CommandExecution(format!("执行 ps 失败: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for (i, line) in stdout.lines().enumerate() {
            if i == 0 {
                continue; // 跳过表头
            }
            if processes.len() >= limit {
                break;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                continue;
            }

            let pid = parts[0];
            let ppid = parts[1];
            let comm = parts[2];
            let rss = parts[3].parse().unwrap_or(0);
            let args = parts[4..].join(" ");

            if let Some(f) = filter {
                if !comm.to_lowercase().contains(&f.to_lowercase())
                    && !args.to_lowercase().contains(&f.to_lowercase())
                {
                    continue;
                }
            }

            processes.push(serde_json::json!({
                "pid": pid,
                "ppid": ppid,
                "name": comm,
                "memory_kb": rss,
                "cmdline": args,
            }));
        }
    }

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .output()
            .map_err(|e| ToolError::CommandExecution(format!("执行 tasklist 失败: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if processes.len() >= limit {
                break;
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 5 {
                continue;
            }

            let name = parts[0].trim_matches('"');
            let pid = parts[1].trim_matches('"');
            let mem = parts[4].trim_matches('"').replace(" K", "").replace(",", "");

            if let Some(f) = filter {
                if !name.to_lowercase().contains(&f.to_lowercase()) {
                    continue;
                }
            }

            processes.push(serde_json::json!({
                "pid": pid,
                "name": name,
                "memory_kb": mem.parse().unwrap_or(0),
            }));
        }
    }

    Ok(processes)
}
