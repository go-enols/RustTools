use crate::modules::yolo::models::training::{TrainingMetrics, TrainingRequest, TrainingStatus};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpListener;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::{Duration, Instant};
use tokio_stream::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainedModelInfo {
    pub id: String,
    pub project_name: String,
    pub project_path: String,
    pub yolo_version: String,
    pub model_size: String,
    pub best_epoch: u32,
    pub total_epochs: u32,
    pub map50: f32,
    pub map50_95: f32,
    pub model_path: String,
    pub created_at: String,
}

pub struct TrainerService {
    processes: Arc<RwLock<HashMap<String, TrainingHandle>>>,
    trained_models: Arc<RwLock<Vec<TrainedModelInfo>>>,
}

struct TrainingHandle {
    status: TrainingStatus,
    stop_tx: Option<oneshot::Sender<()>>,
    model_path: Option<String>,
    project_name: String,
    project_path: String,
    yolo_version: String,
    total_epochs: u32,
}

#[derive(Debug, Clone)]
pub enum TrainingEvent {
    Started {
        training_id: String,
        total_epochs: u32,
        cuda_available: bool,
        cuda_version: Option<String>,
    },
    BatchProgress {
        training_id: String,
        epoch: u32,
        total_epochs: u32,
        batch: u32,
        total_batches: u32,
        box_loss: f32,
        cls_loss: f32,
        dfl_loss: f32,
        learning_rate: f32,
    },
    Progress {
        training_id: String,
        status: TrainingStatus,
    },
    Complete {
        training_id: String,
        model_path: Option<String>,
    },
    Error {
        training_id: String,
        error: String,
    },
    Stopped {
        training_id: String,
    },
}

