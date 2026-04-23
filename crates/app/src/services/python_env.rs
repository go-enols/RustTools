use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};

/// Progress event payload for Python environment installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    pub stage: String,
    pub message: String,
    pub progress: Option<f32>,
}

/// Result event payload for Python environment installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub message: String,
    pub python_version: Option<String>,
    pub torch_version: Option<String>,
}

/// Status of the Python environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonEnvStatus {
    pub python_available: bool,
    pub python_version: Option<String>,
    pub torch_available: bool,
    pub torch_version: Option<String>,
    pub cuda_available: bool,
    pub installing: bool,
    pub is_conda: bool,
    pub is_mamba: bool,
    pub conda_env_name: Option<String>,
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

    /// Install dependencies from pyproject.toml with progress reporting
    pub async fn install_deps(
        &self,
        on_progress: impl Fn(String),
    ) -> Result<(), String> {
        let uv = self.uv_path.as_ref().ok_or("uv not found")?;
        let pyproject = Self::find_pyproject_toml()
            .ok_or("pyproject.toml not found. Please ensure it exists in the application directory.".to_string())?;
        
        on_progress(format!("Using pyproject.toml at: {}", pyproject.display()));

        on_progress("Detecting CUDA availability...".to_string());
        let has_cuda = Self::check_nvidia_gpu();
        
        let torch_index = if has_cuda {
            "https://download.pytorch.org/whl/cu124"
        } else {
            "https://download.pytorch.org/whl/cpu"
        };

        on_progress(format!("Installing dependencies (PyTorch from {})...", torch_index));

        // Build uv pip install command
        let mut cmd = tokio::process::Command::new(uv);
        cmd.arg("pip")
            .arg("install")
            .arg("--python")
            .arg(&self.python_path)
            .arg("-r")
            .arg(&pyproject)
            .arg("--extra-index-url")
            .arg(torch_index)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn uv pip install: {}", e))?;

        // Stream stdout for progress
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    on_progress(line);
                }
            }
        }

        let output = child.wait_with_output().await
            .map_err(|e| format!("Failed to wait for uv pip install: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Dependency installation failed: {}", stderr));
        }

        on_progress("Installation complete".to_string());
        Ok(())
    }

    /// Check if NVIDIA GPU is available
    fn check_nvidia_gpu() -> bool {
        Command::new("nvidia-smi")
            .arg("-L")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
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

/// Cache for the resolved python path
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

/// Get the full status of the Python environment
pub fn get_env_status() -> PythonEnvStatus {
    let python_version = check_python();
    let torch_version = check_torch();
    let installing = *get_install_lock().lock().unwrap();
    let (is_conda, is_mamba, conda_env_name) = check_conda();

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
        is_conda,
        is_mamba,
        conda_env_name,
        detection_error,
    }
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
}
