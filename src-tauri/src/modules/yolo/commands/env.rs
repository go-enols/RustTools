use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonEnvInfo {
    pub python_exists: bool,
    pub python_version: Option<String>,
    pub torch_exists: bool,
    pub torch_version: Option<String>,
    pub torchaudio_exists: bool,
    pub cuda_available: bool,
    pub cuda_version: Option<String>,
    pub ultralytics_exists: bool,
    pub ultralytics_version: Option<String>,
    pub yolo_command_exists: bool,
}

#[derive(Debug, Serialize)]
pub struct EnvCheckResponse {
    pub success: bool,
    pub data: Option<PythonEnvInfo>,
    pub error: Option<String>,
}

/// Check Python environment status
#[tauri::command]
pub async fn check_python_env() -> Result<EnvCheckResponse, String> {
    let python_version = check_python_version();
    let torch_info = check_torch();
    let torchaudio_info = check_torchaudio();
    let cuda_info = check_cuda();
    let ultralytics_info = check_ultralytics();
    let yolo_exists = check_yolo_command();

    let env_info = PythonEnvInfo {
        python_exists: python_version.is_some(),
        python_version,
        torch_exists: torch_info.0,
        torch_version: torch_info.1,
        torchaudio_exists: torchaudio_info,
        cuda_available: cuda_info.0,
        cuda_version: cuda_info.1,
        ultralytics_exists: ultralytics_info.0,
        ultralytics_version: ultralytics_info.1,
        yolo_command_exists: yolo_exists,
    };

    Ok(EnvCheckResponse {
        success: true,
        data: Some(env_info),
        error: None,
    })
}

fn check_python_version() -> Option<String> {
    let output = Command::new("python")
        .arg("--version")
        .output()
        .or_else(|_| Command::new("python3").arg("--version").output())
        .ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(version)
    } else {
        None
    }
}

fn check_torch() -> (bool, Option<String>) {
    let output = Command::new("python")
        .args(["-c", "import torch; print(torch.__version__)"])
        .output()
        .or_else(|_| {
            Command::new("python3")
                .args(["-c", "import torch; print(torch.__version__)"])
                .output()
        });

    match output {
        Ok(o) if o.status.success() => {
            let version = String::from_utf8_lossy(&o.stdout).trim().to_string();
            (true, Some(version))
        }
        _ => (false, None),
    }
}

fn check_torchaudio() -> bool {
    let output = Command::new("python")
        .args(["-c", "import torchaudio; print(torchaudio.__version__)"])
        .output()
        .or_else(|_| {
            Command::new("python3")
                .args(["-c", "import torchaudio; print(torchaudio.__version__)"])
                .output()
        });

    output.map(|o| o.status.success()).unwrap_or(false)
}

fn check_cuda() -> (bool, Option<String>) {
    let output = Command::new("python")
        .args(["-c", "import torch; print(torch.cuda.is_available()); print(torch.version.cuda if torch.cuda.is_available() else '')"])
        .output()
        .or_else(|_| {
            Command::new("python3")
                .args(["-c", "import torch; print(torch.cuda.is_available()); print(torch.version.cuda if torch.cuda.is_available() else '')"])
                .output()
        });

    match output {
        Ok(o) if o.status.success() => {
            let output_str = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let lines: Vec<&str> = output_str.lines().collect();
            if lines.len() >= 2 {
                let available = lines[0].trim() == "True";
                let version = if available { Some(lines[1].trim().to_string()) } else { None };
                (available, version)
            } else {
                (false, None)
            }
        }
        _ => (false, None),
    }
}

fn check_ultralytics() -> (bool, Option<String>) {
    let output = Command::new("python")
        .args(["-c", "import ultralytics; print(ultralytics.__version__)"])
        .output()
        .or_else(|_| {
            Command::new("python3")
                .args(["-c", "import ultralytics; print(ultralytics.__version__)"])
                .output()
        });

    match output {
        Ok(o) if o.status.success() => {
            let version = String::from_utf8_lossy(&o.stdout).trim().to_string();
            (true, Some(version))
        }
        _ => (false, None),
    }
}

