//! 系统环境检测模块
//! 
//! 支持多平台（Linux / Windows / macOS）的 CUDA、GPU、Python 环境检测

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 操作系统类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OsType {
    Linux,
    Windows,
    MacOS,
}

impl std::fmt::Display for OsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsType::Linux => write!(f, "Linux"),
            OsType::Windows => write!(f, "Windows"),
            OsType::MacOS => write!(f, "macOS"),
        }
    }
}

/// GPU 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub memory_mb: u64,
    pub driver_version: String,
}

/// CUDA 环境信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CudaInfo {
    pub available: bool,
    /// 驱动版本，如 "535.154.05"
    pub driver_version: Option<String>,
    /// CUDA Runtime 版本，如 "12.2"
    pub runtime_version: Option<String>,
    /// GPU 列表
    pub gpus: Vec<GpuInfo>,
    /// 检测错误信息
    pub error: Option<String>,
}

impl Default for CudaInfo {
    fn default() -> Self {
        Self {
            available: false,
            driver_version: None,
            runtime_version: None,
            gpus: Vec::new(),
            error: None,
        }
    }
}

/// 系统综合信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: OsType,
    pub os_version: Option<String>,
    pub arch: String,
    pub cpu_cores: usize,
    pub total_memory_mb: u64,
}

/// 完整环境报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvReport {
    pub system: SystemInfo,
    pub cuda: CudaInfo,
    pub uv_installed: bool,
    pub uv_version: Option<String>,
    pub python_installed: bool,
    pub python_version: Option<String>,
    pub venv_exists: bool,
    pub torch_available: bool,
    pub torch_cuda: bool,
    pub ort_available: bool,
    pub ort_cuda: bool,
}

// ============================================================================
// 系统信息检测
// ============================================================================

pub fn detect_os() -> OsType {
    if cfg!(target_os = "linux") {
        OsType::Linux
    } else if cfg!(target_os = "windows") {
        OsType::Windows
    } else if cfg!(target_os = "macos") {
        OsType::MacOS
    } else {
        OsType::Linux
    }
}

pub fn detect_system() -> SystemInfo {
    let os = detect_os();
    let arch = std::env::consts::ARCH.to_string();
    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    
    let total_memory_mb = sysinfo_memory_mb();
    let os_version = detect_os_version(&os);
    
    SystemInfo {
        os,
        os_version,
        arch,
        cpu_cores,
        total_memory_mb,
    }
}

