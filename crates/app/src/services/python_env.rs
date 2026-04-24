use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};


// ============================================================================
// 安装方案与配置类型
// ============================================================================

/// 国内镜像源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MirrorSource {
    Default,
    Tsinghua,
    Aliyun,
    USTC,
}

impl MirrorSource {
    pub fn label(&self) -> &'static str {
        match self {
            MirrorSource::Default => "官方源",
            MirrorSource::Tsinghua => "清华 TUNA",
            MirrorSource::Aliyun => "阿里云",
            MirrorSource::USTC => "中科大 USTC",
        }
    }

    pub fn pypi_url(&self) -> &'static str {
        match self {
            MirrorSource::Default => "https://pypi.org/simple",
            MirrorSource::Tsinghua => "https://pypi.tuna.tsinghua.edu.cn/simple",
            MirrorSource::Aliyun => "https://mirrors.aliyun.com/pypi/simple/",
            MirrorSource::USTC => "https://pypi.mirrors.ustc.edu.cn/simple/",
        }
    }
}

/// PyTorch CUDA 索引
#[derive(Debug, Clone)]
pub struct TorchIndex {
    pub url: String,
    pub label: String,
}

/// 安装方案
#[derive(Debug, Clone)]
pub struct InstallPlan {
    /// Python 版本
    pub python_version: String,
    /// 主索引（国内镜像）
    pub primary_index: String,
    /// PyTorch 额外索引（官方 CUDA/CPU 索引）
    pub torch_index: Option<String>,
    /// 需要安装的包列表
    pub packages: Vec<String>,
    /// 方案描述
    pub description: String,
    /// 是否为 GPU 方案
    pub is_gpu: bool,
    /// 警告信息（如 CUDA 版本不兼容提示）
    pub warning: Option<String>,
}

/// 安装阶段进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    pub stage: String,
    pub message: String,
    pub progress: Option<f32>,
}

/// 安装结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub message: String,
    pub python_version: Option<String>,
    pub torch_version: Option<String>,
}

/// Python 环境状态（仅检测 uv 内部虚拟环境）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonEnvStatus {
    pub python_available: bool,
    pub python_version: Option<String>,
    pub torch_available: bool,
    pub torch_version: Option<String>,
    pub cuda_available: bool,
    pub installing: bool,
    pub detection_error: Option<String>,
}

/// Global install lock to prevent concurrent installations
static INSTALL_LOCK: std::sync::OnceLock<Arc<Mutex<bool>>> = std::sync::OnceLock::new();

pub fn get_install_lock() -> Arc<Mutex<bool>> {
    INSTALL_LOCK
        .get_or_init(|| Arc::new(Mutex::new(false)))
        .clone()
}

// ============================================================================
// UvManager - Unified Python environment management using `uv`
// ============================================================================

pub struct UvManager {
    pub uv_path: Option<PathBuf>,
    venv_path: PathBuf,
    python_path: PathBuf,
}

impl UvManager {
    pub fn new() -> Self {
        let venv_path = Self::default_venv_path();
        let python_path = if cfg!(target_os = "windows") {
            venv_path.join("Scripts").join("python.exe")
        } else {
            venv_path.join("bin").join("python")
        };
        Self {
            uv_path: Self::detect_uv(),
            venv_path,
            python_path,
        }
    }