fn check_yolo_command() -> bool {
    let output = Command::new("yolo")
        .args(["--version"])
        .output()
        .or_else(|_| Command::new("yolo.exe").arg("--version").output())
        .ok();

    output.map(|o| o.status.success()).unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallRequest {
    pub use_mirror: bool,
}

#[derive(Debug, Serialize)]
pub struct InstallResponse {
    pub success: bool,
    pub message: String,
}

/// Install Python dependencies (torch, ultralytics)
#[tauri::command]
pub async fn install_python_deps(
    _app: tauri::AppHandle,
    use_mirror: bool,
    cpu_only: bool,
) -> Result<InstallResponse, String> {
    use std::process::Stdio;
    use tokio::process::Command;

    eprintln!("[Env] Starting Python dependency installation... (cpu_only={})", cpu_only);

    // Step 1: Upgrade pip
    eprintln!("[Env] Upgrading pip...");
    let pip_output = Command::new("python")
        .args(["-m", "pip", "install", "--upgrade", "pip"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run pip: {}", e))?;

    if !pip_output.status.success() {
        let stderr = String::from_utf8_lossy(&pip_output.stderr);
        eprintln!("[Env] pip upgrade failed: {}", stderr);
    }

    // Step 2: Install torch (CPU or GPU version)
    eprintln!("[Env] Installing PyTorch... (cpu_only={})", cpu_only);
    let torch_args: Vec<&str> = if cpu_only {
        // CPU-only version - no CUDA dependency issues
        vec![
            "-m", "pip", "install", "torch", "torchvision", "torchaudio",
            "--index-url", "https://download.pytorch.org/whl/cpu",
        ]
    } else {
        // GPU version - use CUDA 12.1 by default (most common)
        vec![
            "-m", "pip", "install", "torch", "torchvision", "torchaudio",
            "--index-url", "https://download.pytorch.org/whl/cu121",
        ]
    };

    let mut torch_child = Command::new("python").args(&torch_args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        .map_err(|e| format!("Failed to spawn torch install: {}", e))?;

    let torch_out = torch_child.wait().await
        .map_err(|e| format!("Failed to wait torch install: {}", e))?;

    eprintln!("[Env] PyTorch install exit code: {}", torch_out);

    // Step 3: Install ultralytics
    eprintln!("[Env] Installing ultralytics...");
    let ultra_args = if use_mirror {
        vec![
            "-m",
            "pip",
            "install",
            "ultralytics",
            "-i",
            "https://pypi.tuna.tsinghua.edu.cn/simple",
        ]
    } else {
        vec![
            "-m",
            "pip",
            "install",
            "ultralytics",
        ]
    };

    let mut ultra_child = Command::new("python").args(&ultra_args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        .map_err(|e| format!("Failed to spawn ultralytics install: {}", e))?;

    let ultra_out = ultra_child.wait().await
        .map_err(|e| format!("Failed to wait ultralytics install: {}", e))?;

    eprintln!("[Env] Ultralytics install exit code: {}", ultra_out);

    // Step 4: Install onnxruntime for inference (optional but useful)
    eprintln!("[Env] Installing onnxruntime...");
    let onnx_args = if use_mirror {
        vec![
            "-m",
            "pip",
            "install",
            "onnxruntime",
            "-i",
            "https://pypi.tuna.tsinghua.edu.cn/simple",
        ]
    } else {
        vec![
            "-m",
            "pip",
            "install",
            "onnxruntime",
        ]
    };

    let mut onnx_child = Command::new("python").args(&onnx_args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        .map_err(|e| format!("Failed to spawn onnxruntime install: {}", e))?;

    let onnx_out = onnx_child.wait().await
        .map_err(|e| format!("Failed to wait onnxruntime install: {}", e))?;

    eprintln!("[Env] onnxruntime install exit code: {}", onnx_out);

    // Verify installation
    eprintln!("[Env] Verifying installation...");
    let final_check = check_python_env().await?;

    if let Some(info) = final_check.data {
        if info.torch_exists && info.ultralytics_exists {
            // Build success message with CUDA warning
            let cuda_warning = if info.cuda_available {
                format!(" 检测到 CUDA {}，如遇兼容性问题请手动安装对应版本的 PyTorch", info.cuda_version.unwrap_or_default())
            } else {
                " 未检测到 CUDA，将使用 CPU 训练（可正常运行但速度较慢）".to_string()
            };

            Ok(InstallResponse {
                success: true,
                message: format!(
                    "安装成功! Python {}, PyTorch {}, Ultralytics {}{}",
                    info.python_version.unwrap_or_default(),
                    info.torch_version.unwrap_or_default(),
                    info.ultralytics_version.unwrap_or_default(),
                    cuda_warning
                ),
            })
        } else {
            let missing = if !info.torch_exists { "PyTorch " } else { "" }
                .to_string()
                + if !info.ultralytics_exists { "Ultralytics" } else { "" };

            Ok(InstallResponse {
                success: false,
                message: format!("部分安装成功，但以下组件未安装成功: {}", missing),
            })
        }
    } else {
        Ok(InstallResponse {
            success: false,
            message: "安装验证失败".to_string(),
        })
    }
}

/// Get installation instructions for manual install
#[tauri::command]
pub fn get_install_instructions() -> InstallInstructions {
    InstallInstructions {
        pip_install: vec![
            "python -m pip install --upgrade pip".to_string(),
        ],
        torch_install: vec![
            "pip install torch".to_string(),
            // China mirror
            "pip install torch -i https://pypi.tuna.tsinghua.edu.cn/simple".to_string(),
        ],
        torch_cpu_install: vec![
            // CPU-only version - avoids CUDA compatibility issues
            "pip install torch torchvision --index-url https://download.pytorch.org/whl/cpu".to_string(),
            // China mirror for CPU version
            "pip install torch torchvision -i https://pypi.tuna.tsinghua.edu.cn/simple --extra-index-url https://download.pytorch.org/whl/cpu".to_string(),
        ],
        ultralytics_install: vec![
            "pip install ultralytics".to_string(),
            // China mirror
            "pip install ultralytics -i https://pypi.tuna.tsinghua.edu.cn/simple".to_string(),
        ],
        manual_download: vec![
            "PyTorch: https://pytorch.org/get-started/locally/".to_string(),
            "Ultralytics: https://docs.ultralytics.com/".to_string(),
        ],
    }
}

#[derive(Debug, Serialize)]
pub struct InstallInstructions {
    pub pip_install: Vec<String>,
    pub torch_install: Vec<String>,
    pub torch_cpu_install: Vec<String>,
    pub ultralytics_install: Vec<String>,
    pub manual_download: Vec<String>,
}