impl TrainerService {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            trained_models: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    fn get_models_dir() -> PathBuf {
        let cache_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|h| PathBuf::from(h).join(".cache").join("rust-tools").join("models"))
            .unwrap_or_else(|_| PathBuf::from(".cache/rust-tools/models"));
        cache_dir
    }
    
    pub async fn save_trained_model(&self, model_info: TrainedModelInfo) -> Result<(), String> {
        let models_dir = Self::get_models_dir();
        fs::create_dir_all(&models_dir)
            .map_err(|e| format!("Failed to create models directory: {}", e))?;
        
        let models_file = models_dir.join("trained_models.json");
        let mut models = self.trained_models.write().await;
        models.push(model_info.clone());
        
        let json = serde_json::to_string_pretty(&*models)
            .map_err(|e| format!("Failed to serialize models: {}", e))?;
        fs::write(&models_file, json)
            .map_err(|e| format!("Failed to save models: {}", e))?;
        
        eprintln!("[Trainer] Saved trained model: {} to {:?}", model_info.project_name, models_file);
        Ok(())
    }
    
    pub async fn get_trained_models(&self) -> Result<Vec<TrainedModelInfo>, String> {
        let models_dir = Self::get_models_dir();
        let models_file = models_dir.join("trained_models.json");
        
        if !models_file.exists() {
            eprintln!("[Trainer] No trained models file found");
            return Ok(Vec::new());
        }
        
        let content = fs::read_to_string(&models_file)
            .map_err(|e| format!("Failed to read models file: {}", e))?;
        
        let models: Vec<TrainedModelInfo> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse models: {}", e))?;
        
        *self.trained_models.write().await = models.clone();
        eprintln!("[Trainer] Loaded {} trained models", models.len());
        Ok(models)
    }
    
    pub async fn delete_trained_model(&self, model_id: &str) -> Result<(), String> {
        let models_dir = Self::get_models_dir();
        let models_file = models_dir.join("trained_models.json");
        
        let mut models = self.trained_models.write().await;
        let initial_len = models.len();
        models.retain(|m| m.id != model_id);
        
        if models.len() == initial_len {
            return Err("Model not found".to_string());
        }
        
        let json = serde_json::to_string_pretty(&*models)
            .map_err(|e| format!("Failed to serialize models: {}", e))?;
        fs::write(&models_file, json)
            .map_err(|e| format!("Failed to save models: {}", e))?;
        
        eprintln!("[Trainer] Deleted trained model: {}", model_id);
        Ok(())
    }

    fn generate_id() -> String {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 16] = rng.gen();
        hex::encode(bytes)
    }

    async fn start_sidecar(&self, tcp_port: u16) -> Result<Child, String> {
        let search_paths: [std::path::PathBuf; 3] = [
            std::path::PathBuf::from("src-tauri/scripts/yolo_server.py"),
            std::path::PathBuf::from("scripts/yolo_server.py"),
            std::path::PathBuf::from(
                std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default(),
            )
            .join("scripts/yolo_server.py"),
        ];

        let script_path = search_paths
            .iter()
            .find(|p| p.exists())
            .ok_or_else(|| "Python sidecar script not found in any location".to_string())?;

        eprintln!("[Trainer] Using script path: {:?}", script_path);

        let python_cmd = if cfg!(windows) { "python" } else { "python3" };

        let child = Command::new(python_cmd)
            .arg(&script_path)
            .arg(tcp_port.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to start Python sidecar: {}", e))?;

        Ok(child)
    }

    pub async fn start_training(
        &self,
        project_path: String,
        request: TrainingRequest,
        event_tx: mpsc::UnboundedSender<TrainingEvent>,
    ) -> Result<String, String> {
        let training_id = Self::generate_id();
        self.start_training_pipe(training_id, project_path, request, event_tx)
            .await
    }

    async fn start_training_pipe(
        &self,
        training_id: String,
        project_path: String,
        request: TrainingRequest,
        event_tx: mpsc::UnboundedSender<TrainingEvent>,
    ) -> Result<String, String> {
        eprintln!("[Trainer] Starting training via TCP+stdin...");
        eprintln!("[Trainer] Training ID: {}", training_id);
        eprintln!("[Trainer] Project path: {}", project_path);

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Failed to create TCP listener: {}", e))?;
        let tcp_port = listener.local_addr().unwrap().port();
        eprintln!("[Trainer] TCP listener on port {}", tcp_port);

        eprintln!("[Trainer] Starting Python sidecar...");
        let mut child = self.start_sidecar(tcp_port).await?;
        eprintln!("[Trainer] Python sidecar started with PID: {:?}", child.id());

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to take stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to take stdout".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Failed to take stderr".to_string())?;

        eprintln!("[Trainer] Waiting for Python TCP connection (timeout 30s)...");
        let (tcp_stream, addr) = tokio::time::timeout(Duration::from_secs(30), listener.accept())
            .await
            .map_err(|_| "Timeout waiting for Python TCP connection (30s)".to_string())?
            .map_err(|e| format!("Failed to accept TCP connection: {}", e))?;
        eprintln!("[Trainer] Python TCP connection accepted from {}", addr);

        let stdout_drain_id = training_id.clone();
        tokio::spawn(async move {
            let mut stdout_drain = BufReader::new(stdout).lines();
            loop {
                match stdout_drain.next_line().await {
                    Ok(Some(line)) => {
                        eprintln!("[YOLO:{}:stdout] {}", stdout_drain_id, line);
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        });

        let config = serde_json::json!({
            "project_path": project_path,
            "base_model": request.base_model,
            "epochs": request.epochs,
            "batch_size": request.batch_size,
            "image_size": request.image_size,
            "device": request.device_id,
            "workers": request.workers,
            "optimizer": request.optimizer,
            "lr0": request.lr0,
            "lrf": request.lrf,
            "momentum": request.momentum,
            "weight_decay": request.weight_decay,
            "warmup_epochs": request.warmup_epochs,
            "warmup_bias_lr": request.warmup_bias_lr,
            "warmup_momentum": request.warmup_momentum,
            "hsv_h": request.hsv_h,
            "hsv_s": request.hsv_s,
            "hsv_v": request.hsv_v,
            "translate": request.translate,
            "scale": request.scale,
            "shear": request.shear,
            "perspective": request.perspective,
            "flipud": request.flipud,
            "fliplr": request.fliplr,
            "mosaic": request.mosaic,
            "mixup": request.mixup,
            "copy_paste": request.copy_paste,
            "close_mosaic": request.close_mosaic,
            "rect": request.rect,
            "cos_lr": request.cos_lr,
            "single_cls": request.single_cls,
            "amp": request.amp,
            "save_period": request.save_period,
            "cache": request.cache,
        });

        let start_cmd = serde_json::json!({
            "type": "start",
            "config": config,
        });

        eprintln!("[Trainer] Sending start command via stdin...");
        let mut stdin_writer = BufWriter::new(stdin);
        let cmd_str = serde_json::to_string(&start_cmd).unwrap() + "\n";
        eprintln!("[Trainer] Start command length: {} bytes", cmd_str.len());
        stdin_writer
            .write_all(cmd_str.as_bytes())
            .await
            .map_err(|e| format!("Failed to write start command: {}", e))?;
        stdin_writer
            .flush()
            .await
            .map_err(|e| format!("Failed to flush stdin: {}", e))?;
        eprintln!("[Trainer] Start command sent successfully");

        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        
        // Extract project name from path
        let project_name = std::path::Path::new(&project_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        
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
            model_path: None,
            project_name: project_name.clone(),
            project_path: project_path.clone(),
            yolo_version: request.base_model.clone(),
            total_epochs: request.epochs,
        };

        self.processes
            .write()
            .await
            .insert(training_id.clone(), handle);

        let stderr_process_id = training_id.clone();
        tokio::spawn(async move {
            let mut stderr_reader = BufReader::new(stderr).lines();
            loop {
                match stderr_reader.next_line().await {
                    Ok(Some(line)) => {
                        eprintln!("[YOLO:{}:stderr] {}", stderr_process_id, line);
                    }
                    Ok(None) => break,
                    Err(error) => {
                        eprintln!(
                            "[Trainer] Failed to read stderr for {}: {}",
                            stderr_process_id, error
                        );
                        break;
                    }
                }
            }
        });

        let process_id = training_id.clone();
        let processes = Arc::clone(&self.processes);
        let trained_models = Arc::clone(&self.trained_models);
        let trainer_service = Arc::new(self.clone());
        let mut tcp_reader = BufReader::new(tcp_stream).lines();
        let mut stop_rx = stop_rx;
        let mut stop_requested = false;
        let mut stop_timer_enabled = false;
        let mut terminal_event_sent = false;

        tokio::spawn(async move {
            let stop_sleep = tokio::time::sleep(Duration::from_secs(60 * 60 * 24));
            tokio::pin!(stop_sleep);
            loop {
                tokio::select! {
                    _ = &mut stop_rx, if !stop_requested => {
                        eprintln!("[Trainer] Sending stop command to Python...");
                        stop_requested = true;
                        stop_timer_enabled = true;
                        stop_sleep
                            .as_mut()
                            .reset(Instant::now() + Duration::from_secs(5));

                        let stop_cmd = serde_json::json!({ "type": "stop" });
                        let stop_payload = serde_json::to_string(&stop_cmd).unwrap() + "\n";
                        if let Err(error) = stdin_writer.write_all(stop_payload.as_bytes()).await {
                            eprintln!("[Trainer] Failed to write stop command: {}", error);
                            let _ = child.kill().await;
                            if let Some(h) = processes.write().await.get_mut(&process_id) {
                                h.status.running = false;
                                h.status.paused = false;
                            }
                            let _ = event_tx.send(TrainingEvent::Stopped {
                                training_id: process_id.clone(),
                            });
                            break;
                        }

                        if let Err(error) = stdin_writer.flush().await {
                            eprintln!("[Trainer] Failed to flush stop command: {}", error);
                            let _ = child.kill().await;
                            if let Some(h) = processes.write().await.get_mut(&process_id) {
                                h.status.running = false;
                                h.status.paused = false;
                            }
                            let _ = event_tx.send(TrainingEvent::Stopped {
                                training_id: process_id.clone(),
                            });
                            break;
                        }
                    }
                    _ = &mut stop_sleep, if stop_timer_enabled => {
                        eprintln!("[Trainer] Stop timed out, killing Python process...");
                        let _ = child.kill().await;
                        if let Some(h) = processes.write().await.get_mut(&process_id) {
                            h.status.running = false;
                            h.status.paused = false;
                        }
                        let _ = event_tx.send(TrainingEvent::Stopped {
                            training_id: process_id.clone(),
                        });
                        break;
                    }
                    line = tcp_reader.next_line() => {
                        match line {
                            Ok(Some(line)) => {
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
                                                    metrics.val_box_loss =
                                                        data.get("val_box_loss").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                    metrics.val_cls_loss =
                                                        data.get("val_cls_loss").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                    metrics.val_dfl_loss =
                                                        data.get("val_dfl_loss").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                    metrics.map50 =
                                                        data.get("mAP50").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                    metrics.map50_95 =
                                                        data.get("mAP50-95").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                    metrics.learning_rate =
                                                        data.get("learning_rate").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

                                                    eprintln!("[Trainer] Progress updated: epoch={}, metrics box_loss={}", epoch, metrics.train_box_loss);
                                                    h.status.metrics = metrics;
                                                    eprintln!("[Trainer] Sending Progress event to channel...");
                                                    let result = event_tx.send(TrainingEvent::Progress {
                                                        training_id: process_id.clone(),
                                                        status: h.status.clone(),
                                                    });
                                                    if result.is_err() {
                                                        eprintln!("[Trainer] WARNING: Failed to send Progress event: {:?}", result.err());
                                                    } else {
                                                        eprintln!("[Trainer] Progress event sent successfully");
                                                    }
                                                }
                                            }
                                        }
                                        "started" => {
                                            eprintln!("[Trainer] Training started event received");
                                            let total_epochs = resp
                                                .get("data")
                                                .and_then(|data| data.get("total_epochs"))
                                                .and_then(|v| v.as_u64())
                                                .unwrap_or(0) as u32;
                                            let cuda_available = resp
                                                .get("data")
                                                .and_then(|data| data.get("cuda_available"))
                                                .and_then(|v| v.as_bool())
                                                .unwrap_or(false);
                                            let cuda_version = resp
                                                .get("data")
                                                .and_then(|data| data.get("cuda_version"))
                                                .and_then(|v| v.as_str())
                                                .map(String::from);
                                            if let Some(h) = processes.write().await.get_mut(&process_id) {
                                                h.status.total_epochs = total_epochs;
                                            }
                                            let _ = event_tx.send(TrainingEvent::Started {
                                                training_id: process_id.clone(),
                                                total_epochs,
                                                cuda_available,
                                                cuda_version,
                                            });
                                        }
                                        "batch_progress" => {
                                            if let Some(data) = resp.get("data") {
                                                let epoch = data.get("epoch").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                                let total_epochs = data.get("total_epochs").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                                let batch = data.get("batch").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                                let total_batches = data.get("total_batches").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                                let box_loss = data.get("box_loss").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                let cls_loss = data.get("cls_loss").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                let dfl_loss = data.get("dfl_loss").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                                let learning_rate = data.get("learning_rate").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

                                                if let Some(h) = processes.write().await.get_mut(&process_id) {
                                                    h.status.epoch = epoch;
                                                    h.status.total_epochs = total_epochs;
                                                    if total_epochs > 0 && total_batches > 0 {
                                                        let epoch_frac = (epoch - 1) as f32 / total_epochs as f32;
                                                        let batch_frac = batch as f32 / total_batches as f32 / total_epochs as f32;
                                                        h.status.progress_percent = (epoch_frac + batch_frac) * 100.0;
                                                    }
                                                    h.status.metrics.train_box_loss = box_loss;
                                                    h.status.metrics.train_cls_loss = cls_loss;
                                                    h.status.metrics.train_dfl_loss = dfl_loss;
                                                    h.status.metrics.learning_rate = learning_rate;
                                                }

                                                eprintln!("[Trainer] Sending BatchProgress event (batch={}/{})...", batch, total_batches);
                                                match event_tx.send(TrainingEvent::BatchProgress {
                                                    training_id: process_id.clone(),
                                                    epoch,
                                                    total_epochs,
                                                    batch,
                                                    total_batches,
                                                    box_loss,
                                                    cls_loss,
                                                    dfl_loss,
                                                    learning_rate,
                                                }) {
                                                    Ok(_) => eprintln!("[Trainer] BatchProgress event sent"),
                                                    Err(e) => eprintln!("[Trainer] WARNING: BatchProgress send failed: {:?}", e),
                                                }
                                            }
                                        }
                                        "complete" => {
                                            if let Some(h) = processes.write().await.get_mut(&process_id) {
                                                h.status.running = false;
                                                h.status.progress_percent = 100.0;
                                                if let Some(data) = resp.get("data") {
                                                    if let Some(model_path) = data.get("model_path").and_then(|v| v.as_str()) {
                                                        h.model_path = Some(model_path.to_string());
                                                    }
                                                }
                                            }
                                            eprintln!("[Trainer] Training complete event received");
                                            
                                            // Get training info - extract all values before dropping the lock
                                            let model_path_clone: Option<String>;
                                            let project_name_clone: String;
                                            let project_path_clone: String;
                                            let yolo_version_clone: String;
                                            let total_epochs_val: u32;
                                            let best_epoch_val: u32;
                                            let map50_val: f32;
                                            let map50_95_val: f32;
                                            
                                            {
                                                let guard = processes.read().await;
                                                if let Some(h) = guard.get(&process_id) {
                                                    model_path_clone = h.model_path.clone();
                                                    project_name_clone = h.project_name.clone();
                                                    project_path_clone = h.project_path.clone();
                                                    yolo_version_clone = h.yolo_version.clone();
                                                    total_epochs_val = h.total_epochs;
                                                    best_epoch_val = h.status.epoch;
                                                    map50_val = h.status.metrics.map50;
                                                    map50_95_val = h.status.metrics.map50_95;
                                                } else {
                                                    model_path_clone = None;
                                                    project_name_clone = String::new();
                                                    project_path_clone = String::new();
                                                    yolo_version_clone = String::new();
                                                    total_epochs_val = 0;
                                                    best_epoch_val = 0;
                                                    map50_val = 0.0;
                                                    map50_95_val = 0.0;
                                                }
                                            }
                                            
                                            // Save trained model if we have a model path
                                            if let Some(ref model_path_str) = model_path_clone {
                                                let model_info = TrainedModelInfo {
                                                    id: rand::random::<u64>().to_string(),
                                                    project_name: project_name_clone.clone(),
                                                    project_path: project_path_clone.clone(),
                                                    yolo_version: yolo_version_clone.clone(),
                                                    model_size: "0".to_string(),
                                                    best_epoch: best_epoch_val,
                                                    total_epochs: total_epochs_val,
                                                    map50: map50_val,
                                                    map50_95: map50_95_val,
                                                    model_path: model_path_str.clone(),
                                                    created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                                                };
                                                
                                                let trainer_clone = trainer_service.clone();
                                                tokio::spawn(async move {
                                                    if let Err(e) = trainer_clone.save_trained_model(model_info).await {
                                                        eprintln!("[Trainer] Failed to save trained model: {}", e);
                                                    }
                                                });
                                            }
                                            
                                            let _ = event_tx.send(TrainingEvent::Complete {
                                                training_id: process_id.clone(),
                                                model_path: model_path_clone,
                                            });
                                            terminal_event_sent = true;
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
                                            let _ = event_tx.send(TrainingEvent::Error {
                                                training_id: process_id.clone(),
                                                error: error_msg,
                                            });
                                            terminal_event_sent = true;
                                            break;
                                        }
                                        "stopped" => {
                                            if let Some(h) = processes.write().await.get_mut(&process_id) {
                                                h.status.running = false;
                                                h.status.paused = false;
                                            }
                                            eprintln!("[Trainer] Training stopped event received");
                                            let _ = event_tx.send(TrainingEvent::Stopped {
                                                training_id: process_id.clone(),
                                            });
                                            terminal_event_sent = true;
                                            break;
                                        }
                                        "connected" => {
                                            eprintln!("[Trainer] Python TCP connected, pid={}", 
                                                resp.get("data").and_then(|d| d.get("pid")).and_then(|v| v.as_u64()).unwrap_or(0));
                                        }
                                        "heartbeat" => {
                                            if let Some(data) = resp.get("data") {
                                                let running = data.get("running").and_then(|v| v.as_bool()).unwrap_or(false);
                                                let epoch = data.get("epoch").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                                let batch = data.get("batch").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                                let total_batches = data.get("total_batches").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                                eprintln!("[Trainer] Heartbeat: running={}, epoch={}, batch={}/{}", running, epoch, batch, total_batches);
                                            }
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
                                eprintln!("[Trainer] TCP connection closed");
                                break;
                            }
                        }
                    }
                    exit = child.wait() => {
                        match exit {
                            Ok(status) => {
                                eprintln!("[Trainer] Python process exited: {}", status);
                            }
                            Err(error) => {
                                eprintln!("[Trainer] Failed waiting Python process: {}", error);
                            }
                        }
                        if let Some(h) = processes.write().await.get_mut(&process_id) {
                            h.status.running = false;
                            h.status.paused = false;
                        }
                        if !terminal_event_sent {
                            let error = processes
                                .read()
                                .await
                                .get(&process_id)
                                .and_then(|h| h.status.error.clone());
                            if let Some(error) = error {
                                let _ = event_tx.send(TrainingEvent::Error {
                                    training_id: process_id.clone(),
                                    error,
                                });
                            } else if stop_requested {
                                let _ = event_tx.send(TrainingEvent::Stopped {
                                    training_id: process_id.clone(),
                                });
                            }
                        }
                        return;
                    }
                }
            }

            if let Err(error) = child.wait().await {
                eprintln!("[Trainer] Failed final wait for Python process: {}", error);
            }
            if let Some(h) = processes.write().await.get_mut(&process_id) {
                h.status.running = false;
                h.status.paused = false;
            }
        });

        Ok(training_id)
    }

    pub async fn stop_training(&self, training_id: &str) -> Result<(), String> {
        let stop_tx = {
            let mut processes = self.processes.write().await;
            let handle = processes
                .get_mut(training_id)
                .ok_or_else(|| "Training not found".to_string())?;
            handle.status.paused = false;
            handle.stop_tx.take()
        };

        if let Some(tx) = stop_tx {
            let _ = tx.send(());
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

    pub async fn check_model(&self, model_name: &str) -> Result<(bool, Option<String>), String> {
        let cache_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|h| std::path::PathBuf::from(h).join(".cache").join("ultralytics"))
            .unwrap_or_else(|_| std::path::PathBuf::from(".cache/ultralytics"));

        let model_path = cache_dir.join(model_name);

        eprintln!("[Model] 检查模型是否存在: {}", model_name);
        eprintln!("[Model] 模型路径: {:?}", model_path);
        eprintln!("[Model] 缓存目录存在: {}", cache_dir.exists());

        if model_path.exists() {
            let metadata = model_path.metadata();
            match metadata {
                Ok(m) => {
                    let size = m.len();
                    eprintln!("[Model] 模型文件存在, 大小: {} bytes ({} MB)", size, size / 1024 / 1024);
                    if size > 1_000_000 {
                        eprintln!("[Model] 模型有效，返回路径");
                        Ok((true, Some(model_path.to_string_lossy().to_string())))
                    } else {
                        eprintln!("[Model] 模型文件过小 (< 1MB)，视为无效");
                        Ok((false, None))
                    }
                }
                Err(e) => {
                    eprintln!("[Model] 读取模型文件元数据失败: {}", e);
                    Ok((false, None))
                }
            }
        } else {
            eprintln!("[Model] 模型文件不存在");
            Ok((false, None))
        }
    }

    pub async fn download_model(
        &self,
        model_name: &str,
        progress_callback: impl Fn(String),
    ) -> Result<String, String> {
        use std::path::PathBuf;

        eprintln!("[Download] 开始下载模型: {}", model_name);

        // 根据模型名称确定对应的 HuggingFace 仓库
        // 不同版本的 YOLO 存储在不同的仓库中
        // 分类、分割等变体模型通常也在同一仓库
        let (repo_owner, repo_name) = if model_name.starts_with("yolo26") {
            ("Ultralytics", "YOLO26")
        } else if model_name.starts_with("yolo12") {
            ("Ultralytics", "YOLO12")
        } else if model_name.starts_with("yolo11") {
            ("Ultralytics", "YOLO11")
        } else if model_name.starts_with("yolo10") || model_name.starts_with("yolov10") {
            ("Ultralytics", "YOLOv10")
        } else if model_name.starts_with("yolov9") {
            ("WongKinYiu", "yolov9")
        } else if model_name.starts_with("yolov6") {
            ("meituan", "YOLOv6")
        } else if model_name.starts_with("yolov8") {
            ("Ultralytics", "assets")  // YOLOv8 在 assets 仓库
        } else if model_name.starts_with("yolov5") {
            ("Ultralytics", "assets")  // YOLOv5 在 assets 仓库
        } else {
            // 默认使用 assets 仓库
            ("Ultralytics", "assets")
        };

        // 模型下载镜像源（按优先级排序）
        // 1. HuggingFace 官方地址
        // 2. HuggingFace 镜像
        // 3. GitHub 直连
        let mut download_urls = Vec::new();
        
        // 镜像1：HuggingFace 官方（推荐，速度快且稳定）
        download_urls.push(format!(
            "https://huggingface.co/{}/{}/resolve/main/{}",
            repo_owner, repo_name, model_name
        ));
        
        // 镜像2：hf-mirror（国内加速）
        download_urls.push(format!(
            "https://hf-mirror.com/{}/{}/resolve/main/{}",
            repo_owner, repo_name, model_name
        ));
        
        // 镜像3：GitHub 官方 releases（仅适用于 assets 仓库中的模型）
        if repo_name == "assets" {
            download_urls.push(format!(
                "https://github.com/ultralytics/assets/releases/download/v0.0.0/{}",
                model_name
            ));
        }

        eprintln!("[Download] 使用仓库: {}/{}", repo_owner, repo_name);

        let cache_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|h| PathBuf::from(h).join(".cache").join("ultralytics"))
            .unwrap_or_else(|_| PathBuf::from(".cache/ultralytics"));

        eprintln!("[Download] 缓存目录: {:?}", cache_dir);

        let model_path = cache_dir.join(model_name);

        // Check if already downloaded
        if model_path.exists() {
            if let Ok(metadata) = model_path.metadata() {
                let size = metadata.len();
                if size > 1_000_000 {
                    eprintln!("[Download] 模型已在缓存中: {} ({} MB)", model_path.display(), size / 1024 / 1024);
                    progress_callback(format!("模型已在缓存中: {}", model_path.display()));
                    return Ok(model_path.to_string_lossy().to_string());
                }
            }
        }

        if let Some(parent) = model_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create cache dir: {}", e))?;
            eprintln!("[Download] 已创建缓存目录: {:?}", parent);
        }

        progress_callback(format!("正在下载模型 {}...", model_name));

        let mut last_error = String::new();
        for (index, url) in download_urls.iter().enumerate() {
            progress_callback(format!("尝试从镜像 {} 下载...", index + 1));
            eprintln!("[Download] 尝试 {}: {}", index + 1, url);

            match self
                .download_file_with_progress(url, &model_path, |msg| progress_callback(msg))
                .await
            {
                Ok(_) => {
                    eprintln!("[Download] 下载成功，正在验证...");
                    progress_callback("正在验证模型...".to_string());
                    if self.validate_model_file(&model_path).await {
                        eprintln!("[Download] 模型验证通过: {}", model_path.display());
                        progress_callback(format!("模型已保存到: {}", model_path.display()));
                        return Ok(model_path.to_string_lossy().to_string());
                    } else {
                        last_error = "模型验证失败".to_string();
                        eprintln!("[Download] 模型验证失败，删除文件");
                        let _ = std::fs::remove_file(&model_path);
                    }
                }
                Err(e) => {
                    last_error = e.clone();
                    eprintln!("[Download] 下载失败: {}", e);
                }
            }
        }

        eprintln!("[Download] 所有镜像下载失败");
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
            .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("网络连接失败: {}", e))?;

        let status = response.status();
        
        // 处理HTTP错误状态码
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(format!("模型文件不存在 (404)"))
        }
        
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err(format!("访问被拒绝 (403)"))
        }
        
        if !status.is_success() {
            return Err(format!("HTTP错误 ({}): 服务器返回错误状态", status));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut file = tokio::fs::File::create(path)
            .await
            .map_err(|e| format!("创建文件失败: {}", e))?;

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("下载中断: {}", e))?;
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

impl Clone for TrainerService {
    fn clone(&self) -> Self {
        Self {
            processes: Arc::clone(&self.processes),
            trained_models: Arc::clone(&self.trained_models),
        }
    }
}

impl Default for TrainerService {
    fn default() -> Self {
        Self::new()
    }
}