    /// Default venv location: ~/.rusttools/yolo-env
    fn default_venv_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".rusttools")
            .join("yolo-env")
    }

    /// Detect if `uv` is available in PATH or common locations
    pub fn detect_uv() -> Option<PathBuf> {
        // Check PATH
        if let Ok(output) = Command::new("uv").arg("--version").output() {
            if output.status.success() {
                return Some(PathBuf::from("uv"));
            }
        }

        // Check common installation paths
        let candidates = if cfg!(target_os = "windows") {
            vec![
                dirs::home_dir().map(|h| h.join(".cargo").join("bin").join("uv.exe")),
            ]
        } else {
            vec![
                dirs::home_dir().map(|h| h.join(".cargo").join("bin").join("uv")),
                Some(PathBuf::from("/usr/local/bin/uv")),
                Some(PathBuf::from("/usr/bin/uv")),
            ]
        };

        for candidate in candidates.into_iter().flatten() {
            if candidate.exists() {
                if Command::new(&candidate).arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
                    return Some(candidate);
                }
            }
        }

        None
    }

    /// Auto-install uv using the official installer script
    pub async fn install_uv() -> Result<PathBuf, String> {
        let install_script = if cfg!(target_os = "windows") {
            // Windows: use PowerShell
            return Err("Auto-install uv on Windows: please install manually from https://docs.astral.sh/uv/getting-started/installation/".to_string());
        } else {
            "curl -LsSf https://astral.sh/uv/install.sh | sh"
        };

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(install_script)
            .output()
            .await
            .map_err(|e| format!("Failed to run uv installer: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("uv installation failed: {}", stderr));
        }

        // After installation, try to detect again
        std::thread::sleep(std::time::Duration::from_secs(1));
        Self::detect_uv().ok_or_else(|| "uv installed but not found in PATH. Please restart the application.".to_string())
    }

    /// Check if the managed venv exists
    pub fn venv_exists(&self) -> bool {
        self.python_path.exists()
    }

    /// Create the venv using uv
    pub async fn create_venv(&self) -> Result<(), String> {
        let uv = self.uv_path.as_ref().ok_or("uv not found")?;
        
        let output = tokio::process::Command::new(uv)
            .args(["venv", "--python", "python3.11"])
            .arg(&self.venv_path)
            .output()
            .await
            .map_err(|e| format!("Failed to create venv: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Try without explicit python version
            let output2 = tokio::process::Command::new(uv)
                .args(["venv"])
                .arg(&self.venv_path)
                .output()
                .await
                .map_err(|e| format!("Failed to create venv (fallback): {}", e))?;
            if !output2.status.success() {
                return Err(format!("Failed to create venv: {}", stderr));
            }
        }

        Ok(())
    }

    /// Get the Python executable path
    pub fn python_path(&self) -> Option<&Path> {
        if self.python_path.exists() {
            Some(&self.python_path)
        } else {
            None
        }
    }

    /// Get the uv path if available
    pub fn uv_path(&self) -> Option<&PathBuf> {
        self.uv_path.as_ref()
    }

    /// Check if uv is available (lazy check)
    pub fn ensure_uv(&mut self) -> Result<&PathBuf, String> {
        if self.uv_path.is_none() {
            self.uv_path = Self::detect_uv();
        }
        self.uv_path.as_ref().ok_or_else(|| "uv not found".to_string())
    }

    /// 根据 InstallPlan 安装依赖
    pub async fn install_with_plan(
        &self,
        plan: &InstallPlan,
        on_progress: impl Fn(String),
    ) -> Result<(), String> {
        let uv = self.uv_path.as_ref().ok_or("uv not found")?;
        
        on_progress(format!("安装方案: {}", plan.description));
        on_progress(format!("Python 版本: {}", plan.python_version));
        if let Some(ref torch_idx) = plan.torch_index {
            on_progress(format!("PyTorch 索引: {}", torch_idx));
        }
        on_progress(format!("包列表: {}", plan.packages.join(", ")));

        // 安装 torch 相关（使用 PyTorch 官方索引）
        if let Some(ref torch_index) = plan.torch_index {
            on_progress("正在安装 PyTorch（含 CUDA 支持）...".to_string());
            let mut cmd = tokio::process::Command::new(uv);
            cmd.arg("pip")
                .arg("install")
                .arg("--python")
                .arg(&self.python_path)
                .arg("--index-url")
                .arg(torch_index)
                .args(["torch", "torchvision"]);
            
            let output = cmd.output().await
                .map_err(|e| format!("安装 torch 失败: {}", e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("torch 安装失败: {}", stderr));
            }
            on_progress("PyTorch 安装完成".to_string());
        } else {
            // CPU 模式，使用主索引安装 torch
            on_progress("正在安装 PyTorch（CPU 版本）...".to_string());
            let mut cmd = tokio::process::Command::new(uv);
            cmd.arg("pip")
                .arg("install")
                .arg("--python")
                .arg(&self.python_path)
                .args(["torch", "torchvision"]);
            
            let output = cmd.output().await
                .map_err(|e| format!("安装 torch 失败: {}", e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("torch 安装失败: {}", stderr));
            }
            on_progress("PyTorch 安装完成".to_string());
        }

        // 安装其他包（使用主索引）
        let other_packages: Vec<&str> = plan.packages.iter()
            .filter(|p| !p.starts_with("torch"))
            .map(|s| s.as_str())
            .collect();
        
        if !other_packages.is_empty() {
            on_progress(format!("正在安装其他依赖: {}...", other_packages.join(", ")));
            let mut cmd = tokio::process::Command::new(uv);
            cmd.arg("pip")
                .arg("install")
                .arg("--python")
                .arg(&self.python_path)
                .args(&other_packages);
            
            let output = cmd.output().await
                .map_err(|e| format!("安装依赖失败: {}", e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("依赖安装失败: {}", stderr));
            }
            on_progress("所有依赖安装完成".to_string());
        }

        Ok(())
    }

    /// 兼容旧接口：从 pyproject.toml 安装（已废弃，使用 install_with_plan）
    pub async fn install_deps(
        &self,
        on_progress: impl Fn(String),
    ) -> Result<(), String> {
        let plan = Self::generate_install_plan(MirrorSource::Default);
        self.install_with_plan(&plan, on_progress).await
    }

    /// Check if NVIDIA GPU is available
    fn check_nvidia_gpu() -> bool {
        Command::new("nvidia-smi")
            .arg("-L")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// 生成安装方案（CUDA 感知）
    pub fn generate_install_plan(mirror: MirrorSource) -> InstallPlan {
        use crate::services::env::detect_cuda;
        
        let cuda = detect_cuda();
        let primary_index = mirror.pypi_url().to_string();
        
        if cfg!(target_os = "macos") {
            // macOS 使用 CPU 版本（Apple Silicon 可用 MPS，但 PyTorch 的 MPS 支持通过 CPU wheel 提供）
            return InstallPlan {
                python_version: "3.11".to_string(),
                primary_index,
                torch_index: None,
                packages: vec![
                    "torch".to_string(),
                    "torchvision".to_string(),
                    "onnxruntime".to_string(),
                    "opencv-python".to_string(),
                    "numpy".to_string(),
                    "pillow".to_string(),
                ],
                description: "macOS CPU 模式（Apple Silicon 自动使用 MPS 加速）".to_string(),
                is_gpu: false,
                warning: None,
            };
        }
        
        if !cuda.available {
            // 无 NVIDIA GPU — CPU 模式
            return InstallPlan {
                python_version: "3.11".to_string(),
                primary_index,
                torch_index: Some("https://download.pytorch.org/whl/cpu".to_string()),
                packages: vec![
                    "torch".to_string(),
                    "torchvision".to_string(),
                    "onnxruntime".to_string(),
                    "opencv-python".to_string(),
                    "numpy".to_string(),
                    "pillow".to_string(),
                ],
                description: "CPU 模式（无 NVIDIA GPU  detected）".to_string(),
                is_gpu: false,
                warning: Some("未检测到 NVIDIA GPU，将使用 CPU 推理。如需 GPU 加速，请安装 NVIDIA 显卡驱动和 CUDA 12.x。".to_string()),
            };
        }
        
        // 解析 CUDA runtime 版本（如 "12.2" → 12）
        let cuda_major = cuda.runtime_version.as_ref()
            .and_then(|v| v.split('.').next())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        
        match cuda_major {
            12 => {
                // CUDA 12.x — 使用 cu124（PyTorch 2.4+ 推荐）
                InstallPlan {
                    python_version: "3.11".to_string(),
                    primary_index,
                    torch_index: Some("https://download.pytorch.org/whl/cu124".to_string()),
                    packages: vec![
                        "torch".to_string(),
                        "torchvision".to_string(),
                        "onnxruntime-gpu".to_string(),
                        "opencv-python".to_string(),
                        "numpy".to_string(),
                        "pillow".to_string(),
                    ],
                    description: format!("GPU 加速模式（CUDA {}）", cuda.runtime_version.clone().unwrap_or_default()),
                    is_gpu: true,
                    warning: None,
                }
            }
            11 => {
                // CUDA 11.x — 警告：ORT 1.19+ 不再在 PyPI 提供 CUDA 11 版本
                InstallPlan {
                    python_version: "3.11".to_string(),
                    primary_index,
                    torch_index: Some("https://download.pytorch.org/whl/cu118".to_string()),
                    packages: vec![
                        "torch".to_string(),
                        "torchvision".to_string(),
                        "onnxruntime".to_string(),  // fallback 到 CPU 版本
                        "opencv-python".to_string(),
                        "numpy".to_string(),
                        "pillow".to_string(),
                    ],
                    description: format!("GPU 模式（CUDA {}，部分兼容）", cuda.runtime_version.clone().unwrap_or_default()),
                    is_gpu: true,
                    warning: Some("检测到 CUDA 11.x。ONNX Runtime GPU 1.19+ 不再支持 CUDA 11，将使用 CPU 版本的 ONNX Runtime。建议升级 NVIDIA 驱动以支持 CUDA 12.x。".to_string()),
                }
            }
            _ => {
                // 未知 CUDA 版本 — 回退 CPU
                InstallPlan {
                    python_version: "3.11".to_string(),
                    primary_index,
                    torch_index: None,
                    packages: vec![
                        "torch".to_string(),
                        "torchvision".to_string(),
                        "onnxruntime".to_string(),
                        "opencv-python".to_string(),
                        "numpy".to_string(),
                        "pillow".to_string(),
                    ],
                    description: "CPU 回退模式（CUDA 版本未知或不兼容）".to_string(),
                    is_gpu: false,
                    warning: Some(format!("检测到不支持的 CUDA 版本 {:?}，将使用 CPU 模式。", cuda.runtime_version)),
                }
            }
        }
    }

    /// 配置 uv 使用国内镜像（写入 ~/.config/uv/uv.toml）
    pub fn configure_mirror(mirror: MirrorSource) -> Result<(), String> {
        if mirror == MirrorSource::Default {
            // 删除配置文件（恢复默认）
            let config_path = Self::uv_config_path();
            if config_path.exists() {
                std::fs::remove_file(&config_path).map_err(|e| format!("删除 uv 配置失败: {}", e))?;
            }
            return Ok(());
        }
        
        let config_dir = dirs::config_dir()
            .or_else(dirs::home_dir)
            .map(|h| h.join(".config").join("uv"))
            .unwrap_or_else(|| PathBuf::from(".config/uv"));
        
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("创建 uv 配置目录失败: {}", e))?;
        
        let config_content = format!(
            r#"[[index]]
url = "{}"
default = true

[pip]
index-url = "{}"
"#,
            mirror.pypi_url(),
            mirror.pypi_url()
        );
        
        let config_path = config_dir.join("uv.toml");
        std::fs::write(&config_path, config_content)
            .map_err(|e| format!("写入 uv 配置失败: {}", e))?;
        
        Ok(())
    }

    fn uv_config_path() -> PathBuf {
        dirs::config_dir()
            .or_else(dirs::home_dir)
            .map(|h| h.join(".config").join("uv").join("uv.toml"))
            .unwrap_or_else(|| PathBuf::from(".config/uv/uv.toml"))
    }

    /// Find pyproject.toml by checking multiple locations
    fn find_pyproject_toml() -> Option<PathBuf> {
        // Try current working directory
        let cwd_pyproject = PathBuf::from("pyproject.toml");
        if cwd_pyproject.exists() {
            return Some(cwd_pyproject);
        }

        // Try executable directory
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let exe_pyproject = exe_dir.join("pyproject.toml");
                if exe_pyproject.exists() {
                    return Some(exe_pyproject);
                }
                // Try parent of executable directory (for target/debug or target/release)
                if let Some(parent) = exe_dir.parent() {
                    let parent_pyproject = parent.join("pyproject.toml");
                    if parent_pyproject.exists() {
                        return Some(parent_pyproject);
                    }
                    // Try one more level up
                    if let Some(grandparent) = parent.parent() {
                        let gp_pyproject = grandparent.join("pyproject.toml");
                        if gp_pyproject.exists() {
                            return Some(gp_pyproject);
                        }
                    }
                }
            }
        }

        // Try CARGO_MANIFEST_DIR (available in development)
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let manifest_pyproject = PathBuf::from(&manifest_dir).join("pyproject.toml");
            if manifest_pyproject.exists() {
                return Some(manifest_pyproject);
            }
            // Try workspace root (go up from crates/app/ to workspace root)
            let mut current = PathBuf::from(&manifest_dir);
            for _ in 0..3 {
                if let Some(parent) = current.parent() {
                    let ws_pyproject = parent.join("pyproject.toml");
                    if ws_pyproject.exists() {
                        return Some(ws_pyproject);
                    }
                    current = parent.to_path_buf();
                } else {
                    break;
                }
            }
        }

        None
    }
}

