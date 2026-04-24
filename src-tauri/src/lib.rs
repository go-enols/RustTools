use std::sync::{Arc, Mutex};
use tauri::State;

// ============================================================================
// AppState
// ============================================================================

pub struct AppState {
    pub current_project: Mutex<Option<rusttools_app::models::ProjectConfig>>,
    pub trainer_service: Arc<rusttools_app::services::trainer::TrainerService>,
    pub yolo_engine: Mutex<Option<rusttools_app::services::yolo_onnx::YoloOnnxEngine>>,
    pub capture: Arc<Mutex<CaptureState>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_project: Mutex::new(None),
            trainer_service: Arc::new(rusttools_app::services::trainer::TrainerService::new()),
            yolo_engine: Mutex::new(None),
            capture: Arc::new(Mutex::new(CaptureState::default())),
        }
    }
}

#[derive(Default, serde::Serialize, Clone)]
pub struct OnnxDetection {
    pub class_id: usize,
    pub confidence: f32,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

#[derive(Default, serde::Serialize)]
pub struct CaptureState {
    pub running: bool,
    pub fps: f32,
    pub last_frame_base64: Option<String>,
    pub detections: Vec<OnnxDetection>,
}

// ============================================================================
// Environment Commands
// ============================================================================

#[tauri::command]
fn get_env_status() -> Result<rusttools_app::services::python_env::PythonEnvStatus, String> {
    Ok(rusttools_app::services::python_env::get_env_status())
}

#[tauri::command]
fn refresh_env_status() -> Result<rusttools_app::services::python_env::PythonEnvStatus, String> {
    Ok(rusttools_app::services::python_env::refresh_env_status())
}

#[tauri::command]
fn generate_env_report() -> Result<rusttools_app::services::env::EnvReport, String> {
    Ok(rusttools_app::services::env::generate_env_report())
}

#[tauri::command]
fn refresh_env_report() -> Result<rusttools_app::services::env::EnvReport, String> {
    Ok(rusttools_app::services::env::refresh_env_report())
}

#[tauri::command]
fn install_python_env() -> Result<(), String> {
    // 启动后台安装线程（非阻塞），安装进度通过 get_env_status() 的 installing 字段查询
    rusttools_app::services::python_env::install_python_deps(None, None);
    Ok(())
}

#[tauri::command]
fn get_device_info() -> Result<DeviceInfo, String> {
    use sysinfo::{System, RefreshKind, CpuRefreshKind, MemoryRefreshKind};

    let cuda = rusttools_app::services::env::detect_cuda();

    let gpus = cuda.gpus.into_iter().map(|gpu| GpuInfo {
        name: gpu.name,
        memory_mb: gpu.memory_mb,
        cuda_available: cuda.available,
    }).collect();

    // 使用 sysinfo 获取真实的 CPU 和内存信息
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );
    sys.refresh_all();

    let cpu_model = sys.cpus().first()
        .map(|cpu| cpu.brand().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let physical_cores = sys.physical_core_count().unwrap_or(1);
    let logical_threads = sys.cpus().len();

    let total_mb = sys.total_memory();
    let used_mb = sys.used_memory();

    Ok(DeviceInfo {
        cpu: CpuInfo {
            model: cpu_model,
            cores: physical_cores,
            threads: logical_threads,
        },
        memory: MemoryInfo {
            total_mb,
            used_mb,
        },
        gpus,
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    })
}

#[derive(serde::Serialize)]
struct CpuInfo {
    model: String,
    cores: usize,
    threads: usize,
}

#[derive(serde::Serialize)]
struct MemoryInfo {
    total_mb: u64,
    used_mb: u64,
}

#[derive(serde::Serialize)]
struct GpuInfo {
    name: String,
    memory_mb: u64,
    cuda_available: bool,
}

#[derive(serde::Serialize)]
struct DeviceInfo {
    cpu: CpuInfo,
    memory: MemoryInfo,
    gpus: Vec<GpuInfo>,
    os: String,
    arch: String,
}

// ============================================================================
// Dialog Commands
// ============================================================================

#[tauri::command]
async fn pick_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let file_path = app.dialog().file().blocking_pick_folder();
    Ok(file_path.map(|p| p.to_string()))
}

