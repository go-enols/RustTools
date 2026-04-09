use serde::{Deserialize, Serialize};
use std::process::Command;

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: i32,
    pub name: String,
    #[serde(rename = "type")]
    pub device_type: String, // "GPU" or "CPU"
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_free: u64,
    pub driver_version: String,
}

/// GPU utilization stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStats {
    pub gpu_util: f32,
    pub memory_util: f32,
    pub temperature: f32,
}

/// Probe GPU info using nvidia-smi
fn probe_nvidia_smi() -> Option<Vec<DeviceInfo>> {
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=index,name,memory.total,memory.used,memory.free,driver_version", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() >= 6 {
            let id = parts[0].parse::<i32>().ok()?;
            let name = parts[1].to_string();
            let memory_total = parts[2].parse::<u64>().ok()? * 1024 * 1024; // MB to bytes
            let memory_used = parts[3].parse::<u64>().ok()? * 1024 * 1024;
            let memory_free = parts[4].parse::<u64>().ok()? * 1024 * 1024;
            let driver_version = parts[5].to_string();

            devices.push(DeviceInfo {
                id,
                name,
                device_type: "GPU".to_string(),
                memory_total,
                memory_used,
                memory_free,
                driver_version,
            });
        }
    }

    if devices.is_empty() {
        None
    } else {
        Some(devices)
    }
}

/// Get GPU utilization stats using nvidia-smi
pub fn get_gpu_stats(device_id: i32) -> Option<DeviceStats> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=utilization.gpu,utilization.memory,temperature.gpu",
            "--format=csv,noheader,nounits",
            "-i", &device_id.to_string(),
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().split(',').map(|s| s.trim()).collect();

    if parts.len() >= 3 {
        let gpu_util = parts[0].parse::<f32>().ok()?;
        let memory_util = parts[1].parse::<f32>().ok()?;
        let temperature = parts[2].parse::<f32>().ok()?;
        return Some(DeviceStats { gpu_util, memory_util, temperature });
    }

    None
}

/// List all available compute devices
pub fn list_devices() -> Vec<DeviceInfo> {
    // Try NVIDIA first
    if let Some(gpus) = probe_nvidia_smi() {
        return gpus;
    }

    // Fallback: CPU-only system
    // Get system memory info (Linux)
    let (memory_total, memory_used) = if let Some(mem_info) = get_system_memory() {
        mem_info
    } else {
        (8 * 1024 * 1024 * 1024, 0) // Default 8GB
    };

    vec![DeviceInfo {
        id: -1,
        name: "CPU".to_string(),
        device_type: "CPU".to_string(),
        memory_total,
        memory_used,
        memory_free: memory_total.saturating_sub(memory_used),
        driver_version: "N/A".to_string(),
    }]
}

/// Get system memory info on Linux (from /proc/meminfo)
fn get_system_memory() -> Option<(u64, u64)> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut mem_total: u64 = 0;
    let mut mem_available: u64 = 0;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            mem_total = parse_meminfo_line(line)?;
        } else if line.starts_with("MemAvailable:") {
            mem_available = parse_meminfo_line(line)?;
        }
    }

    if mem_total == 0 {
        return None;
    }

    let mem_used = mem_total.saturating_sub(mem_available);
    Some((mem_total * 1024, mem_used * 1024)) // KB to bytes
}

fn parse_meminfo_line(line: &str) -> Option<u64> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse::<u64>().ok()
    } else {
        None
    }
}