// ============================================================================
// Legacy compatibility functions
// ============================================================================

/// List of python executables to try (in order of preference)
const PYTHON_CANDIDATES: &[&str] = &[
    "python3.11",
    "python3.10",
    "python3.9",
    "python3",
    "python",
    "/usr/bin/python3",
    "/usr/local/bin/python3",
    "/usr/bin/python3.11",
    "/usr/local/bin/python3.11",
    "py",
    "C:\\Python312\\python.exe",
    "C:\\Python311\\python.exe",
    "C:\\Python310\\python.exe",
    "C:\\Program Files\\Python312\\python.exe",
    "C:\\Program Files\\Python311\\python.exe",
];

/// Resolve the actual python path by trying multiple executables directly.
/// Prefers the managed venv, then falls back to system Python.
pub fn resolve_python_path() -> Option<String> {
    // First, check the managed venv
    let manager = UvManager::new();
    if let Some(path) = manager.python_path() {
        if Command::new(path).arg("--version").output().ok()?.status.success() {
            return Some(path.to_string_lossy().to_string());
        }
    }

    // Check HOME-based dynamic paths
    if let Ok(home) = std::env::var("HOME") {
        let home_hermes_python = format!("{}/.hermes/hermes-agent/venv/bin/python", home);
        if Command::new(&home_hermes_python).arg("--version").output().ok()?.status.success() {
            return Some(home_hermes_python);
        }
    }

    // Try system candidates
    for &python in PYTHON_CANDIDATES {
        if Command::new(python).arg("--version").output().ok()?.status.success() {
            return Some(python.to_string());
        }
    }
    None
}

