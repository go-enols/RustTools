#![allow(dead_code)]

//! Rust环境检测模块
//! 
//! 用于检测训练和推理所需的Rust依赖环境
//! 由于使用纯Rust实现（Burn框架），不再需要Python环境

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustEnvInfo {
    pub burn_available: bool,
    pub cuda_available: bool,
    pub cuda_version: Option<String>,
    pub model_cache_dir: String,
    pub inference_backends: Vec<String>,
}

impl Default for RustEnvInfo {
    fn default() -> Self {
        Self {
            burn_available: true, // burn依赖已添加到Cargo.toml
            cuda_available: false,
            cuda_version: None,
            model_cache_dir: Self::default_cache_dir(),
            inference_backends: vec!["tract-onnx (CPU)".to_string()],
        }
    }
}

impl RustEnvInfo {
    /// 获取默认的模型缓存目录
    pub fn default_cache_dir() -> String {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|h| {
                PathBuf::from(h)
                    .join(".cache")
                    .join("rust-tools")
                    .join("models")
                    .to_string_lossy()
                    .to_string()
            })
            .unwrap_or_else(|_| ".cache/rust-tools/models".to_string())
    }
    
    /// 检测CUDA是否可用
    pub fn check_cuda() -> (bool, Option<String>) {
        // 检查NVIDIA驱动
        if cfg!(feature = "cuda") {
            // 如果编译时启用了cuda特性
            (true, Some("CUDA available (compiled)".to_string()))
        } else {
            // 尝试通过环境变量检测
            if std::env::var("CUDA_VISIBLE_DEVICES").is_ok() {
                return (true, Some("CUDA_VISIBLE_DEVICES set".to_string()));
            }
            (false, None)
        }
    }
    
    /// 检测可用的推理后端
    pub fn detect_inference_backends() -> Vec<String> {
        let mut backends = Vec::new();
        
        // tract-onnx 总是可用
        backends.push("tract-onnx (CPU)".to_string());
        
        // 检查CUDA
        let (cuda_ok, _) = Self::check_cuda();
        if cuda_ok {
            backends.push("tract-onnx (GPU)".to_string());
        }
        
        // 检查tch (如果可用)
        #[cfg(feature = "tch")]
        {
            if tch::Cuda::is_available() {
                backends.push("tch (CUDA)".to_string());
            }
        }
        
        backends
    }
    
    /// 创建完整的环境信息
    pub fn detect() -> Self {
        let (cuda_available, cuda_version) = Self::check_cuda();
        let inference_backends = Self::detect_inference_backends();
        
        Self {
            burn_available: true,
            cuda_available,
            cuda_version,
            model_cache_dir: Self::default_cache_dir(),
            inference_backends,
        }
    }
}

/// 检查Rust环境是否满足训练要求
pub fn check_training_env() -> Result<RustEnvInfo, String> {
    let env = RustEnvInfo::detect();
    
    eprintln!("[Env] Rust environment check:");
    eprintln!("[Env]   - Burn framework: {}", if env.burn_available { "✓ Available" } else { "✗ Not found" });
    eprintln!("[Env]   - CUDA: {}", if env.cuda_available { 
        format!("✓ Available ({})", env.cuda_version.clone().unwrap_or_default())
    } else { 
        "✗ Not available (CPU only)".to_string() 
    });
    eprintln!("[Env]   - Model cache: {}", env.model_cache_dir);
    eprintln!("[Env]   - Inference backends: {:?}", env.inference_backends);
    
    // Burn总是可用的，因为是编译时依赖
    Ok(env)
}

/// 获取简化的环境状态（用于前端显示）
pub fn get_env_status() -> String {
    let env = RustEnvInfo::detect();
    
    if env.cuda_available {
        format!("✓ Rust + CUDA (Burn + GPU acceleration)")
    } else {
        format!("✓ Rust (Burn CPU mode)")
    }
}