fn detect_os_version(os: &OsType) -> Option<String> {
    match os {
        OsType::Linux => {
            std::fs::read_to_string("/etc/os-release")
                .ok()
                .and_then(|content| {
                    content.lines().find(|l| l.starts_with("PRETTY_NAME="))
                        .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
                })
                .or_else(|| {
                    std::process::Command::new("uname")
                        .arg("-r")
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .map(|s| s.trim().to_string())
                })
        }
        OsType::MacOS => {
            std::process::Command::new("sw_vers")
                .arg("-productVersion")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
        }
        OsType::Windows => {
            std::process::Command::new("cmd")
                .args(["/C", "ver"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn sysinfo_memory_mb() -> u64 {
    // Linux/Windows: 读取 /proc/meminfo 或使用 sysinfo 命令
    if cfg!(target_os = "linux") {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|content| {
                content.lines().next().and_then(|line| {
                    let parts: Vec<_> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[0] == "MemTotal:" {
                        parts[1].parse::<u64>().ok().map(|kb| kb / 1024)
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(0)
    } else {
        // Windows fallback
        0
    }
}

#[cfg(target_os = "macos")]
fn sysinfo_memory_mb() -> u64 {
    std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .and_then(|s| s.trim().parse::<u64>().ok())
                    .map(|bytes| bytes / 1024 / 1024)
            } else {
                None
            }
        })
        .unwrap_or(0)
}

// ============================================================================
// CUDA 检测（多平台）
// ============================================================================

pub fn detect_cuda() -> CudaInfo {
    let mut info = CudaInfo::default();
    
    // macOS 没有 NVIDIA GPU（Apple Silicon 使用 Metal）
    if cfg!(target_os = "macos") {
        info.error = Some("macOS 不支持 NVIDIA CUDA".to_string());
        return info;
    }
    
    // 尝试运行 nvidia-smi
    let nvidia_smi = if cfg!(target_os = "windows") {
        "nvidia-smi.exe"
    } else {
        "nvidia-smi"
    };
    
    match std::process::Command::new(nvidia_smi).output() {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            parse_nvidia_smi(&text, &mut info);
        }
        Ok(output) => {
            let err = String::from_utf8_lossy(&output.stderr);
            info.error = Some(format!("nvidia-smi 失败: {}", err));
        }
        Err(e) => {
            info.error = Some(format!("nvidia-smi 未找到: {}", e));
        }
    }
    
    info
}

fn parse_nvidia_smi(text: &str, info: &mut CudaInfo) {
    // 解析驱动版本和 CUDA Runtime 版本
    // 示例: | NVIDIA-SMI 535.154.05             Driver Version: 535.154.05   CUDA Version: 12.2     |
    for line in text.lines() {
        if line.contains("Driver Version:") {
            if let Some(drv_start) = line.find("Driver Version:") {
                let after = &line[drv_start + "Driver Version:".len()..];
                let ver = after.split_whitespace().next().unwrap_or("");
                if !ver.is_empty() {
                    info.driver_version = Some(ver.to_string());
                }
            }
            if let Some(cuda_start) = line.find("CUDA Version:") {
                let after = &line[cuda_start + "CUDA Version:".len()..];
                let ver = after.split_whitespace().next().unwrap_or("");
                if !ver.is_empty() {
                    info.runtime_version = Some(ver.to_string());
                    info.available = true;
                }
            }
        }
    }
    
    // 解析 GPU 信息
    // 示例: |   0  NVIDIA GeForce RTX 4090        Off | 00000000:01:00.0  On |                  Off |
    let lines: Vec<_> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("|=====") || line.contains("| GPU  ") {
            continue;
        }
        if line.starts_with("|") && line.contains("NVIDIA") && !line.contains("NVIDIA-SMI") && !line.contains("Driver Version") && i + 1 < lines.len() {
            let parts: Vec<_> = line.split('|').collect();
            if parts.len() >= 2 {
                let gpu_part = parts[1];
                // 提取 GPU 名称（从 NVIDIA 开始）
                let name_start = gpu_part.find("NVIDIA").unwrap_or(0);
                let name_end = gpu_part.find("Off").or_else(|| gpu_part.find("On")).unwrap_or(gpu_part.len());
                let name = gpu_part[name_start..name_end].trim().to_string();
                
                // 解析下一行的显存
                let mem_mb = if i + 1 < lines.len() {
                    parse_gpu_memory(lines[i + 1])
                } else {
                    0
                };
                
                if !name.is_empty() {
                    info.gpus.push(GpuInfo {
                        name,
                        memory_mb: mem_mb,
                        driver_version: info.driver_version.clone().unwrap_or_default(),
                    });
                }
            }
        }
    }
}

fn parse_gpu_memory(line: &str) -> u64 {
    // 示例: |  0%   45C    P8              20W / 450W |    852MiB / 24564MiB |      0%      Default |
    if let Some(mem_part) = line.rsplit("|").nth(2) {
        let tokens: Vec<_> = mem_part.split_whitespace().collect();
        if tokens.len() >= 2 {
            let mem_str = tokens.last().unwrap_or(&"");
            if mem_str.ends_with("MiB") {
                return mem_str.trim_end_matches("MiB").parse::<u64>().unwrap_or(0);
            } else if mem_str.ends_with("GiB") {
                return mem_str.trim_end_matches("GiB").parse::<u64>().unwrap_or(0) * 1024;
            }
        }
    }
    0
}

// ============================================================================
// UV / Python 环境检测
// ============================================================================

pub fn detect_uv() -> (bool, Option<String>) {
    match std::process::Command::new("uv").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (true, Some(ver))
        }
        _ => (false, None),
    }
}

pub fn detect_venv_python(venv_path: &PathBuf) -> (bool, Option<String>) {
    let python_exe = if cfg!(target_os = "windows") {
        venv_path.join("Scripts").join("python.exe")
    } else {
        venv_path.join("bin").join("python")
    };
    
    if !python_exe.exists() {
        return (false, None);
    }
    
    match std::process::Command::new(&python_exe)
        .args(["-c", "import sys; print(sys.version_info.major, sys.version_info.minor, sys.version_info.micro)"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (true, Some(ver))
        }
        _ => (true, None),
    }
}

/// 检测 Python 包是否可用（通过 uv run 或直接调用 python）
pub fn check_python_package(python: &PathBuf, package: &str) -> (bool, Option<String>) {
    let script = format!(
        "import {}; print(getattr({}, '__version__', 'unknown'))",
        package, package
    );
    match std::process::Command::new(python)
        .args(["-c", &script])
        .output()
    {
        Ok(output) if output.status.success() => {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (true, Some(ver))
        }
        _ => (false, None),
    }
}

/// 检测 torch CUDA 是否可用
pub fn check_torch_cuda(python: &PathBuf) -> bool {
    let script = "import torch; print(torch.cuda.is_available())";
    match std::process::Command::new(python)
        .args(["-c", script])
        .output()
    {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
            text == "true"
        }
        _ => false,
    }
}

/// 检测 ort CUDA 是否可用（使用 ort 的 CUDA execution provider）
pub fn check_ort_cuda(python: &PathBuf) -> bool {
    let script = r#"
import onnxruntime as ort
providers = ort.get_available_providers()
print('CUDAExecutionProvider' in providers)
"#;
    match std::process::Command::new(python)
        .args(["-c", script])
        .output()
    {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
            text == "true"
        }
        _ => false,
    }
}

