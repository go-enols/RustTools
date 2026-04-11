use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelFormat {
    PyTorch,      // .pt
    ONNX,         // .onnx
    Safetensors,  // .safetensors
    CandleModel,  // candle模型
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    pub success: bool,
    pub input_path: String,
    pub output_path: Option<String>,
    pub format: ModelFormat,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub format: ModelFormat,
    pub path: String,
    pub file_size: String,
    pub is_compatible: bool,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityResult {
    pub is_compatible: bool,
    pub format: ModelFormat,
    pub message: String,
    pub conversion_hint: Option<String>,
}

pub fn resolve_inference_model_path(path: &str) -> Result<PathBuf, String> {
    let path_obj = Path::new(path);

    if !path_obj.exists() {
        return Err(format!("文件不存在: {}", path));
    }

    match detect_model_format(path) {
        ModelFormat::ONNX => Ok(path_obj.to_path_buf()),
        ModelFormat::PyTorch => ensure_cached_onnx(path_obj),
        ModelFormat::Safetensors => Err("暂不支持直接加载 Safetensors 模型，请先转换为 ONNX".to_string()),
        ModelFormat::CandleModel => Err("暂不支持直接加载 Candle 模型，请先转换为 ONNX".to_string()),
        ModelFormat::Unknown => Err("未知模型格式，仅支持 .onnx 或可自动转换的 .pt/.pth".to_string()),
    }
}

fn ensure_cached_onnx(path: &Path) -> Result<PathBuf, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("读取模型文件失败: {}", e))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("model");
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("rust-tools")
        .join("onnx-cache");

    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("创建模型缓存目录失败: {}", e))?;

    let output_path = cache_dir.join(format!("{}_{}_{}.onnx", stem, metadata.len(), modified));

    if output_path.exists() {
        return Ok(output_path);
    }

    export_pytorch_to_onnx(path, &output_path)?;

    if !output_path.exists() {
        return Err(format!("自动转换完成后未找到 ONNX 文件: {}", output_path.display()));
    }

    Ok(output_path)
}

fn export_pytorch_to_onnx(input_path: &Path, output_path: &Path) -> Result<(), String> {
    let python = find_python_command()?;
    let input_arg = input_path.to_string_lossy().into_owned();
    let output_arg = output_path.to_string_lossy().into_owned();
    let script = r#"
import shutil
import sys
from pathlib import Path
from ultralytics import YOLO

source = Path(sys.argv[1])
target = Path(sys.argv[2])
target.parent.mkdir(parents=True, exist_ok=True)

exported = YOLO(str(source)).export(
    format="onnx",
    imgsz=640,
    device="cpu",
    simplify=False,
    dynamic=False
)

exported_path = Path(str(exported))
if not exported_path.exists():
    raise FileNotFoundError(f"导出后未找到文件: {exported_path}")

if exported_path.resolve() != target.resolve():
    shutil.copy2(exported_path, target)

print(target)
"#;

    let output = Command::new(&python)
        .args(["-c", script, input_arg.as_str(), output_arg.as_str()])
        .output()
        .map_err(|e| format!("启动 Python 自动转换失败: {}", e))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let details = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        "无详细输出".to_string()
    };

    Err(format!(
        "自动转换 PyTorch 模型失败，请确认已安装 Python、torch、ultralytics。\n{}",
        details
    ))
}

fn find_python_command() -> Result<String, String> {
    for command in ["python", "python3"] {
        let output = Command::new(command)
            .args(["-c", "import sys; print(sys.executable)"])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                return Ok(command.to_string());
            }
        }
    }

    Err("未找到可用的 Python，请先安装 Python 并确保已加入 PATH".to_string())
}

fn check_python_export_support() -> Result<(), String> {
    let python = find_python_command()?;
    let output = Command::new(python)
        .args(["-c", "import torch; import ultralytics; print('ok')"])
        .output()
        .map_err(|e| format!("检查 Python 推理环境失败: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let details = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "缺少 torch 或 ultralytics".to_string()
        };
        Err(details)
    }
}

/// 检测模型格式
pub fn detect_model_format(path: &str) -> ModelFormat {
    let path = Path::new(path);
    
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|value| value.to_ascii_lowercase());

    match extension.as_deref() {
        Some("pt") | Some("pth") => ModelFormat::PyTorch,
        Some("onnx") => ModelFormat::ONNX,
        Some("safetensors") | Some("safetensor") => ModelFormat::Safetensors,
        Some("bin") | Some("ot") => ModelFormat::CandleModel,
        _ => ModelFormat::Unknown,
    }
}