#[tauri::command]
async fn pick_file(app: tauri::AppHandle, extensions: Vec<String>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let mut builder = app.dialog().file();
    if !extensions.is_empty() {
        builder = builder.add_filter("files", &extensions.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    }
    let file_path = builder.blocking_pick_file();
    Ok(file_path.map(|p| p.to_string()))
}

// ============================================================================
// Project Commands
// ============================================================================

#[tauri::command]
fn create_project(
    config: rusttools_app::models::ProjectConfig,
    state: State<AppState>,
) -> Result<rusttools_app::models::ProjectResponse, String> {
    let response = rusttools_app::services::project::create_project(config.clone());
    if response.success {
        *state.current_project.lock().unwrap() = Some(config);
    }
    Ok(response)
}

#[tauri::command]
fn open_project(
    path: String,
    state: State<AppState>,
) -> Result<rusttools_app::models::ProjectResponse, String> {
    let response = rusttools_app::services::project::open_project(path);
    if let Some(ref config) = response.data {
        *state.current_project.lock().unwrap() = Some(config.clone());
    }
    Ok(response)
}

#[tauri::command]
fn get_current_project(
    state: State<AppState>,
) -> Result<Option<rusttools_app::models::ProjectConfig>, String> {
    Ok(state.current_project.lock().unwrap().clone())
}

#[tauri::command]
fn update_project_classes(
    path: String,
    classes: Vec<String>,
) -> Result<(), String> {
    rusttools_app::services::project::update_classes(path, classes)
        .map_err(|e| e)
}

#[tauri::command]
fn scan_project(path: String) -> Result<rusttools_app::models::ProjectScanResult, String> {
    Ok(rusttools_app::services::project::scan_project(&path))
}

// ============================================================================
// Training Commands
// ============================================================================

#[tauri::command]
async fn start_training(
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
async fn stop_training(
    training_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.trainer_service.stop_training(&training_id).await
}

#[tauri::command]
async fn get_training_status(
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
async fn list_training_logs(
    training_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    Ok(state.trainer_service.get_logs(&training_id).await)
}

fn format_unix_timestamp(ts: u64) -> String {
    // 简化的本地时间格式化（避免 chrono 依赖）
    let days_since_epoch = ts / 86400;
    let seconds_of_day = ts % 86400;
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    // 粗略计算年月日（基于 1970-01-01，不考虑闰秒和时区）
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
fn list_training_results(project_path: String) -> Result<Vec<TrainingResult>, String> {
    let train_dir = std::path::Path::new(&project_path).join("runs").join("train");
    if !train_dir.exists() {
        return Ok(vec![]);
    }

    let mut results = Vec::new();
    for entry in std::fs::read_dir(&train_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let weights_dir = path.join("weights");
        let has_best = weights_dir.join("best.pt").exists();
        let has_last = weights_dir.join("last.pt").exists();

        // 尝试读取 results.csv 获取最后一轮的指标
        let mut epochs_completed = 0u32;
        let mut map50 = 0.0f32;
        let mut map50_95 = 0.0f32;

        let results_csv = path.join("results.csv");
        if results_csv.exists() {
            if let Ok(content) = std::fs::read_to_string(&results_csv) {
                let lines: Vec<&str> = content.lines().collect();
                if lines.len() >= 2 {
                    // 跳过表头，取最后一行
                    let last_line = lines.last().unwrap_or(&"");
                    let parts: Vec<&str> = last_line.split(',').map(|s| s.trim()).collect();
                    // Ultralytics results.csv columns:
                    // epoch, time, train/box_loss, train/cls_loss, train/dfl_loss,
                    // metrics/precision(B), metrics/recall(B), metrics/mAP50(B), metrics/mAP50-95(B),
                    // val/box_loss, val/cls_loss, val/dfl_loss
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

    // 按创建时间倒序
    results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(results)
}

// ============================================================================
// Inference Commands
// ============================================================================

#[tauri::command]
fn load_model(
    model_path: String,
    state: State<AppState>,
) -> Result<(), String> {
    let engine = rusttools_app::services::yolo_onnx::YoloOnnxEngine::new(&model_path)?;
    *state.yolo_engine.lock().unwrap() = Some(engine);
    Ok(())
}

#[tauri::command]
fn run_inference_image(
    image_path: String,
    conf_threshold: Option<f32>,
    state: State<AppState>,
) -> Result<Vec<rusttools_app::services::yolo_onnx::OnnxDetection>, String> {
    let mut guard = state.yolo_engine.lock().unwrap();
    let engine = guard.as_mut().ok_or("No model loaded. Call load_model first.")?;

    let image = image::open(&image_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;

    if let Some(threshold) = conf_threshold {
        engine.set_conf_threshold(threshold);
    }

    let detections = engine.infer(&image)?;
    Ok(detections)
}

#[tauri::command]
fn unload_model(state: State<AppState>) -> Result<(), String> {
    *state.yolo_engine.lock().unwrap() = None;
    Ok(())
}

#[derive(serde::Serialize)]
struct ModelInfo {
    name: String,
    path: String,
    size: u64,
}

#[tauri::command]
fn list_models(dir: Option<String>, ext: Option<String>) -> Result<Vec<ModelInfo>, String> {
    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let target_ext = ext.unwrap_or_else(|| "onnx".to_string()).to_lowercase();

    let mut scan_dir = |path: &std::path::Path| {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Some(file_ext) = p.extension() {
                        if file_ext.to_string_lossy().to_lowercase() == target_ext {
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
        // 默认扫描路径：exe 同级目录、models 子目录、当前工作目录
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
fn extract_video_frame(video_path: String, timestamp_sec: f64) -> Result<String, String> {
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
fn list_images(folder: String, recursive: Option<bool>) -> Result<Vec<String>, String> {
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

/// Resolve the label file path for a given image path using YOLO directory conventions.
/// Supports structures like:
///   - project/images/train/img.jpg  →  project/labels/train/img.txt
///   - project/images/img.jpg        →  project/labels/img.txt
fn resolve_label_path(img_path: &std::path::Path) -> Option<std::path::PathBuf> {
    let stem = img_path.file_stem()?.to_string_lossy();
    let parent = img_path.parent()?;
    let parent_name = parent.file_name()?.to_string_lossy();

    // Case 1: /project/images/train/img.jpg → /project/labels/train/img.txt
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

    // Case 2: /project/images/img.jpg → /project/labels/img.txt
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
fn read_yolo_labels(image_path: String) -> Result<Vec<rusttools_app::models::YoloAnnotation>, String> {
    let img_path = std::path::Path::new(&image_path);
    let stem = img_path.file_stem().ok_or("Invalid image path")?.to_string_lossy();
    let parent = img_path.parent().ok_or("Invalid image path")?;

    // 1. Try standard YOLO images→labels mapping
    let txt_path = resolve_label_path(img_path);

    // 2. Fallback: labels/ subdir or same directory
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
                annotations.push(rusttools_app::models::YoloAnnotation {
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
fn save_yolo_labels(
    image_path: String,
    annotations: Vec<rusttools_app::models::YoloAnnotation>,
) -> Result<(), String> {
    let img_path = std::path::Path::new(&image_path);
    let stem = img_path.file_stem().ok_or("Invalid image path")?.to_string_lossy();
    let parent = img_path.parent().ok_or("Invalid image path")?;
    let parent_name = parent.file_name().ok_or("Invalid image path")?.to_string_lossy();

    // Try standard YOLO images→labels mapping first
    let txt_path = {
        let grandparent = parent.parent().ok_or("Invalid image path")?;
        let grandparent_name = grandparent.file_name().ok_or("Invalid image path")?.to_string_lossy();

        if grandparent_name.eq_ignore_ascii_case("images") {
            // /project/images/train/img.jpg → /project/labels/train/img.txt
            let great_grandparent = grandparent.parent().ok_or("Invalid image path")?;
            let labels_dir = great_grandparent.join("labels").join(&*parent_name);
            std::fs::create_dir_all(&labels_dir).map_err(|e| e.to_string())?;
            labels_dir.join(format!("{}.txt", stem))
        } else if parent_name.eq_ignore_ascii_case("images") {
            // /project/images/img.jpg → /project/labels/img.txt
            let labels_dir = grandparent.join("labels");
            std::fs::create_dir_all(&labels_dir).map_err(|e| e.to_string())?;
            labels_dir.join(format!("{}.txt", stem))
        } else {
            // Fallback: labels/ subdir or same directory
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
fn get_image_dimensions(path: String) -> Result<(u32, u32), String> {
    let img = image::open(&path).map_err(|e| format!("Failed to open image: {}", e))?;
    Ok((img.width(), img.height()))
}

// ============================================================================
// Desktop Capture Commands
// ============================================================================

#[tauri::command]
fn start_capture(
    state: State<AppState>,
    model_path: Option<String>,
    conf_threshold: Option<f32>,
) -> Result<(), String> {
    let mut capture = state.capture.lock().unwrap();
    if capture.running {
        return Ok(());
    }
    capture.running = true;
    capture.detections.clear();
    drop(capture);

    let capture_arc = state.capture.clone();

    std::thread::spawn(move || {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::mpsc::RecvTimeoutError;

        // 1. Initialize scrap capturer (direct frame buffer access, not screenshot API)
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

        // Warm-up frames
        for _ in 0..3 {
            let _ = capturer.frame();
        }

        // 2. Load ONNX model
        let mut engine: Option<rusttools_app::services::yolo_onnx::YoloOnnxEngine> = None;
        if let Some(ref path) = model_path {
            match rusttools_app::services::yolo_onnx::YoloOnnxEngine::new(path) {
                Ok(mut e) => {
                    if let Some(th) = conf_threshold {
                        e.set_conf_threshold(th);
                    }
                    engine = Some(e);
                }
                Err(e) => eprintln!("[capture] Failed to load model: {}", e),
            }
        }

        // 3. mpsc channel: capture thread -> inference thread
        let (infer_tx, infer_rx) = std::sync::mpsc::sync_channel::<(Vec<u8>, u32, u32)>(1);
        let infer_running = Arc::new(AtomicBool::new(true));
        let infer_running_c = Arc::clone(&infer_running);
        let infer_capture_arc = Arc::clone(&capture_arc);

        // 4. Spawn inference thread (completely decoupled from capture)
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
                    let bgra_data: Vec<u8> = frame.to_vec();

                    // Send BGRA to inference thread (non-blocking, drop old frames if full)
                    let _ = infer_tx.try_send((bgra_data.clone(), width, height));

                    // Fast BGRA -> RGB conversion for JPEG encoding
                    let mut rgb_data = vec![0u8; (width * height * 3) as usize];
                    for (i, chunk) in bgra_data.chunks_exact(4).enumerate() {
                        rgb_data[i * 3] = chunk[2];     // R
                        rgb_data[i * 3 + 1] = chunk[1]; // G
                        rgb_data[i * 3 + 2] = chunk[0]; // B
                    }

                    let mut jpeg_bytes: Vec<u8> = Vec::new();
                    if {
                        let mut cursor = std::io::Cursor::new(&mut jpeg_bytes);
                        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 30);
                        encoder.encode(&rgb_data, width, height, image::ExtendedColorType::Rgb8).is_ok()
                    } {
                        let base64 = {
                            use base64::Engine;
                            base64::engine::general_purpose::STANDARD.encode(&jpeg_bytes)
                        };
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

        // Signal inference thread to stop and wait
        infer_running.store(false, Ordering::Relaxed);
        drop(infer_tx);
        if let Some(h) = infer_handle {
            let _ = h.join();
        }
    });

    Ok(())
}

#[tauri::command]
fn stop_capture(state: State<AppState>) -> Result<(), String> {
    let mut capture = state.capture.lock().unwrap();
    capture.running = false;
    capture.last_frame_base64 = None;
    Ok(())
}

#[tauri::command]
fn get_capture_state(state: State<AppState>) -> Result<CaptureState, String> {
    let capture = state.capture.lock().unwrap();
    Ok(CaptureState {
        running: capture.running,
        fps: capture.fps,
        last_frame_base64: capture.last_frame_base64.clone(),
        detections: capture.detections.clone(),
    })
}

#[tauri::command]
fn get_capture_frame(state: State<AppState>) -> Result<Option<String>, String> {
    Ok(state.capture.lock().unwrap().last_frame_base64.clone())
}

// ============================================================================
// Application Entry
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_env_status,
            refresh_env_status,
            generate_env_report,
            refresh_env_report,
            install_python_env,
            get_device_info,
            pick_folder,
            pick_file,
            create_project,
            open_project,
            get_current_project,
            update_project_classes,
            scan_project,
            start_training,
            stop_training,
            get_training_status,
            list_training_logs,
            list_training_results,
            load_model,
            run_inference_image,
            unload_model,
            list_models,
            list_images,
            read_yolo_labels,
            save_yolo_labels,
            get_image_dimensions,
            extract_video_frame,
            start_capture,
            stop_capture,
            get_capture_state,
            get_capture_frame,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_device_info_returns_valid_struct() {
        let result = get_device_info();
        assert!(result.is_ok(), "get_device_info should succeed");
        let info = result.unwrap();
        assert!(!info.os.is_empty(), "OS should not be empty");
        assert!(!info.arch.is_empty(), "Architecture should not be empty");
        assert!(info.cpu.cores > 0, "CPU cores should be > 0");
        let _ = info.gpus.len();
    }

    #[test]
    fn test_generate_env_report_returns_all_fields() {
        let result = generate_env_report();
        assert!(result.is_ok(), "generate_env_report should succeed");
        let report = result.unwrap();

        assert!(
            matches!(
                report.system.os,
                rusttools_app::services::env::OsType::Linux
                    | rusttools_app::services::env::OsType::Windows
                    | rusttools_app::services::env::OsType::MacOS
            ),
            "OS should be detected"
        );
        assert!(report.system.cpu_cores > 0, "CPU cores should be > 0");

        let _ = report.cuda.available;
        let _ = report.cuda.gpus.len();
        let _ = report.uv_installed;
        let _ = report.python_installed;
        let _ = report.venv_exists;
        let _ = report.torch_available;
        let _ = report.torch_cuda;
        let _ = report.ort_available;
        let _ = report.ort_cuda;
    }

    #[test]
    fn test_get_env_status_cuda_matches_report() {
        let status = get_env_status().unwrap();
        let report = generate_env_report().unwrap();

        // CUDA detection should be consistent between both functions
        assert_eq!(
            status.cuda_available, report.cuda.available,
            "CUDA availability should match between status and report"
        );
    }

    #[test]
    fn test_install_python_env_starts_installation() {
        let result = install_python_env();
        assert!(result.is_ok(), "install_python_env should return Ok immediately (installation runs in background)");
    }

    #[test]
    fn test_scan_project_nonexistent_path() {
        let scan = scan_project("/nonexistent/path".to_string()).unwrap();
        assert_eq!(scan.train_images, 0, "Non-existent path should have 0 train images");
        assert_eq!(scan.val_images, 0, "Non-existent path should have 0 val images");
        assert_eq!(scan.total_annotations, 0, "Non-existent path should have 0 annotations");
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(state.current_project.lock().unwrap().is_none());
        assert!(state.yolo_engine.lock().unwrap().is_none());
        assert!(!state.capture.lock().unwrap().running);
    }
}
