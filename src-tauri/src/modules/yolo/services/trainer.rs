use crate::modules::yolo::models::training::{TrainingMetrics, TrainingRequest, TrainingStatus};
use rand::Rng;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, RwLock};

pub struct TrainerService {
    processes: Arc<RwLock<HashMap<String, TrainingHandle>>>,
}

struct TrainingHandle {
    status: TrainingStatus,
    stop_tx: Option<mpsc::Sender<()>>,
    child_id: u32,
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

    pub async fn start_training(
        &self,
        project_path: String,
        request: TrainingRequest,
    ) -> Result<String, String> {
        let training_id = Self::generate_id();
        let (stop_tx, stop_rx) = mpsc::channel::<()>(1);

        // Build YOLO command
        let mut cmd = Command::new("yolo");
        cmd.args([
            "detect", "train",
            &format!("data={}", project_path),
            &format!("model={}", request.base_model),
            &format!("epochs={}", request.epochs),
            &format!("batch={}", request.batch_size),
            &format!("imgsz={}", request.image_size),
            &format!("device={}", request.device_id),
            &format!("workers={}", request.workers),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

        let mut child = cmd.spawn().map_err(|e| format!("Failed to start training: {}", e))?;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

        let child_id = child.id().ok_or("Failed to get child id")?;

        let handle = TrainingHandle {
            status: TrainingStatus {
                running: true,
                paused: false,
                epoch: 0,
                total_epochs: request.epochs,
                progress_percent: 0.0,
                metrics: TrainingMetrics::default(),
            },
            stop_tx: Some(stop_tx),
            child_id,
        };

        self.processes.write().await.insert(training_id.clone(), handle);

        // Spawn log reader task
        let process_id = training_id.clone();
        let processes = Arc::clone(&self.processes);

        tokio::spawn(async move {
            let mut stdout_reader = BufReader::new(stdout).lines();
            let mut stderr_reader = BufReader::new(stderr).lines();
            let mut stop_rx = stop_rx;

            loop {
                tokio::select! {
                    _ = stop_rx.recv() => {
                        break;
                    }
                    line = stdout_reader.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                if let Some(proc) = processes.write().await.get_mut(&process_id) {
                                    Self::parse_output(&line, &mut proc.status);
                                }
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                    line = stderr_reader.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                if let Some(proc) = processes.write().await.get_mut(&process_id) {
                                    if proc.status.epoch == 0 {
                                        // Show errors during setup
                                        eprintln!("[YOLO] {}", line);
                                    }
                                }
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                }
            }

            // Training ended
            if let Some(proc) = processes.write().await.get_mut(&process_id) {
                proc.status.running = false;
                if let Some(tx) = proc.stop_tx.take() {
                    let _ = tx.send(()).await;
                }
            }
        });

        Ok(training_id)
    }

    pub async fn stop_training(&self, training_id: &str) -> Result<(), String> {
        let mut processes = self.processes.write().await;
        if let Some(mut handle) = processes.get_mut(training_id) {
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

    fn parse_output(line: &str, status: &mut TrainingStatus) {
        // Parse YOLO output lines
        // Format: "Epoch 1/50: 100%|██████████| 100/100 [00:05<00:00]"
        if line.contains("Epoch") {
            if let Some((epoch, total)) = Self::extract_epoch_info(line) {
                status.epoch = epoch;
                status.total_epochs = total;
                status.progress_percent = (epoch as f32 / total as f32) * 100.0;
            }
        }

        // Parse metrics - YOLO outputs metrics at end of epoch
        // Format: "metrics/mAP50(B): 0.85  metrics/mAP50-95(B): 0.65"
        if let Some(v) = Self::extract_float(line, "box_loss") {
            status.metrics.train_box_loss = v;
        }
        if let Some(v) = Self::extract_float(line, "cls_loss") {
            status.metrics.train_cls_loss = v;
        }
        if let Some(v) = Self::extract_float(line, "dfl_loss") {
            status.metrics.train_dfl_loss = v;
        }
        if let Some(v) = Self::extract_float(line, "mAP50") {
            status.metrics.map50 = v;
        }
        if let Some(v) = Self::extract_float(line, "mAP50-95") {
            status.metrics.map50_95 = v;
        }
        if let Some(v) = Self::extract_float(line, "precision") {
            status.metrics.precision = v;
        }
        if let Some(v) = Self::extract_float(line, "recall") {
            status.metrics.recall = v;
        }
    }

    fn extract_epoch_info(line: &str) -> Option<(u32, u32)> {
        // Try to parse "Epoch 1/50" pattern
        let parts: Vec<&str> = line.split_whitespace().collect();
        for (i, part) in parts.iter().enumerate() {
            if *part == "Epoch" && i + 1 < parts.len() {
                let epoch_part = parts[i + 1]; // "1/50"
                let nums: Vec<&str> = epoch_part.split('/').collect();
                if nums.len() == 2 {
                    if let (Ok(current), Ok(total)) = (nums[0].parse::<u32>(), nums[1].parse::<u32>()) {
                        return Some((current, total));
                    }
                }
            }
        }
        None
    }

    fn extract_float(line: &str, metric: &str) -> Option<f32> {
        // Simple float extraction - looks for "metric: value" pattern
        if let Some(pos) = line.find(metric) {
            let rest = &line[pos..];
            if let Some(colon_pos) = rest.find(':') {
                let value_str = &rest[colon_pos + 1..];
                let value = value_str
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<f32>().ok());
                return value;
            }
        }
        None
    }
}

impl Default for TrainerService {
    fn default() -> Self {
        Self::new()
    }
}