/// 获取模型文件大小
fn get_file_size_string(path: &Path) -> String {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            let size = metadata.len();
            if size > 1024 * 1024 * 1024 {
                format!("{:.2} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
            } else if size > 1024 * 1024 {
                format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
            } else {
                format!("{:.2} KB", size as f64 / 1024.0)
            }
        }
        Err(_) => "Unknown".to_string(),
    }
}

/// 检查模型兼容性
pub fn is_model_compatible(path: &str) -> CompatibilityResult {
    let format = detect_model_format(path);
    let path_obj = Path::new(path);
    
    // 检查文件是否存在
    if !path_obj.exists() {
        return CompatibilityResult {
            is_compatible: false,
            format,
            message: format!("文件不存在: {}", path),
            conversion_hint: None,
        };
    }
    
    let file_size = get_file_size_string(path_obj);
    
    match format {
        ModelFormat::ONNX => {
            // 尝试验证ONNX模型
            use tract_onnx::prelude::Framework;
            
            match tract_onnx::onnx().model_for_path(path) {
                Ok(model) => {
                    // 获取输入信息
                    let input_info = model.input_fact(0)
                        .map(|f| format!("{:?}", f))
                        .unwrap_or_else(|_| "Unknown".to_string());
                    
                    CompatibilityResult {
                        is_compatible: true,
                        format,
                        message: format!("✓ ONNX模型格式正确，可以使用\n文件大小: {}\n输入形状: {}", 
                            file_size,
                            input_info.split_whitespace().take(5).collect::<String>()),
                        conversion_hint: None,
                    }
                }
                Err(e) => {
                    CompatibilityResult {
                        is_compatible: false,
                        format,
                        message: format!("✗ ONNX文件已损坏或格式不正确\n文件大小: {}\n错误: {}", 
                            file_size,
                            e.to_string()),
                        conversion_hint: Some("请重新导出模型: yolo export model=your_model.pt format=onnx".to_string()),
                    }
                }
            }
        }
        ModelFormat::PyTorch => {
            match check_python_export_support() {
                Ok(()) => CompatibilityResult {
                    is_compatible: true,
                    format,
                    message: format!(
                        "✓ PyTorch 模型可用于推理\n文件大小: {}\n首次使用时会自动转换为 ONNX 并缓存",
                        file_size
                    ),
                    conversion_hint: Some("支持常见 YOLO .pt/.pth 模型；首次加载会稍慢，后续直接复用缓存".to_string()),
                },
                Err(reason) => CompatibilityResult {
                    is_compatible: false,
                    format,
                    message: format!(
                        "✗ PyTorch 模型当前无法自动转换\n文件大小: {}\n原因: {}",
                        file_size, reason
                    ),
                    conversion_hint: Some("请先安装 Python、torch、ultralytics，或手动转换为 ONNX".to_string()),
                },
            }
        }
        ModelFormat::Safetensors => {
            CompatibilityResult {
                is_compatible: false,
                format,
                message: format!("✗ Safetensors格式暂未支持\n文件大小: {}", file_size),
                conversion_hint: Some("请转换为ONNX格式".to_string()),
            }
        }
        ModelFormat::CandleModel => {
            CompatibilityResult {
                is_compatible: false,
                format,
                message: format!("✗ Candle模型格式暂未支持\n文件大小: {}", file_size),
                conversion_hint: Some("请转换为ONNX格式".to_string()),
            }
        }
        ModelFormat::Unknown => {
            CompatibilityResult {
                is_compatible: false,
                format,
                message: format!("✗ 未知模型格式\n文件大小: {}\n支持的格式: .onnx (推荐), .pt (需转换)", file_size),
                conversion_hint: Some("请使用ONNX格式的模型".to_string()),
            }
        }
    }
}

