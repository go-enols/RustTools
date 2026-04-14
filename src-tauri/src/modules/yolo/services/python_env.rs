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
    pub pip_ok: bool,
    pub pip_version: Option<String>,
    pub ultralytics_available: bool,
    pub ultralytics_version: Option<String>,
    pub gpu_name: Option<String>,
    pub gpu_driver_version: Option<String>,
    pub ready_for_training: bool,
    pub installing: bool,
}

/// Global install lock to prevent concurrent installations
static INSTALL_LOCK: std::sync::OnceLock<Arc<Mutex<bool>>> = std::sync::OnceLock::new();

pub fn get_install_lock() -> Arc<Mutex<bool>> {
    INSTALL_LOCK
        .get_or_init(|| Arc::new(Mutex::new(false)))
        .clone()
}

/// Check if Python is available and get its version
pub fn check_python() -> Option<String> {
    let output = Command::new("python3")
        .arg("--version")
        .output()
        .or_else(|_| Command::new("python").arg("--version").output())
        .ok()?;
    
    if output.status.success() {
        let version_str = String::from_utf8_lossy(&output.stdout).to_string();
        let version = version_str.trim().replace("Python ", "");
        Some(version)
    } else {
        None
    }
}

/// Check if PyTorch is available and get its version
pub fn check_torch() -> Option<String> {
    let output = Command::new("python3")
        .args(["-c", "import torch; print(torch.__version__)"])
        .output()
        .or_else(|_| Command::new("python")
            .args(["-c", "import torch; print(torch.__version__)"])
            .output())
        .ok()?;
    
    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(version)
    } else {
        None
    }
}

/// Check if CUDA is available via PyTorch
pub fn check_cuda() -> bool {
    let output = Command::new("python3")
        .args(["-c", "import torch; print(torch.cuda.is_available())"])
        .output()
        .or_else(|_| {
            Command::new("python")
                .args(["-c", "import torch; print(torch.cuda.is_available())"])
                .output()
        })
        .ok();

    output
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().contains("True"))
        .unwrap_or(false)
}

/// Check if pip is available and get its version
pub fn check_pip() -> (bool, Option<String>) {
    let output = Command::new("python3")
        .args(["-m", "pip", "--version"])
        .output()
        .or_else(|_| Command::new("python").args(["-m", "pip", "--version"]).output())
        .ok();

    match output {
        Some(o) if o.status.success() => {
            let version_str = String::from_utf8_lossy(&o.stdout).trim().to_string();
            // Parse version from "pip 24.0 from ..." or "pip 24.0.2 from ..."
            let version = version_str
                .strip_prefix("pip ")
                .and_then(|s| s.split_whitespace().next())
                .map(|s| s.to_string());
            (true, version)
        }
        _ => (false, None),
    }
}

/// Check if ultralytics is available and get its version
pub fn check_ultralytics() -> (bool, Option<String>) {
    let output = Command::new("python3")
        .args(["-c", "import ultralytics; print(ultralytics.__version__)"])
        .output()
        .or_else(|_| {
            Command::new("python")
                .args(["-c", "import ultralytics; print(ultralytics.__version__)"])
                .output()
        })
        .ok();

    match output {
        Some(o) if o.status.success() => {
            let version = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if version.is_empty() {
                (true, None)
            } else {
                (true, Some(version))
            }
        }
        _ => (false, None),
    }
}

/// Check GPU info via nvidia-smi
pub fn check_gpu() -> (Option<String>, Option<String>) {
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=name,driver_version", "--format=csv,noheader,nounits"])
        .output()
        .ok();

    match output {
        Some(o) if o.status.success() => {
            let line = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 2 {
                (Some(parts[0].to_string()), Some(parts[1].to_string()))
            } else if parts.len() == 1 && !parts[0].is_empty() {
                (Some(parts[0].to_string()), None)
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    }
}

/// Get the full status of the Python environment
pub fn get_env_status() -> PythonEnvStatus {
    let python_version = check_python();
    let torch_version = check_torch();
    let cuda_available = check_cuda();
    let (pip_ok, pip_version) = check_pip();
    let (ultralytics_available, ultralytics_version) = check_ultralytics();
    let (gpu_name, gpu_driver_version) = check_gpu();
    let installing = *get_install_lock().lock().unwrap();

    // ready_for_training is true when ultralytics + torch are available (CUDA optional for CPU training)
    let ready_for_training = ultralytics_available && torch_version.is_some();

    PythonEnvStatus {
        python_available: python_version.is_some(),
        python_version,
        torch_available: torch_version.is_some(),
        torch_version,
        cuda_available,
        pip_ok,
        pip_version,
        ultralytics_available,
        ultralytics_version,
        gpu_name,
        gpu_driver_version,
        ready_for_training,
        installing,
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
        // Use catch_unwind to ensure lock is released even if thread panics
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Emit initial progress
            if let Some(ref cb) = on_progress {
                cb(InstallProgress {
                    stage: "starting".to_string(),
                    message: "Starting Python environment setup...".to_string(),
                    progress: Some(0.0),
                });
            }

            // Install torch with CUDA 12.4 support
            if let Some(ref cb) = on_progress {
                cb(InstallProgress {
                    stage: "installing_torch".to_string(),
                    message: "Installing PyTorch with CUDA 12.4 support...".to_string(),
                    progress: Some(0.2),
                });
            }

            let install_torch = Command::new("python3")
                .args([
                    "-m",
                    "pip",
                    "install",
                    "torch",
                    "--index-url",
                    "https://download.pytorch.org/whl/cu124",
                ])
                .status();

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

            let install_ultralytics = Command::new("python3")
                .args(["-m", "pip", "install", "ultralytics"])
                .status();

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
        }));

        // Release lock regardless of panic status
        *lock.lock().unwrap() = false;

        // Re-panic if caught a panic
        if let Err(e) = result {
            std::panic::panic_any(e);
        }
    });
}