// ============================================================================
// 完整环境报告（带缓存）
// ============================================================================

use std::time::{Duration, Instant};

static ENV_REPORT_CACHE: std::sync::OnceLock<std::sync::Mutex<(EnvReport, Instant)>> = std::sync::OnceLock::new();
const REPORT_CACHE_TTL: Duration = Duration::from_secs(60);

fn do_generate_env_report() -> EnvReport {
    let system = detect_system();
    let cuda = detect_cuda();
    let (uv_installed, uv_version) = detect_uv();
    
    let venv_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".rusttools")
        .join("yolo-env");
    let venv_exists = venv_path.exists();
    let (python_installed, python_version) = detect_venv_python(&venv_path);
    
    let python_exe = if cfg!(target_os = "windows") {
        venv_path.join("Scripts").join("python.exe")
    } else {
        venv_path.join("bin").join("python")
    };
    
    let (torch_available, _) = if python_installed {
        check_python_package(&python_exe, "torch")
    } else {
        (false, None)
    };
    let torch_cuda = python_installed && check_torch_cuda(&python_exe);
    
    let (ort_available, _) = if python_installed {
        check_python_package(&python_exe, "onnxruntime")
    } else {
        (false, None)
    };
    let ort_cuda = python_installed && check_ort_cuda(&python_exe);
    
    EnvReport {
        system,
        cuda,
        uv_installed,
        uv_version,
        python_installed,
        python_version,
        venv_exists,
        torch_available,
        torch_cuda,
        ort_available,
        ort_cuda,
    }
}