// Cache for the resolved python path
thread_local! {
    static RESOLVED_PYTHON: std::cell::OnceCell<String> = std::cell::OnceCell::new();
}

/// Get the cached resolved python path, or resolve and cache it
pub fn resolved_python() -> Option<String> {
    RESOLVED_PYTHON.with(|cell| cell.get().cloned()).or_else(|| {
        let path = resolve_python_path();
        if let Some(ref p) = path {
            RESOLVED_PYTHON.with(|cell| { let _ = cell.set(p.clone()); });
        }
        path
    })
}

/// Check if conda/mamba environment is active
pub fn check_conda() -> (bool, bool, Option<String>) {
    if let Ok(mamba_env) = std::env::var("MAMBA_DEFAULT_ENV") {
        if !mamba_env.is_empty() {
            return (false, true, Some(mamba_env));
        }
    }
    if let Ok(conda_env) = std::env::var("CONDA_DEFAULT_ENV") {
        if !conda_env.is_empty() {
            return (true, false, Some(conda_env));
        }
    }
    (false, false, None)
}

/// Check if Python is available and get its version
pub fn check_python() -> Option<String> {
    let python = resolved_python()?;
    let output = Command::new(&python)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let version_str = String::from_utf8_lossy(&output.stdout).to_string();
    let version = version_str.trim().replace("Python ", "");
    Some(version)
}

