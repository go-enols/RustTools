use crate::models::{TrainingMetrics, TrainingRequest, TrainingStatus};
use crate::services::python_env::resolve_managed_python;
use rand::Rng;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::StreamExt;

pub struct TrainerService {
    processes: Arc<RwLock<HashMap<String, TrainingHandle>>>,
}

struct TrainingHandle {
    status: TrainingStatus,
    stop_tx: Option<mpsc::Sender<()>>,
    child: Child,
    stdin: Option<tokio::io::BufWriter<tokio::process::ChildStdin>>,
    model_path: Option<String>,
    stderr_rx: tokio::sync::oneshot::Receiver<String>,
    log_messages: Vec<String>,
}

impl TrainerService {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn generate_id() -> String {
        let mut rng = rand::rng();
        let bytes: [u8; 16] = rng.random();
        hex::encode(bytes)
    }

    /// Find and start the Python sidecar process
    async fn start_sidecar(&self) -> Result<(Child, tokio::sync::oneshot::Receiver<String>), String> {
        // Try to find script relative to executable first, then relative to cwd
        let mut candidates = Vec::new();

        // 1. Relative to current executable (for packaged apps)
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                candidates.push(exe_dir.join("python/scripts/yolo_server.py"));
                candidates.push(exe_dir.join("../python/scripts/yolo_server.py"));
                candidates.push(exe_dir.join("../../python/scripts/yolo_server.py"));
                candidates.push(exe_dir.join("scripts/yolo_server.py"));
                candidates.push(exe_dir.join("../scripts/yolo_server.py"));
            }
        }

        // 2. Relative to current working directory (for dev mode)
        candidates.push(std::path::PathBuf::from("python/scripts/yolo_server.py"));
        candidates.push(std::path::PathBuf::from("scripts/yolo_server.py"));
        candidates.push(std::path::PathBuf::from("src-tauri/scripts/yolo_server.py"));

        let script_path = candidates
            .iter()
            .find(|p| p.exists())
            .ok_or_else(|| {
                let checked = candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ");
                format!("Python sidecar script not found. Checked: {}", checked)
            })?;

        let script_path = script_path.to_string_lossy().to_string();

        let python_cmd = resolve_managed_python()
            .or_else(|| {
                // Fallback: try the general resolver (system python)
                crate::services::python_env::resolved_python()
            })
            .ok_or_else(|| "Python not found. Please install the environment in Settings first.".to_string())?;

        // Ensure ultralytics is installed; auto-install via uv if missing
        match tokio::process::Command::new(&python_cmd)
            .args(["-c", "import ultralytics; print('ok')"])
            .output()
            .await
        {
            Ok(output) if output.status.success() => {}
            _ => {
                eprintln!("[Trainer] ultralytics not found in {}, attempting to install...", python_cmd);
                let manager = crate::services::python_env::UvManager::new();
                if let Some(uv) = manager.uv_path() {
                    let install = tokio::process::Command::new(uv)
                        .args(["pip", "install", "--python", &python_cmd, "ultralytics"])
                        .output()
                        .await
                        .map_err(|e| format!("Failed to run uv install ultralytics: {}", e))?;
                    if !install.status.success() {
                        let stderr = String::from_utf8_lossy(&install.stderr);
                        return Err(format!("ultralytics is not installed and auto-install failed: {}", stderr));
                    }
                    eprintln!("[Trainer] ultralytics installed successfully");
                } else {
                    return Err("ultralytics is not installed and uv is not available. Please install the environment in Settings.".to_string());
                }
            }
        }

        // Capture stderr so Python-side errors (ImportError, CUDA init failure)
        // are relayed to the frontend instead of lost to the terminal.
        // We use a oneshot channel to collect stderr output from a blocking thread.
        let mut child = Command::new(&python_cmd)
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to start Python sidecar: {}", e))?;

        // Create oneshot channel to receive stderr output
        let (tx, rx) = tokio::sync::oneshot::channel::<String>();

        // Take stderr and read it in a blocking thread using a nested tokio runtime
        // because tokio::process::ChildStderr only implements tokio::io::AsyncRead, not std::io::Read
        let stderr = child.stderr.take().expect("stderr was piped");
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create runtime for stderr reading");
            rt.block_on(async {
                use tokio::io::AsyncReadExt;
                let mut reader = tokio::io::BufReader::new(stderr);
                let mut buf = String::new();
                reader.read_to_string(&mut buf).await.ok();
                let _ = tx.send(buf);
            });
        });

        Ok((child, rx))
    }

    pub async fn start_training(
        &self,
        project_path: String,
        request: TrainingRequest,
    ) -> Result<String, String> {
        let training_id = Self::generate_id();
        self.start_training_pipe(training_id, project_path, request)
            .await
    }

    /// Start training via stdin/stdout pipe
    /// Python actively reports progress via callbacks
    async fn start_training_pipe(
        &self,
        training_id: String,
        project_path: String,
        request: TrainingRequest,
    ) -> Result<String, String> {
        eprintln!("[Trainer] Starting training via pipe...");

        // 1) Ensure model is present in cache before launching sidecar
        let model_path = match self.check_model(&request.base_model).await {
            Ok((true, Some(path))) => {
                eprintln!("[Trainer] Model found in cache: {}", path);
                path
            }
            _ => {
                eprintln!("[Trainer] Model not cached, downloading {}...", request.base_model);
                self.download_model(&request.base_model, |msg| {
                    eprintln!("[Trainer] Download: {}", msg);
                }).await.map_err(|e| format!("模型下载失败: {}", e))?
            }
        };

        let (mut child, stderr_rx) = self.start_sidecar().await?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to take stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to take stdout".to_string())?;

        // Build config — pass absolute model path so Python never needs to download
        let config = serde_json::json!({
            "project_path": project_path,
            "base_model": model_path,          // absolute path, e.g. /home/xxx/.cache/ultralytics/yolo11n.pt
            "epochs": request.epochs,
            "batch_size": request.batch_size,
            "image_size": request.image_size,
            "device": request.device_id,
            "workers": request.workers,
            "optimizer": request.optimizer,
        });

        // Send start command - keep stdin alive by wrapping in BufWriter but not dropping
        let start_cmd = serde_json::json!({
            "type": "start",
            "config": config,
        });

        let mut stdin_writer = tokio::io::BufWriter::new(stdin);
        stdin_writer
            .write_all(
                (serde_json::to_string(&start_cmd).unwrap() + "\n").as_bytes(),
            )
            .await
            .map_err(|e| format!("Failed to write start command: {}", e))?;
        stdin_writer
            .flush()
            .await
            .map_err(|e| format!("Failed to flush stdin: {}", e))?;

        // Keep stdin writer alive - don't drop it here! Python needs stdin to stay open.

        let (stop_tx, stop_rx) = mpsc::channel::<()>(1);

        let handle = TrainingHandle {
            status: TrainingStatus {
                running: true,
                paused: false,
                epoch: 0,
                total_epochs: request.epochs,
                progress_percent: 0.0,
                metrics: TrainingMetrics::default(),
                error: None,
            },
            stop_tx: Some(stop_tx),
            child,
            stdin: Some(stdin_writer),  // Keep stdin alive
            model_path: None,
            stderr_rx,
            log_messages: Vec::new(),
        };

        self.processes
            .write()
            .await
            .insert(training_id.clone(), handle);

        // Spawn event reader - Python actively sends events
        let process_id = training_id.clone();
        let processes = Arc::clone(&self.processes);
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stop_rx = stop_rx;

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop_rx.recv() => {
                        // Graceful stop already sent via stdin in stop_training().
                        // Give Python up to 3 seconds to exit gracefully before killing.
                        eprintln!("[Trainer] Stop signal received, waiting for graceful shutdown...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                        if let Some(h) = processes.write().await.get_mut(&process_id) {
                            if h.status.running {
                                eprintln!("[Trainer] Python did not exit gracefully, forcing kill...");
                                let _ = h.child.kill().await;
                                h.status.running = false;
                            } else {
                                eprintln!("[Trainer] Python exited gracefully");
                            }
                        }
                        break;
                    }
                    line = stdout_reader.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                // Parse JSON event from Python
                                if let Ok(resp) = serde_json::from_str::<serde_json::Value>(&line) {
                                    let event_type = resp
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");

                                    eprintln!("[Trainer] Received event: {} - {}", event_type, line);

                                    match event_type {
                                        "progress" => {
                                            eprintln!("[Trainer] Processing progress event from Python");
                                            if let Some(data) = resp.get("data") {
                                                let epoch = data
                                                    .get("epoch")
                                                    .and_then(|v| v.as_u64())
                                                    .unwrap_or(0) as u32;
                                                let total = data
                                                    .get("total_epochs")
                                                    .and_then(|v| v.as_u64())
                                                    .unwrap_or(0) as u32;

                                                if let Some(h) = processes.write().await.get_mut(&process_id) {
                                                    h.status.epoch = epoch;
                                                    h.status.total_epochs = total;
                                                    if total > 0 {
                                                        h.status.progress_percent =
                                                            (epoch as f32 / total as f32) * 100.0;
                                                    }

                                                    // Extract metrics
                                                    let mut metrics = TrainingMetrics::default();
                                                    metrics.train_box_loss =
                                                        data.get("box_loss").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                    metrics.train_cls_loss =
                                                        data.get("cls_loss").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                    metrics.train_dfl_loss =
                                                        data.get("dfl_loss").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                    metrics.precision =
                                                        data.get("precision").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                    metrics.recall =
                                                        data.get("recall").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                    metrics.map50 =
                                                        data.get("mAP50").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                    metrics.map50_95 =
                                                        data.get("mAP50-95").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

                                                    eprintln!("[Trainer] Progress updated: epoch={}, metrics box_loss={}", epoch, metrics.train_box_loss);
                                                    h.status.metrics = metrics;
                                                }
                                            }
                                        }
                                        "started" => {
                                            eprintln!("[Trainer] Training started event received");
                                        }
                                        "complete" => {
                                            if let Some(h) = processes.write().await.get_mut(&process_id) {
                                                h.status.running = false;
                                                if let Some(data) = resp.get("data") {
                                                    if let Some(_final_metrics) = data.get("final_metrics") {
                                                        h.status.progress_percent = 100.0;
                                                    }
                                                }
                                            }
                                            eprintln!("[Trainer] Training complete event received");
                                            break;
                                        }
                                        "error" => {
                                            let error_msg = resp
                                                .get("error")
                                                .and_then(|v| v.as_str())
                                                .map(String::from)
                                                .unwrap_or_else(|| "Unknown error".to_string());
                                            eprintln!("[Trainer] Received error event: {}", error_msg);
                                            if let Some(h) = processes.write().await.get_mut(&process_id) {
                                                h.status.running = false;
                                                h.status.error = Some(error_msg.clone());
                                            }
                                            eprintln!("[Trainer] Training error event processed: {}", error_msg);
                                            break;
                                        }
                                        "stopped" => {
                                            if let Some(h) = processes.write().await.get_mut(&process_id) {
                                                h.status.running = false;
                                            }
                                            eprintln!("[Trainer] Training stopped event received");
                                            break;
                                        }
                                        "log" => {
                                            if let Some(data) = resp.get("data") {
                                                if let Some(msg) = data.get("message").and_then(|v| v.as_str()) {
                                                    eprintln!("[YOLO] {}", msg);
                                                    if let Some(h) = processes.write().await.get_mut(&process_id) {
                                                        h.log_messages.push(msg.to_string());
                                                    }
                                                }
                                            }
                                        }
                                        "model_saved" => {
                                            if let Some(data) = resp.get("data") {
                                                if let Some(path) = data.get("path").and_then(|v| v.as_str()) {
                                                    eprintln!("[Trainer] Model saved: {}", path);
                                                    if let Some(h) = processes.write().await.get_mut(&process_id) {
                                                        h.model_path = Some(path.to_string());
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            eprintln!("[Trainer] Unknown event type: {}", event_type);
                                        }
                                    }
                                }
                            }
                            Ok(None) | Err(_) => {
                                // Stream ended
                                eprintln!("[Trainer] Output stream ended");
                                if let Some(h) = processes.write().await.get_mut(&process_id) {
                                    h.status.running = false;
                                }
                                break;
                            }
                        }
                    }
                }
            }

            // Clean up
            if let Some(h) = processes.write().await.get_mut(&process_id) {
                h.status.running = false;
                // Close stdin to signal EOF to Python so its main() loop exits
                let _ = h.stdin.take();
                // Try to get stderr output from the oneshot receiver
                let stderr_output = match h.stderr_rx.try_recv() {
                    Ok(output) => output,
                    Err(_) => String::new(), // already closed or dropped
                };
                let exit = h.child.wait().await.map(|e| e.code()).ok().flatten();
                if !stderr_output.is_empty() && (exit != Some(0) || h.status.error.is_some()) {
                    h.status.error = Some(stderr_output);
                }
            }
        });

        Ok(training_id)
    }

    pub async fn stop_training(&self, training_id: &str) -> Result<(), String> {
        // Extract handle fields under lock, then release before async I/O
        let (mut stdin_opt, stop_tx_opt) = {
            let mut processes = self.processes.write().await;
            if let Some(handle) = processes.get_mut(training_id) {
                let stdin = handle.stdin.take();
                let tx = handle.stop_tx.take();
                handle.status.running = false;
                handle.status.paused = false;
                (stdin, tx)
            } else {
                return Err("Training not found".to_string());
            }
        };

        // 1) Send graceful stop command to Python via stdin
        if let Some(ref mut stdin) = stdin_opt {
            let stop_cmd = serde_json::json!({"type": "stop"});
            let cmd_str = serde_json::to_string(&stop_cmd).unwrap() + "\n";
            if let Err(e) = stdin.write_all(cmd_str.as_bytes()).await {
                eprintln!("[Trainer] Failed to write stop command: {}", e);
            }
            if let Err(e) = stdin.flush().await {
                eprintln!("[Trainer] Failed to flush stop command: {}", e);
            }
            eprintln!("[Trainer] Stop command sent to Python sidecar");
        }

        // 2) Notify stdout reader to enforce kill after grace period
        if let Some(tx) = stop_tx_opt {
            let _ = tx.send(()).await;
        }

        Ok(())
    }

    pub async fn pause_training(&self, training_id: &str) -> Result<(), String> {
        let mut processes = self.processes.write().await;
        if let Some(handle) = processes.get_mut(training_id) {
            handle.status.paused = true;
            Ok(())
        } else {
            Err("Training not found".to_string())
        }
    }

    pub async fn resume_training(&self, training_id: &str) -> Result<(), String> {
        let mut processes = self.processes.write().await;
        if let Some(handle) = processes.get_mut(training_id) {
            handle.status.paused = false;
            Ok(())
        } else {
            Err("Training not found".to_string())
        }
    }

    pub async fn get_status(&self, training_id: &str) -> Option<TrainingStatus> {
        // Use try_read to avoid blocking the UI thread
        match self.processes.try_read() {
            Ok(processes) => processes.get(training_id).map(|h| h.status.clone()),
            Err(_) => None, // Lock contested, try next frame
        }
    }

    pub async fn get_logs(&self, training_id: &str) -> Vec<String> {
        // Use try_write to avoid blocking the UI thread if the lock is contended
        match self.processes.try_write() {
            Ok(mut processes) => {
                if let Some(h) = processes.get_mut(training_id) {
                    std::mem::take(&mut h.log_messages)
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(), // Lock contested, return empty and try next frame
        }
    }

    pub async fn is_training(&self, training_id: &str) -> bool {
        self.processes
            .read()
            .await
            .get(training_id)
            .map(|h| h.status.running)
            .unwrap_or(false)
    }

    pub async fn get_error(&self, training_id: &str) -> Option<String> {
        self.processes
            .read()
            .await
            .get(training_id)
            .and_then(|h| h.status.error.clone())
    }

    pub async fn get_model_path(&self, training_id: &str) -> Option<String> {
        self.processes
            .read()
            .await
            .get(training_id)
            .and_then(|h| h.model_path.clone())
    }

    /// Check if a model exists locally in cache directory
    pub async fn check_model(&self, model_name: &str) -> Result<(bool, Option<String>), String> {
        let cache_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|h| std::path::PathBuf::from(h).join(".cache").join("ultralytics"))
            .unwrap_or_else(|_| std::path::PathBuf::from(".cache/ultralytics"));

        let model_path = cache_dir.join(model_name);

        if model_path.exists() && model_path.metadata().map(|m| m.len() > 1_000_000).unwrap_or(false) {
            Ok((true, Some(model_path.to_string_lossy().to_string())))
        } else {
            Ok((false, None))
        }
    }

    /// Download a model directly with progress tracking
    pub async fn download_model(
        &self,
        model_name: &str,
        progress_callback: impl Fn(String),
    ) -> Result<String, String> {
        use std::path::PathBuf;

        let download_urls = [
            format!(
                "https://github.com/ultralytics/assets/releases/download/v0.0.0/{}",
                model_name
            ),
            format!(
                "https://github.com/ultralytics/assets/releases/download/v8.3.0/{}",
                model_name
            ),
        ];

        let cache_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|h| PathBuf::from(h).join(".cache").join("ultralytics"))
            .unwrap_or_else(|_| PathBuf::from(".cache/ultralytics"));

        let model_path = cache_dir.join(model_name);

        // Check if model already exists
        if model_path.exists() && model_path.metadata().map(|m| m.len() > 1_000_000).unwrap_or(false) {
            progress_callback(format!("模型已在缓存中: {}", model_path.display()));
            return Ok(model_path.to_string_lossy().to_string());
        }

        if let Some(parent) = model_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create cache dir: {}", e))?;
        }

        progress_callback(format!("正在下载模型 {}...", model_name));

        let mut last_error = String::new();
        for url in &download_urls {
            progress_callback(format!("尝试从 {}", url));

            match self
                .download_file_with_progress(url, &model_path, |msg| progress_callback(msg))
                .await
            {
                Ok(_) => {
                    progress_callback("正在验证模型...".to_string());
                    if self.validate_model_file(&model_path).await {
                        progress_callback(format!("模型已保存到: {}", model_path.display()));
                        return Ok(model_path.to_string_lossy().to_string());
                    } else {
                        last_error = "模型验证失败".to_string();
                        let _ = std::fs::remove_file(&model_path);
                    }
                }
                Err(e) => {
                    last_error = e;
                }
            }
        }

        Err(format!("下载失败: {}", last_error))
    }

    async fn download_file_with_progress(
        &self,
        url: &str,
        path: &std::path::PathBuf,
        progress_callback: impl Fn(String),
    ) -> Result<(), String> {
        use tokio::io::AsyncWriteExt;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("网络请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP错误: {}", response.status()));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut file = tokio::fs::File::create(path)
            .await
            .map_err(|e| format!("Failed to create file: {}", e))?;

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("下载出错: {}", e))?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                .await
                .map_err(|e| format!("写入文件失败: {}", e))?;
            file.flush()
                .await
                .map_err(|e| format!("刷新文件失败: {}", e))?;
            downloaded += chunk.len() as u64;

            if total_size > 0 {
                let percent = (downloaded as f64 / total_size as f64 * 100.0) as u32;
                progress_callback(format!(
                    "下载进度: {}% ({}/{})",
                    percent,
                    Self::format_bytes(downloaded),
                    Self::format_bytes(total_size)
                ));
            }
        }

        Ok(())
    }

    async fn validate_model_file(&self, path: &std::path::PathBuf) -> bool {
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() < 1_000_000 {
                eprintln!("[Trainer] Model file too small: {}", metadata.len());
                return false;
            }
        }
        true
    }

    fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

impl Default for TrainerService {
    fn default() -> Self {
        Self::new()
    }
}
