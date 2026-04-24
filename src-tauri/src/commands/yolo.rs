//! YOLO / Training / Inference / Capture 命令模块
//!
//! 提供所有与模型训练、推理、标注、桌面捕获相关的 Tauri 命令。

use crate::{AppState, CaptureState, OnnxDetection};
use rusttools_app::models::YoloAnnotation;
use tauri::State;

// ============================================================================
// Training Commands
// ============================================================================

#[tauri::command]
pub async fn start_training(
    config: rusttools_app::models::TrainingRequest,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let project_path = state
        .current_project
        .lock()
        .unwrap()
        .as_ref()
        .map(|p| p.path.clone())
        .ok_or("No project is currently open".to_string())?;

    let training_id = state
        .trainer_service
        .start_training(project_path, config)
        .await?;

    Ok(training_id)
}

#[tauri::command]
pub async fn stop_training(
    training_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.trainer_service.stop_training(&training_id).await
}

#[tauri::command]
pub async fn get_training_status(
    training_id: String,
    state: State<'_, AppState>,
) -> Result<rusttools_app::models::TrainingStatus, String> {
    let status = state
        .trainer_service
        .get_status(&training_id)
        .await
        .unwrap_or_default();
    Ok(status)
}

#[tauri::command]
pub async fn list_training_logs(
    training_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    Ok(state.trainer_service.get_logs(&training_id).await)
}

fn format_unix_timestamp(ts: u64) -> String {
    let days_since_epoch = ts / 86400;
    let seconds_of_day = ts % 86400;
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let mut year = 1970u64;
    let mut days = days_since_epoch;
    loop {
        let days_in_year = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for (i, dim) in days_in_month.iter().enumerate() {
        let dim = if i == 1 && ((year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)) { 29 } else { *dim };
        if days < dim {
            month = (i + 1) as u64;
            break;
        }
        days -= dim;
        month = (i + 2) as u64;
    }
    let day = days + 1;
    format!("{:04}-{:02}-{:02} {:02}:{:02}", year, month, day, hour, minute)
}

#[derive(serde::Serialize)]
pub struct TrainingResult {
    pub name: String,
    pub path: String,
    pub has_best: bool,
    pub has_last: bool,
    pub epochs_completed: u32,
    pub map50: f32,
    pub map50_95: f32,
    pub created_at: String,
}

#[tauri::command]
pub fn list_training_results(project_path: String) -> Result<Vec<TrainingResult>, String> {
    let runs_dir = std::path::Path::new(&project_path).join("runs");
    if !runs_dir.exists() {
        return Ok(vec![]);
    }

    let mut results = Vec::new();
    for entry in std::fs::read_dir(&runs_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        // Ultralytics creates directories like train/, train2/, train3/ ...
        if !name.starts_with("train") {
            continue;
        }

        let weights_dir = path.join("weights");
        let has_best = weights_dir.join("best.pt").exists();
        let has_last = weights_dir.join("last.pt").exists();

        let mut epochs_completed = 0u32;
        let mut map50 = 0.0f32;
        let mut map50_95 = 0.0f32;

        let results_csv = path.join("results.csv");
        if results_csv.exists() {
            if let Ok(content) = std::fs::read_to_string(&results_csv) {
                let lines: Vec<&str> = content.lines().collect();
                if lines.len() >= 2 {
                    // Skip empty lines at the end of CSV
                    let last_line = lines.iter().rev().find(|l| !l.trim().is_empty()).copied().unwrap_or("");
                    let parts: Vec<&str> = last_line.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 9 {
                        epochs_completed = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
                        map50 = parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        map50_95 = parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    }
                }
            }
        }

        let created_at = entry.metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| {
                let secs = d.as_secs();
                format_unix_timestamp(secs)
            })
            .unwrap_or_default();

        results.push(TrainingResult {
            name,
            path: path.to_string_lossy().to_string(),
            has_best,
            has_last,
            epochs_completed,
            map50,
            map50_95,
            created_at,
        });
    }

    results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(results)
}

// ============================================================================
// Inference Commands
// ============================================================================