/// Check if PyTorch is available
pub fn check_torch() -> Option<String> {
    let python = resolved_python()?;
    let output = Command::new(&python)
        .args(["-c", "import torch; print(torch.__version__)"])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some(version)
}

/// Check if CUDA is available via PyTorch
pub fn check_cuda() -> bool {
    let python = match resolved_python() {
        Some(p) => p,
        None => return false,
    };
    let output = Command::new(&python)
        .args(["-c", "import torch; print(torch.cuda.is_available())"])
        .output()
        .ok()
        .filter(|o| o.status.success());
    output.map_or(false, |o| {
        String::from_utf8_lossy(&o.stdout).trim().contains("True")
    })
}

// ============================================================================
// Environment status cache
// ============================================================================

use std::time::{Duration, Instant};

static ENV_STATUS_CACHE: std::sync::OnceLock<std::sync::Mutex<(PythonEnvStatus, Instant)>> = std::sync::OnceLock::new();
const CACHE_TTL: Duration = Duration::from_secs(30);

fn get_status_cache() -> std::sync::Mutex<(PythonEnvStatus, Instant)> {
    std::sync::Mutex::new((
        PythonEnvStatus {
            python_available: false,
            python_version: None,
            torch_available: false,
            torch_version: None,
            cuda_available: false,
            installing: false,
            detection_error: None,
        },
        Instant::now() - CACHE_TTL * 2,
    ))
}