/// 获取模型详细信息
pub fn get_model_info(path: &str) -> ModelInfo {
    let format = detect_model_format(path);
    let path_obj = Path::new(path);
    
    let file_size = if path_obj.exists() {
        get_file_size_string(path_obj)
    } else {
        "Unknown".to_string()
    };
    
    let file_size_clone = file_size.clone();
    let format_clone = format.clone();
    
    match format {
        ModelFormat::ONNX => {
            use tract_onnx::prelude::Framework;
            
            match tract_onnx::onnx().model_for_path(path) {
                Ok(model) => {
                    // 获取输入输出信息
                    let input_info = model.input_fact(0)
                        .map(|f| format!("{:?}", f))
                        .unwrap_or_else(|_| "Unknown".to_string());
                    
                    let output_info = model.output_fact(0)
                        .map(|f| format!("{:?}", f))
                        .unwrap_or_else(|_| "Unknown".to_string());
                    
                    ModelInfo {
                        format,
                        path: path.to_string(),
                        file_size,
                        is_compatible: true,
                        details: format!(
                            "模型格式: ONNX\n\
                            文件大小: {}\n\
                            输入形状: {}\n\
                            输出形状: {}",
                            file_size_clone,
                            input_info.split_whitespace().take(5).collect::<String>(),
                            output_info.split_whitespace().take(5).collect::<String>()
                        ),
                    }
                }
                Err(e) => ModelInfo {
                    format,
                    path: path.to_string(),
                    file_size,
                    is_compatible: false,
                    details: format!(
                        "模型格式: ONNX (但解析失败)\n\
                        文件大小: {}\n\
                        错误: {}\n\
                        \n\
                        建议: 重新导出模型",
                        file_size_clone,
                        e.to_string()
                    ),
                },
            }
        }
        ModelFormat::PyTorch => {
            match check_python_export_support() {
                Ok(()) => ModelInfo {
                    format,
                    path: path.to_string(),
                    file_size,
                    is_compatible: true,
                    details: format!(
                        "模型格式: PyTorch (.pt/.pth)\n\
                        文件大小: {}\n\
                        \n\
                        ✓ 可用于推理\n\
                        首次加载时会自动导出为 ONNX 并写入缓存，之后直接复用缓存文件",
                        file_size_clone
                    ),
                },
                Err(reason) => ModelInfo {
                    format,
                    path: path.to_string(),
                    file_size,
                    is_compatible: false,
                    details: format!(
                        "模型格式: PyTorch (.pt/.pth)\n\
                        文件大小: {}\n\
                        \n\
                        ✗ 当前无法自动转换\n\
                        原因: {}\n\
                        \n\
                        请先安装 Python、torch、ultralytics，或手动转换为 ONNX",
                        file_size_clone,
                        reason
                    ),
                },
            }
        }
        _ => {
            ModelInfo {
                format,
                path: path.to_string(),
                file_size,
                is_compatible: false,
                details: format!(
                    "模型格式: {:?}\n\
                    文件大小: {}\n\
                    \n\
                    ⚠ 此格式暂不支持\n\
                    请转换为ONNX格式",
                    format_clone,
                    file_size_clone
                ),
            }
        }
    }
}

/// 格式化模型转换提示
pub fn format_conversion_instructions(format: &ModelFormat) -> String {
    match format {
        ModelFormat::PyTorch => {
            r#"📦 PyTorch模型转换指南

方法1: 使用YOLO命令 (推荐)
  yolo export model=your_model.pt format=onnx

方法2: 使用Python脚本
  python scripts/convert_model.py --input model.pt --output model.onnx

方法3: 手动转换
  import torch
  import torch.onnx
  
  # 加载模型
  model = torch.load('model.pt', map_location='cpu')
  model.eval()
  
  # 创建示例输入
  dummy_input = torch.randn(1, 3, 640, 640)
  
  # 导出为ONNX
  torch.onnx.export(
      model,
      dummy_input,
      'model.onnx',
      export_params=True,
      opset_version=11,
      do_constant_folding=True,
      input_names=['input'],
      output_names=['output'],
      dynamic_axes={
          'input': {0: 'batch_size'},
          'output': {0: 'batch_size'}
      }
  )
  
注意事项:
  - 确保PyTorch版本 >= 1.8
  - ONNX opset版本建议使用11或更高
  - 转换后验证模型是否正常工作"#.to_string()
        }
        _ => "此格式无需转换".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format() {
        assert_eq!(detect_model_format("model.onnx"), ModelFormat::ONNX);
        assert_eq!(detect_model_format("model.pt"), ModelFormat::PyTorch);
        assert_eq!(detect_model_format("model.pth"), ModelFormat::PyTorch);
        assert_eq!(detect_model_format("model.safetensors"), ModelFormat::Safetensors);
        assert_eq!(detect_model_format("model.bin"), ModelFormat::CandleModel);
        assert_eq!(detect_model_format("model.unknown"), ModelFormat::Unknown);
    }
}
