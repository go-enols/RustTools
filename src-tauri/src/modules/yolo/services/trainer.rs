use crate::modules::yolo::models::training::{TrainingMetrics, TrainingRequest, TrainingStatus};
use rand::Rng;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpListener;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::{Duration, Instant};
use tokio_stream::StreamExt;

pub struct TrainerService {
    processes: Arc<RwLock<HashMap<String, TrainingHandle>>>,
}

struct TrainingHandle {
    status: TrainingStatus,
    stop_tx: Option<oneshot::Sender<()>>,
    model_path: Option<String>,
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
        }
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
                                            let model_path = processes
                                                .read()
                                                .await
                                                .get(&process_id)
                                                .and_then(|h| h.model_path.clone());
                                            let _ = event_tx.send(TrainingEvent::Complete {
                                                training_id: process_id.clone(),
                                                model_path,
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

        if model_path.exists() && model_path.metadata().map(|m| m.len() > 1_000_000).unwrap_or(false) {
            Ok((true, Some(model_path.to_string_lossy().to_string())))
        } else {
            Ok((false, None))
        }
    }

    pub async fn download_model(
        &self,
        model_name: &str,
        progress_callback: impl Fn(String),
    ) -> Result<String, String> {
        use std::path::PathBuf;

        let download_urls = [
            format!(
                "https://mirror.ghproxy.com/https://github.com/ultralytics/assets/releases/download/v0.0.0/{}",
                model_name
            ),
            format!(
                "https://hf-mirror.com/ultralytics/assets/releases/download/v0.0.0/{}",
                model_name
            ),
            format!(
                "https://github.com/ultralytics/assets/releases/download/v0.0.0/{}",
                model_name
            ),
        ];

        let cache_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|h| PathBuf::from(h).join(".cache").join("ultralytics"))
            .unwrap_or_else(|_| PathBuf::from(".cache/ultralytics"));

        let model_path = cache_dir.join(model_name);

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