fn do_check_env() -> PythonEnvStatus {
    let python_version = check_python();
    let torch_version = check_torch();
    let installing = *get_install_lock().lock().unwrap();

    let detection_error = if python_version.is_none() {
        Some("Python not found. Please install Python 3.8+ or use the Settings page to set up the environment.".to_string())
    } else {
        None
    };

    PythonEnvStatus {
        python_available: python_version.is_some(),
        python_version,
        torch_available: torch_version.is_some(),
        torch_version,
        cuda_available: check_cuda(),
        installing,
        detection_error,
    }
}

/// Get cached env status (auto-refresh on first call or cache expiry)
pub fn get_env_status() -> PythonEnvStatus {
    let cache = ENV_STATUS_CACHE.get_or_init(get_status_cache);
    let mut guard = cache.lock().unwrap();
    if guard.1.elapsed() > CACHE_TTL {
        let fresh = do_check_env();
        *guard = (fresh.clone(), Instant::now());
    }
    guard.0.clone()
}

/// Force refresh env status (bypass cache)
pub fn refresh_env_status() -> PythonEnvStatus {
    let cache = ENV_STATUS_CACHE.get_or_init(get_status_cache);
    let fresh = do_check_env();
    *cache.lock().unwrap() = (fresh.clone(), Instant::now());
    fresh
}

/// Progress callback type
pub type ProgressCallback = Box<dyn Fn(InstallProgress) + Send + Sync>;
/// Done callback type
pub type DoneCallback = Box<dyn Fn(InstallResult) + Send + Sync>;

