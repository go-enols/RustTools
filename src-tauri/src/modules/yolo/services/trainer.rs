use crate::modules::yolo::models::training::{TrainingMetrics, TrainingRequest, TrainingStatus};
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
}

impl TrainerService {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn generate_id() -> String {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 16] = rng.gen();
        hex::encode(bytes)
    }

    /// Find and start the Python sidecar process
    async fn start_sidecar(&self) -> Result<Child, String> {
        let script_paths = [
            "scripts/yolo_server.py",
            "src-tauri/scripts/yolo_server.py",
        ];

        let script_path = script_paths
            .iter()
            .find(|p| std::path::Path::new(p).exists())
            .ok_or_else(|| "Python sidecar script not found".to_string())?;

        let python_cmd = if cfg!(windows) { "python" } else { "python3" };

        let child = Command::new(python_cmd)
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())  // Print Python stderr to console
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to start Python sidecar: {}", e))?;

        Ok(child)
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

        let mut child = self.start_sidecar().await?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to take stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to take stdout".to_string())?;

        // Build config
        let config = serde_json::json!({
            "project_path": project_path,
            "base_model": request.base_model,
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
                        // Send stop command to Python
                        eprintln!("[Trainer] Sending stop command to Python...");
                        // Note: Can't easily write to stdin here, so just kill
                        if let Some(h) = processes.write().await.get_mut(&process_id) {
                            let _ = h.child.kill().await;
                            h.status.running = false;
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
                let _ = h.child.wait().await;
            }
        });

        Ok(training_id)
    }

    pub async fn stop_training(&self, training_id: &str) -> Result<(), String> {
        let mut processes = self.processes.write().await;
        if let Some(handle) = processes.get_mut(training_id) {
            if let Some(tx) = handle.stop_tx.take() {
                let _ = tx.send(()).await;
            }
            handle.status.running = false;
            handle.status.paused = false;
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
        self.processes
            .read()
            .await
            .get(training_id)
            .map(|h| h.status.clone())
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