pub fn generate_env_report() -> EnvReport {
    let cache = ENV_REPORT_CACHE.get_or_init(|| {
        std::sync::Mutex::new((do_generate_env_report(), Instant::now()))
    });
    let mut guard = cache.lock().unwrap();
    if guard.1.elapsed() > REPORT_CACHE_TTL {
        let fresh = do_generate_env_report();
        *guard = (fresh.clone(), Instant::now());
    }
    guard.0.clone()
}

pub fn refresh_env_report() -> EnvReport {
    let cache = ENV_REPORT_CACHE.get_or_init(|| {
        std::sync::Mutex::new((do_generate_env_report(), Instant::now()))
    });
    let fresh = do_generate_env_report();
    *cache.lock().unwrap() = (fresh.clone(), Instant::now());
    fresh
}

/// 获取人类可读的环境状态摘要
pub fn env_status_summary(report: &EnvReport) -> String {
    let mut parts = Vec::new();
    
    parts.push(format!("{} {}", report.system.os, report.system.arch));
    
    if report.cuda.available {
        if let Some(ref ver) = report.cuda.runtime_version {
            parts.push(format!("CUDA {}", ver));
        }
        if let Some(ref gpu) = report.cuda.gpus.first() {
            parts.push(format!("{}", gpu.name));
        }
    } else {
        parts.push("CPU 模式".to_string());
    }
    
    if report.uv_installed {
        parts.push("uv ✓".to_string());
    }
    
    if report.python_installed {
        parts.push("Python ✓".to_string());
    }
    
    if report.torch_available {
        if report.torch_cuda {
            parts.push("PyTorch (GPU) ✓".to_string());
        } else {
            parts.push("PyTorch (CPU) ✓".to_string());
        }
    }
    
    if report.ort_available {
        if report.ort_cuda {
            parts.push("ONNX Runtime (GPU) ✓".to_string());
        } else {
            parts.push("ONNX Runtime (CPU) ✓".to_string());
        }
    }
    
    parts.join(" | ")
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_os_returns_valid_enum() {
        let os = detect_os();
        // Should be one of the three supported OS types
        assert!(
            matches!(os, OsType::Linux | OsType::Windows | OsType::MacOS),
            "detect_os should return a valid OsType"
        );
    }

    #[test]
    fn test_detect_system_has_cores_and_memory() {
        let sys = detect_system();
        assert!(
            sys.cpu_cores > 0,
            "System should have at least 1 CPU core"
        );
        // Architecture should not be empty
        assert!(!sys.arch.is_empty(), "Architecture should be detected");
    }

    #[test]
    fn test_detect_os_version_returns_some_on_linux() {
        if cfg!(target_os = "linux") {
            let os = detect_os();
            let version = detect_os_version(&os);
            // On Linux, we should get something (either from /etc/os-release or uname)
            assert!(
                version.is_some(),
                "OS version should be detectable on Linux"
            );
        }
    }

    #[test]
    fn test_sysinfo_memory_mb_returns_positive() {
        let mem = sysinfo_memory_mb();
        // Memory could be 0 on Windows (fallback), but on Linux/macOS it should be > 0
        if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            assert!(
                mem > 0,
                "Memory should be detectable on Linux/macOS, got {} MB",
                mem
            );
        }
    }

    #[test]
    fn test_parse_nvidia_smi_valid_output() {
        let sample_output = r#"
+---------------------------------------------------------------------------------------+
| NVIDIA-SMI 535.154.05             Driver Version: 535.154.05   CUDA Version: 12.2     |
|-----------------------------------------+----------------------+----------------------+
| GPU  Name                 Persistence-M | Bus-Id        Disp.A | Volatile Uncorr. ECC |
| Fan  Temp   Perf          Pwr:Usage/Cap |         Memory-Usage | GPU-Util  Compute M. |
|                                         |                      |               MIG M. |
|=========================================+======================+======================|
|   0  NVIDIA GeForce RTX 4090        Off | 00000000:01:00.0  On |                  Off |
|  0%   45C    P8              20W / 450W |    852MiB / 24564MiB |      0%      Default |
+-----------------------------------------+----------------------+----------------------+
"#;
        let mut info = CudaInfo::default();
        parse_nvidia_smi(sample_output, &mut info);

        assert!(info.available, "CUDA should be available");
        assert_eq!(
            info.driver_version,
            Some("535.154.05".to_string()),
            "Driver version should be parsed"
        );
        assert_eq!(
            info.runtime_version,
            Some("12.2".to_string()),
            "CUDA runtime version should be parsed"
        );
        assert_eq!(info.gpus.len(), 1, "Should detect 1 GPU");
        assert_eq!(
            info.gpus[0].name, "NVIDIA GeForce RTX 4090",
            "GPU name should be parsed"
        );
        assert_eq!(
            info.gpus[0].memory_mb, 24564,
            "GPU memory should be parsed in MiB"
        );
    }

    #[test]
    fn test_parse_nvidia_smi_no_gpu() {
        let sample_output = "NVIDIA-SMI has failed because it couldn't communicate with the NVIDIA driver.";
        let mut info = CudaInfo::default();
        parse_nvidia_smi(sample_output, &mut info);

        assert!(!info.available, "CUDA should not be available");
        assert!(info.gpus.is_empty(), "No GPUs should be detected");
    }

    #[test]
    fn test_parse_gpu_memory_mib() {
        let line = "|  0%   45C    P8              20W / 450W |    852MiB / 24564MiB |      0%      Default |";
        let mem = parse_gpu_memory(line);
        assert_eq!(mem, 24564, "Should parse MiB memory");
    }

    #[test]
    fn test_parse_gpu_memory_gib() {
        let line = "|  0%   45C    P8              20W / 450W |    1GiB / 24GiB |      0%      Default |";
        let mem = parse_gpu_memory(line);
        assert_eq!(mem, 24 * 1024, "Should parse GiB memory and convert to MiB");
    }

    #[test]
    fn test_parse_gpu_memory_no_match() {
        let line = "some random text without memory info";
        let mem = parse_gpu_memory(line);
        assert_eq!(mem, 0, "Should return 0 for unparseable line");
    }

    #[test]
    fn test_generate_env_report_structure() {
        let report = generate_env_report();

        // System info should be populated
        assert!(
            matches!(
                report.system.os,
                OsType::Linux | OsType::Windows | OsType::MacOS
            ),
            "OS should be detected"
        );
        assert!(
            !report.system.arch.is_empty(),
            "Architecture should be detected"
        );
        assert!(report.system.cpu_cores > 0, "CPU cores should be > 0");

        // CUDA info should exist (may or may not be available depending on system)
        // Just verify the struct is properly constructed
        let _ = report.cuda.available;
        let _ = report.cuda.gpus.len();

        // Python env fields should be boolean
        let _ = report.uv_installed;
        let _ = report.python_installed;
        let _ = report.venv_exists;
        let _ = report.torch_available;
        let _ = report.ort_available;
    }

    #[test]
    fn test_env_status_summary_format() {
        let report = generate_env_report();
        let summary = env_status_summary(&report);

        // Summary should contain OS info
        assert!(
            !summary.is_empty(),
            "Summary should not be empty"
        );
        assert!(
            summary.contains("Linux") || summary.contains("Windows") || summary.contains("macOS"),
            "Summary should contain OS name: {}",
            summary
        );
    }

    #[test]
    fn test_cuda_info_default() {
        let info = CudaInfo::default();
        assert!(!info.available);
        assert!(info.driver_version.is_none());
        assert!(info.runtime_version.is_none());
        assert!(info.gpus.is_empty());
        assert!(info.error.is_none());
    }

    #[test]
    fn test_os_type_display() {
        assert_eq!(format!("{}", OsType::Linux), "Linux");
        assert_eq!(format!("{}", OsType::Windows), "Windows");
        assert_eq!(format!("{}", OsType::MacOS), "macOS");
    }
}