/// Install Python dependencies using uv (legacy callback interface)
pub fn install_python_deps(
    on_progress: Option<ProgressCallback>,
    on_done: Option<DoneCallback>,
) {
    let lock = get_install_lock();
    {
        let mut locked = lock.lock().unwrap();
        if *locked {
            if let Some(cb) = on_done {
                cb(InstallResult {
                    success: false,
                    message: "Installation already in progress".to_string(),
                    python_version: None,
                    torch_version: None,
                });
            }
            return;
        }
        *locked = true;
    }

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        rt.block_on(async {
            if let Some(ref cb) = on_progress {
                cb(InstallProgress {
                    stage: "starting".to_string(),
                    message: "Starting Python environment setup with uv...".to_string(),
                    progress: Some(0.0),
                });
            }

            let manager = UvManager::new();
            
            // Ensure uv is available
            if manager.uv_path.is_none() {
                if let Some(ref cb) = on_progress {
                    cb(InstallProgress {
                        stage: "installing_uv".to_string(),
                        message: "Installing uv package manager...".to_string(),
                        progress: Some(0.1),
                    });
                }
                if let Err(e) = UvManager::install_uv().await {
                    if let Some(ref cb) = on_done {
                        cb(InstallResult {
                            success: false,
                            message: format!("Failed to install uv: {}", e),
                            python_version: None,
                            torch_version: None,
                        });
                    }
                    *lock.lock().unwrap() = false;
                    return;
                }
            }

            // Ensure venv exists
            if !manager.venv_exists() {
                if let Some(ref cb) = on_progress {
                    cb(InstallProgress {
                        stage: "creating_venv".to_string(),
                        message: "Creating Python virtual environment...".to_string(),
                        progress: Some(0.2),
                    });
                }
                if let Err(e) = manager.create_venv().await {
                    if let Some(ref cb) = on_done {
                        cb(InstallResult {
                            success: false,
                            message: format!("Failed to create venv: {}", e),
                            python_version: None,
                            torch_version: None,
                        });
                    }
                    *lock.lock().unwrap() = false;
                    return;
                }
            }

            // Install dependencies
            if let Some(ref cb) = on_progress {
                cb(InstallProgress {
                    stage: "installing_deps".to_string(),
                    message: "Installing Python dependencies...".to_string(),
                    progress: Some(0.4),
                });
            }

            let progress_cb = |msg: String| {
                if let Some(ref cb) = on_progress {
                    cb(InstallProgress {
                        stage: "installing".to_string(),
                        message: msg,
                        progress: Some(0.5),
                    });
                }
            };

            match manager.install_deps(progress_cb).await {
                Ok(()) => {
                    let torch_version = check_torch();
                    let python_version = check_python();
                    let success = torch_version.is_some();
                    if let Some(ref cb) = on_done {
                        cb(InstallResult {
                            success,
                            message: if success {
                                "Python environment setup completed with uv".to_string()
                            } else {
                                "Python environment setup completed but PyTorch not detected".to_string()
                            },
                            python_version,
                            torch_version,
                        });
                    }
                }
                Err(e) => {
                    if let Some(ref cb) = on_done {
                        cb(InstallResult {
                            success: false,
                            message: format!("Installation failed: {}", e),
                            python_version: None,
                            torch_version: None,
                        });
                    }
                }
            }
        });

        *lock.lock().unwrap() = false;
    });
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_pyproject_toml_current_dir() {
        // Should find pyproject.toml in the project root
        let result = UvManager::find_pyproject_toml();
        assert!(result.is_some(), "pyproject.toml should be found in project root");
        let path = result.unwrap();
        assert!(path.exists(), "Found pyproject.toml should exist");
        assert!(path.file_name().unwrap() == "pyproject.toml", "Should be named pyproject.toml");
    }

    #[test]
    fn test_resolve_python_path_candidates() {
        // This test just ensures the function doesn't panic
        // and returns either Some or None depending on system state
        let _result = resolve_python_path();
        // We don't assert on the result because Python may or may not be installed
    }

    #[test]
    fn test_check_conda_no_panic() {
        // Should not panic even if no conda env is active
        let (is_conda, is_mamba, env_name) = check_conda();
        // At most one of conda/mamba should be active
        assert!(!(is_conda && is_mamba), "Cannot be both conda and mamba active");
        if is_conda || is_mamba {
            assert!(env_name.is_some(), "Env name should be present if conda/mamba is active");
        }
    }

    #[test]
    fn test_mirror_source_pypi_url() {
        assert_eq!(
            MirrorSource::Default.pypi_url(),
            "https://pypi.org/simple"
        );
        assert_eq!(
            MirrorSource::Tsinghua.pypi_url(),
            "https://pypi.tuna.tsinghua.edu.cn/simple"
        );
        assert_eq!(
            MirrorSource::Aliyun.pypi_url(),
            "https://mirrors.aliyun.com/pypi/simple/"
        );
        assert_eq!(
            MirrorSource::USTC.pypi_url(),
            "https://pypi.mirrors.ustc.edu.cn/simple/"
        );
    }

    #[test]
    fn test_mirror_source_labels() {
        assert_eq!(MirrorSource::Default.label(), "官方源");
        assert_eq!(MirrorSource::Tsinghua.label(), "清华 TUNA");
        assert_eq!(MirrorSource::Aliyun.label(), "阿里云");
        assert_eq!(MirrorSource::USTC.label(), "中科大 USTC");
    }

    #[test]
    fn test_generate_install_plan_cpu_fallback() {
        let plan = UvManager::generate_install_plan(MirrorSource::Tsinghua);
        // Plan should have Python version specified
        assert!(!plan.python_version.is_empty(), "Python version should be specified");
        // Should contain torch
        assert!(
            plan.packages.iter().any(|p| p.contains("torch")),
            "Plan should include torch"
        );
        // Should contain onnxruntime
        assert!(
            plan.packages.iter().any(|p| p.contains("onnxruntime")),
            "Plan should include onnxruntime"
        );
        // Primary index should be set to mirror URL
        assert_eq!(plan.primary_index, MirrorSource::Tsinghua.pypi_url());
    }

    #[test]
    fn test_uv_manager_default_venv_path() {
        let path = UvManager::default_venv_path();
        assert!(
            path.to_string_lossy().contains(".rusttools") || path.to_string_lossy().contains("rusttools"),
            "Default venv path should contain rusttools: {:?}",
            path
        );
    }

    #[test]
    fn test_python_env_status_no_conda_fields() {
        let status = PythonEnvStatus {
            python_available: false,
            python_version: None,
            torch_available: false,
            torch_version: None,
            cuda_available: false,
            installing: false,
            detection_error: None,
        };
        // Verify the struct can be serialized without conda/mamba fields
        let json = serde_json::to_string(&status).unwrap();
        assert!(!json.contains("conda"), "Should not contain conda field");
        assert!(!json.contains("mamba"), "Should not contain mamba field");
    }
}