#[tauri::command]
pub fn load_model(
    model_path: String,
    state: State<AppState>,
) -> Result<(), String> {
    let engine = rusttools_app::services::yolo_onnx::YoloOnnxEngine::new(&model_path)?;
    *state.yolo_engine.lock().unwrap() = Some(engine);
    Ok(())
}

/// 若 model_path 是 .pt，尝试返回同目录同名 .onnx 的路径
fn resolve_onnx_path(model_path: &str) -> Result<std::path::PathBuf, String> {
    let path = std::path::Path::new(model_path);
    if model_path.to_lowercase().ends_with(".pt") {
        let onnx = path.with_extension("onnx");
        if onnx.exists() {
            Ok(onnx)
        } else {
            Err(format!(
                "PT 模型尚未转换为 ONNX：{}. 请先调用 export_pt_to_onnx 进行转换。",
                onnx.display()
            ))
        }
    } else {
        Ok(path.to_path_buf())
    }
}

/// 检查 .pt 模型是否已有对应的 .onnx 文件
#[tauri::command]
pub fn check_onnx_for_pt(model_path: String) -> Result<bool, String> {
    let path = std::path::Path::new(&model_path);
    if model_path.to_lowercase().ends_with(".pt") {
        Ok(path.with_extension("onnx").exists())
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub fn run_inference_image(
    model_path: String,
    image_path: String,
    conf_threshold: Option<f32>,
    _state: State<AppState>,
) -> Result<Vec<OnnxDetection>, String> {
    let onnx_path = resolve_onnx_path(&model_path)?;
    let threshold = conf_threshold.unwrap_or(0.25).clamp(0.01, 1.0);

    let mut engine = rusttools_app::services::yolo_onnx::YoloOnnxEngine::new(
        onnx_path.to_str().unwrap_or(&model_path)
    )?;
    engine.set_conf_threshold(threshold);
    let image = image::open(&image_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;
    let detections = engine.infer(&image)?;
    Ok(detections.into_iter().map(|d| OnnxDetection {
        class_id: d.class_id,
        confidence: d.confidence,
        x1: d.x1,
        y1: d.y1,
        x2: d.x2,
        y2: d.y2,
    }).collect())
}

#[tauri::command]
pub fn unload_model(state: State<AppState>) -> Result<(), String> {
    *state.yolo_engine.lock().unwrap() = None;
    Ok(())
}

// ============================================================================
// Auto Annotation Command
// ============================================================================

#[derive(serde::Deserialize)]
struct PythonDetection {
    class_id: usize,
    #[allow(dead_code)]
    confidence: f32,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

fn find_inference_script() -> Result<std::path::PathBuf, String> {
    let candidates = [
        std::path::PathBuf::from("python/scripts/yolo_inference.py"),
        std::path::PathBuf::from("../python/scripts/yolo_inference.py"),
        std::path::PathBuf::from("../../python/scripts/yolo_inference.py"),
    ];

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for rel in [
                "python/scripts/yolo_inference.py",
                "../python/scripts/yolo_inference.py",
                "../../python/scripts/yolo_inference.py",
            ] {
                let p = dir.join(rel);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }

    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }

    Err("Inference script not found: python/scripts/yolo_inference.py".to_string())
}

fn find_export_script() -> Result<std::path::PathBuf, String> {
    let candidates = [
        std::path::PathBuf::from("python/scripts/export_pt_to_onnx.py"),
        std::path::PathBuf::from("../python/scripts/export_pt_to_onnx.py"),
        std::path::PathBuf::from("../../python/scripts/export_pt_to_onnx.py"),
    ];

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for rel in [
                "python/scripts/export_pt_to_onnx.py",
                "../python/scripts/export_pt_to_onnx.py",
                "../../python/scripts/export_pt_to_onnx.py",
            ] {
                let p = dir.join(rel);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }

    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }

    Err("Export script not found: python/scripts/export_pt_to_onnx.py".to_string())
}

/// 将 .pt 模型导出为 ONNX（一次性转换，后续推理纯 Rust）
#[tauri::command]
pub fn export_pt_to_onnx(model_path: String) -> Result<String, String> {
    let python = rusttools_app::services::python_env::resolved_python()
        .ok_or("Python environment not available. Please install Python dependencies first.")?;
    let script = find_export_script()?;

    let output = std::process::Command::new(&python)
        .arg(&script)
        .arg(&model_path)
        .output()
        .map_err(|e| format!("Failed to run export process: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Export failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    let onnx_path = lines.last().unwrap_or(&"").trim();
    if onnx_path.is_empty() {
        return Err("Export produced empty output".to_string());
    }

    Ok(onnx_path.to_string())
}

#[tauri::command]
pub fn auto_annotate_image(
    model_path: String,
    image_path: String,
    conf_threshold: Option<f32>,
) -> Result<Vec<YoloAnnotation>, String> {
    let threshold = conf_threshold.unwrap_or(0.25).clamp(0.01, 1.0);

    // Try ONNX fast path first
    if model_path.to_lowercase().ends_with(".onnx") {
        if let Ok(mut engine) = rusttools_app::services::yolo_onnx::YoloOnnxEngine::new(&model_path) {
            engine.set_conf_threshold(threshold);
            let image = image::open(&image_path)
                .map_err(|e| format!("Failed to open image: {}", e))?;
            let detections = engine.infer(&image)
                .map_err(|e| format!("ONNX inference failed: {}", e))?;
            let img_w = image.width() as f64;
            let img_h = image.height() as f64;

            return Ok(detections.into_iter().map(|det| {
                let x_center = ((det.x1 + det.x2) as f64 / 2.0) / img_w;
                let y_center = ((det.y1 + det.y2) as f64 / 2.0) / img_h;
                let width = ((det.x2 - det.x1).abs() as f64) / img_w;
                let height = ((det.y2 - det.y1).abs() as f64) / img_h;
                YoloAnnotation {
                    class_id: det.class_id,
                    x_center: x_center.clamp(0.0, 1.0),
                    y_center: y_center.clamp(0.0, 1.0),
                    width: width.clamp(0.0, 1.0),
                    height: height.clamp(0.0, 1.0),
                }
            }).collect());
        }
    }

    // Fallback to Python (supports .pt and .onnx via ultralytics)
    let python = rusttools_app::services::python_env::resolved_python()
        .ok_or("Python environment not available. Please install Python dependencies first.")?;

    let script = find_inference_script()?;

    let output = std::process::Command::new(&python)
        .arg(&script)
        .arg(&model_path)
        .arg(&image_path)
        .arg(threshold.to_string())
        .output()
        .map_err(|e| format!("Failed to run inference process: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Inference failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    let json_line = lines.last().unwrap_or(&"").trim();
    if json_line.is_empty() {
        return Ok(vec![]);
    }

    let dets: Vec<PythonDetection> = serde_json::from_str(json_line)
        .map_err(|e| format!("Failed to parse detections: {} (output: {})", e, stdout))?;

    let image = image::open(&image_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;
    let img_w = image.width() as f64;
    let img_h = image.height() as f64;

    Ok(dets.into_iter().map(|det| {
        let x_center = ((det.x1 + det.x2) / 2.0) / img_w;
        let y_center = ((det.y1 + det.y2) / 2.0) / img_h;
        let width = (det.x2 - det.x1).abs() / img_w;
        let height = (det.y2 - det.y1).abs() / img_h;
        YoloAnnotation {
            class_id: det.class_id,
            x_center: x_center.clamp(0.0, 1.0),
            y_center: y_center.clamp(0.0, 1.0),
            width: width.clamp(0.0, 1.0),
            height: height.clamp(0.0, 1.0),
        }
    }).collect())
}

#[derive(serde::Serialize)]
pub struct ModelInfo {
    name: String,
    path: String,
    size: u64,
}

#[tauri::command]
pub fn list_models(dir: Option<String>, ext: Option<String>) -> Result<Vec<ModelInfo>, String> {
    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();
    // Support comma-separated extensions, e.g. "onnx,pt" or single "onnx"
    let target_exts: Vec<String> = ext
        .unwrap_or_else(|| "onnx".to_string())
        .to_lowercase()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut scan_dir = |path: &std::path::Path| {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Some(file_ext) = p.extension() {
                        let ext_lower = file_ext.to_string_lossy().to_lowercase();
                        if target_exts.iter().any(|te| te == &ext_lower) {
                            let name = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
                            let path_str = p.to_string_lossy().to_string();
                            if seen.insert(path_str.clone()) {
                                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                                results.push(ModelInfo { name, path: path_str, size });
                            }
                        }
                    }
                }
            }
        }
    };

    if let Some(d) = dir {
        scan_dir(std::path::Path::new(&d));
    } else {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                scan_dir(parent);
                scan_dir(&parent.join("models"));
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            scan_dir(&cwd);
            scan_dir(&cwd.join("models"));
        }
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(results)
}

// ============================================================================
// Video Inference Commands
// ============================================================================

#[tauri::command]
pub fn extract_video_frame(video_path: String, timestamp_sec: f64) -> Result<String, String> {
    let output_dir = std::env::temp_dir().join("rusttools_video_frames");
    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    let stem = std::path::Path::new(&video_path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let output_path = output_dir.join(format!("{}_{:.3}.jpg", stem, timestamp_sec));

    let output = std::process::Command::new("ffmpeg")
        .args(&[
            "-y",
            "-ss",
            &format!("{:.3}", timestamp_sec),
            "-i",
            &video_path,
            "-vframes",
            "1",
            "-q:v",
            "2",
            &output_path.to_string_lossy(),
        ])
        .output()
        .map_err(|e| format!("ffmpeg failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg error: {}", stderr));
    }

    Ok(output_path.to_string_lossy().to_string())
}

// ============================================================================
// Annotation Commands
// ============================================================================

#[tauri::command]
pub fn list_images(folder: String, recursive: Option<bool>) -> Result<Vec<String>, String> {
    let path = std::path::Path::new(&folder);
    if !path.is_dir() {
        return Ok(vec![]);
    }
    let mut images = vec![];
    let recursive = recursive.unwrap_or(true);

    fn scan_dir(dir: &std::path::Path, images: &mut Vec<String>) -> Result<(), String> {
        let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(ext) = p.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "bmp" | "webp") {
                        images.push(p.to_string_lossy().to_string());
                    }
                }
            } else if p.is_dir() {
                scan_dir(&p, images)?;
            }
        }
        Ok(())
    }

    if recursive {
        scan_dir(path, &mut images)?;
    } else {
        let entries = std::fs::read_dir(path).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(ext) = p.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "bmp" | "webp") {
                        images.push(p.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    images.sort();
    Ok(images)
}

fn resolve_label_path(img_path: &std::path::Path) -> Option<std::path::PathBuf> {
    let stem = img_path.file_stem()?.to_string_lossy();
    let parent = img_path.parent()?;
    let parent_name = parent.file_name()?.to_string_lossy();

    let grandparent = parent.parent()?;
    let grandparent_name = grandparent.file_name()?.to_string_lossy();
    if grandparent_name.eq_ignore_ascii_case("images") {
        let great_grandparent = grandparent.parent()?;
        let labels_dir = great_grandparent.join("labels").join(&*parent_name);
        let txt = labels_dir.join(format!("{}.txt", stem));
        if txt.exists() {
            return Some(txt);
        }
    }

    if parent_name.eq_ignore_ascii_case("images") {
        let labels_dir = parent.parent()?.join("labels");
        let txt = labels_dir.join(format!("{}.txt", stem));
        if txt.exists() {
            return Some(txt);
        }
    }

    None
}

#[tauri::command]
pub fn read_yolo_labels(image_path: String) -> Result<Vec<YoloAnnotation>, String> {
    let img_path = std::path::Path::new(&image_path);
    let stem = img_path.file_stem().ok_or("Invalid image path")?.to_string_lossy();
    let parent = img_path.parent().ok_or("Invalid image path")?;

    let txt_path = resolve_label_path(img_path);

    let txt_path = txt_path.unwrap_or_else(|| {
        let p = parent.join("labels").join(format!("{}.txt", stem));
        if p.exists() { p } else { parent.join(format!("{}.txt", stem)) }
    });

    if !txt_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&txt_path).map_err(|e| e.to_string())?;
    let mut annotations = vec![];
    for line in content.lines() {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.len() >= 5 {
            if let (Ok(cid), Ok(xc), Ok(yc), Ok(w), Ok(h)) = (
                parts[0].parse::<usize>(),
                parts[1].parse::<f64>(),
                parts[2].parse::<f64>(),
                parts[3].parse::<f64>(),
                parts[4].parse::<f64>(),
            ) {
                annotations.push(YoloAnnotation {
                    class_id: cid,
                    x_center: xc,
                    y_center: yc,
                    width: w,
                    height: h,
                });
            }
        }
    }
    Ok(annotations)
}

#[tauri::command]
pub fn save_yolo_labels(
    image_path: String,
    annotations: Vec<YoloAnnotation>,
) -> Result<(), String> {
    let img_path = std::path::Path::new(&image_path);
    let stem = img_path.file_stem().ok_or("Invalid image path")?.to_string_lossy();
    let parent = img_path.parent().ok_or("Invalid image path")?;
    let parent_name = parent.file_name().ok_or("Invalid image path")?.to_string_lossy();

    let txt_path = {
        let grandparent = parent.parent().ok_or("Invalid image path")?;
        let grandparent_name = grandparent.file_name().ok_or("Invalid image path")?.to_string_lossy();

        if grandparent_name.eq_ignore_ascii_case("images") {
            let great_grandparent = grandparent.parent().ok_or("Invalid image path")?;
            let labels_dir = great_grandparent.join("labels").join(&*parent_name);
            std::fs::create_dir_all(&labels_dir).map_err(|e| e.to_string())?;
            labels_dir.join(format!("{}.txt", stem))
        } else if parent_name.eq_ignore_ascii_case("images") {
            let labels_dir = grandparent.join("labels");
            std::fs::create_dir_all(&labels_dir).map_err(|e| e.to_string())?;
            labels_dir.join(format!("{}.txt", stem))
        } else {
            let labels_dir = parent.join("labels");
            if labels_dir.exists() || std::fs::create_dir_all(&labels_dir).is_ok() {
                labels_dir.join(format!("{}.txt", stem))
            } else {
                parent.join(format!("{}.txt", stem))
            }
        }
    };

    let mut content = String::new();
    for ann in &annotations {
        content.push_str(&format!(
            "{} {:.6} {:.6} {:.6} {:.6}\n",
            ann.class_id, ann.x_center, ann.y_center, ann.width, ann.height
        ));
    }
    std::fs::write(&txt_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_image_dimensions(path: String) -> Result<(u32, u32), String> {
    let img = image::open(&path).map_err(|e| format!("Failed to open image: {}", e))?;
    Ok((img.width(), img.height()))
}

// ============================================================================
// Desktop Capture Commands
// ============================================================================

use std::sync::Arc;

#[tauri::command]
pub fn start_capture(
    state: State<AppState>,
    model_path: Option<String>,
    conf_threshold: Option<f32>,
) -> Result<(), String> {
    {
        let mut capture = state.capture.lock().unwrap();
        if capture.running {
            return Ok(());
        }
        capture.running = true;
        capture.detections.clear();
    }

    let capture_arc: std::sync::Arc<std::sync::Mutex<CaptureState>> = state.capture.clone();

    std::thread::spawn(move || {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::mpsc::RecvTimeoutError;

        let display = match scrap::Display::all().ok().and_then(|d| d.into_iter().next()) {
            Some(d) => d,
            None => {
                let mut cap = capture_arc.lock().unwrap();
                cap.running = false;
                return;
            }
        };

        let width = display.width() as u32;
        let height = display.height() as u32;
        let mut capturer = match scrap::Capturer::new(display) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[capture] Failed to create capturer: {}", e);
                let mut cap = capture_arc.lock().unwrap();
                cap.running = false;
                return;
            }
        };

        for _ in 0..3 {
            let _ = capturer.frame();
        }

        let onnx_model_path = model_path.as_ref().and_then(|p| resolve_onnx_path(p).ok());

        let mut engine: Option<rusttools_app::services::yolo_onnx::YoloOnnxEngine> = None;
        if let Some(ref path) = onnx_model_path {
            match rusttools_app::services::yolo_onnx::YoloOnnxEngine::new(
                path.to_str().unwrap_or("")
            ) {
                Ok(mut e) => {
                    if let Some(th) = conf_threshold {
                        e.set_conf_threshold(th);
                    }
                    engine = Some(e);
                }
                Err(e) => eprintln!("[capture] Failed to load ONNX model: {}", e),
            }
        }

        let (infer_tx, infer_rx) = std::sync::mpsc::sync_channel::<(Vec<u8>, u32, u32)>(1);
        let infer_running = Arc::new(AtomicBool::new(true));
        let infer_running_c = Arc::clone(&infer_running);
        let infer_capture_arc = Arc::clone(&capture_arc);

        let infer_handle = if let Some(mut engine) = engine {
            Some(std::thread::spawn(move || {
                while infer_running_c.load(Ordering::Relaxed) {
                    match infer_rx.recv_timeout(std::time::Duration::from_millis(10)) {
                        Ok((bgra, w, h)) => {
                            match engine.infer_from_bgra(&bgra, w, h) {
                                Ok(dets) => {
                                    let detections: Vec<OnnxDetection> = dets.into_iter().map(|d| OnnxDetection {
                                        class_id: d.class_id,
                                        confidence: d.confidence,
                                        x1: d.x1,
                                        y1: d.y1,
                                        x2: d.x2,
                                        y2: d.y2,
                                    }).collect();
                                    let mut cap = infer_capture_arc.lock().unwrap();
                                    if cap.running {
                                        cap.detections = detections;
                                    }
                                }
                                Err(e) => eprintln!("[capture] Inference error: {}", e),
                            }
                        }
                        Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            }))
        } else {
            None
        };

        let mut frame_count = 0u32;
        let start_time = std::time::Instant::now();

        while capture_arc.lock().unwrap().running {
            let start_frame = std::time::Instant::now();

            match capturer.frame() {
                Ok(frame) => {
                    // 只 clone 一次给推理线程，RGB 转换直接从 frame 读取
                    let bgra_for_infer: Vec<u8> = frame.to_vec();
                    let _ = infer_tx.try_send((bgra_for_infer, width, height));

                    let mut rgb_data = vec![0u8; (width * height * 3) as usize];
                    for (i, chunk) in frame.chunks_exact(4).enumerate() {
                        rgb_data[i * 3] = chunk[2];
                        rgb_data[i * 3 + 1] = chunk[1];
                        rgb_data[i * 3 + 2] = chunk[0];
                    }

                    let mut jpeg_bytes: Vec<u8> = Vec::new();
                    if {
                        let mut cursor = std::io::Cursor::new(&mut jpeg_bytes);
                        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 30);
                        encoder.encode(&rgb_data, width, height, image::ExtendedColorType::Rgb8).is_ok()
                    } {
                        use base64::Engine;
                        let base64 = base64::engine::general_purpose::STANDARD.encode(&jpeg_bytes);
                        let mut cap = capture_arc.lock().unwrap();
                        if cap.running {
                            cap.last_frame_base64 = Some(base64);
                            frame_count += 1;
                            let elapsed = start_time.elapsed().as_secs_f32();
                            if elapsed > 0.0 {
                                cap.fps = frame_count as f32 / elapsed;
                            }
                        }
                    }
                }
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }

            let frame_time = start_frame.elapsed();
            if frame_time < std::time::Duration::from_millis(16) {
                std::thread::sleep(std::time::Duration::from_millis(16) - frame_time);
            }
        }

        infer_running.store(false, Ordering::Relaxed);
        drop(infer_tx);
        if let Some(h) = infer_handle {
            let _ = h.join();
        }
    });

    Ok(())
}

#[tauri::command]
pub fn stop_capture(state: State<AppState>) -> Result<(), String> {
    {
        let mut capture = state.capture.lock().unwrap();
        capture.running = false;
        capture.last_frame_base64 = None;
    }
    Ok(())
}

#[tauri::command]
pub fn get_capture_state(state: State<AppState>) -> Result<CaptureState, String> {
    Ok(state.capture.lock().unwrap().clone())
}

#[tauri::command]
pub fn get_capture_frame(state: State<AppState>) -> Result<Option<String>, String> {
    Ok(state.capture.lock().unwrap().last_frame_base64.clone())
}
