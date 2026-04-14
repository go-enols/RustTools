use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::{Arc, Mutex};

/// Progress event payload for Python environment installation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub stage: String,
    pub message: String,
    pub progress: Option<f32>,
}

/// Result event payload for Python environment installation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub success: bool,
    pub message: String,
    pub python_version: Option<String>,
    pub torch_version: Option<String>,
}

/// Status of the Python environment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
}

/// Global install lock to prevent concurrent installations
static INSTALL_LOCK: std::sync::OnceLock<Arc<Mutex<bool>>> = std::sync::OnceLock::new();

pub fn get_install_lock() -> Arc<Mutex<bool>> {
    INSTALL_LOCK
        .get_or_init(|| Arc::new(Mutex::new(false)))
        .clone()
}

/// List of python executables to try (in order of preference)
const PYTHON_CANDIDATES: &[&str] = &[
    "python3",
    "python",
    "/usr/bin/python3",
    "/usr/local/bin/python3",
];

/// Resolve the actual python path by trying multiple executables directly
pub fn resolve_python_path() -> Option<String> {
    for &python in PYTHON_CANDIDATES {
        if Command::new(python).arg("--version").output().ok()?.status.success() {
            return Some(python.to_string());
        }
    }
    None
}

/// Check if conda/mamba environment is active and return env info
pub fn check_conda() -> (bool, bool, Option<String>) {
    // Check MAMBA_DEFAULT_ENV first (mamba takes precedence)
    if let Ok(mamba_env) = std::env::var("MAMBA_DEFAULT_ENV") {
        if !mamba_env.is_empty() {
            return (false, true, Some(mamba_env));
        }
    }
    
    // Check CONDA_DEFAULT_ENV
    if let Ok(conda_env) = std::env::var("CONDA_DEFAULT_ENV") {
        if !conda_env.is_empty() {
            return (true, false, Some(conda_env));
        }
    }
    
    (false, false, None)
}

/// Check if Python is available and get its version
pub fn check_python() -> Option<String> {
    for &python in PYTHON_CANDIDATES {
        let output = Command::new(python)
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())?;
        
        let version_str = String::from_utf8_lossy(&output.stdout).to_string();
        let version = version_str.trim().replace("Python ", "");
        return Some(version);
    }
    None
}

/// Check if PyTorch is available and get its version
pub fn check_torch() -> Option<String> {
    for &python in PYTHON_CANDIDATES {
        let output = Command::new(python)
            .args(["-c", "import torch; print(torch.__version__)"])
            .output()
            .ok()
            .filter(|o| o.status.success())?;
        
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Some(version);
    }
    None
}

/// Check if CUDA is available via PyTorch
pub fn check_cuda() -> bool {
    for &python in PYTHON_CANDIDATES {
        let output = Command::new(python)
            .args(["-c", "import torch; print(torch.cuda.is_available())"])
            .output()
            .ok()
            .filter(|o| o.status.success());
        
        if let Some(o) = output {
            return String::from_utf8_lossy(&o.stdout).trim().contains("True");
        }
    }
    false
}

/// Get the full status of the Python environment
pub fn get_env_status() -> PythonEnvStatus {
    let python_version = check_python();
    let torch_version = check_torch();
    let installing = *get_install_lock().lock().unwrap();
    let (is_conda, is_mamba, conda_env_name) = check_conda();

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
    }
}

/// Progress callback type for installation events
pub type ProgressCallback = Box<dyn Fn(InstallProgress) + Send + Sync>;
/// Done callback type for installation completion
pub type DoneCallback = Box<dyn Fn(InstallResult) + Send + Sync>;

/// Install Python dependencies (torch, ultralytics) in a background thread
pub fn install_python_deps(
    on_progress: Option<ProgressCallback>,
    on_done: Option<DoneCallback>,
) {
    let lock = get_install_lock();
    
    // Check if already installing
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
        // Emit initial progress
        if let Some(ref cb) = on_progress {
            cb(InstallProgress {
                stage: "starting".to_string(),
                message: "Starting Python environment setup...".to_string(),
                progress: Some(0.0),
            });
        }

        // Determine pip install command based on environment
        let python_path = resolve_python_path().unwrap_or_else(|| "python3".to_string());
        let pip_cmd = if check_conda().0 || check_conda().1 {
            // In conda/mamba env, use python -m pip for proper environment targeting
            vec![python_path.as_str(), "-m", "pip"]
        } else {
            vec!["python3", "-m", "pip"]
        };

        // Install torch with CUDA 12.4 support
        if let Some(ref cb) = on_progress {
            cb(InstallProgress {
                stage: "installing_torch".to_string(),
                message: "Installing PyTorch with CUDA 12.4 support...".to_string(),
                progress: Some(0.2),
            });
        }

        let mut cmd = Command::new(&pip_cmd[0]);
        cmd.args(&pip_cmd[1..])
            .args([
                "install",
                "torch",
                "--index-url",
                "https://download.pytorch.org/whl/cu124",
            ]);
        let install_torch = cmd.status();

        match install_torch {
            Ok(status) if status.success() => {
                if let Some(ref cb) = on_progress {
                    cb(InstallProgress {
                        stage: "torch_installed".to_string(),
                        message: "PyTorch installed successfully".to_string(),
                        progress: Some(0.6),
                    });
                }
            }
            _ => {
                if let Some(ref cb) = on_progress {
                    cb(InstallProgress {
                        stage: "torch_install_failed".to_string(),
                        message: "Failed to install PyTorch, continuing...".to_string(),
                        progress: Some(0.6),
                    });
                }
            }
        }

        // Install ultralytics (YOLO)
        if let Some(ref cb) = on_progress {
            cb(InstallProgress {
                stage: "installing_ultralytics".to_string(),
                message: "Installing ultralytics (YOLO)...".to_string(),
                progress: Some(0.7),
            });
        }

        let mut cmd_ultralytics = Command::new(&pip_cmd[0]);
        cmd_ultralytics.args(&pip_cmd[1..]).arg("install");
        let install_ultralytics = cmd_ultralytics.arg("ultralytics").status();

        let ultralytics_ok = match install_ultralytics {
            Ok(status) => status.success(),
            _ => false,
        };

        if ultralytics_ok {
            if let Some(ref cb) = on_progress {
                cb(InstallProgress {
                    stage: "ultralytics_installed".to_string(),
                    message: "Ultralytics installed successfully".to_string(),
                    progress: Some(0.9),
                });
            }
        }

        // Get versions
        let torch_version = check_torch();
        let python_version = check_python();

        // Emit completion
        let success = torch_version.is_some();
        if let Some(ref cb) = on_done {
            cb(InstallResult {
                success,
                message: if success {
                    "Python environment setup completed".to_string()
                } else {
                    "Python environment setup completed with errors".to_string()
                },
                python_version,
                torch_version,
            });
        }

        // Release lock
        *lock.lock().unwrap() = false;
    });
}
