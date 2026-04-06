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
        // YOLO expects data parameter to point to data.yaml file
        // We need to run from the project directory so relative paths work
        let data_yaml = "data.yaml".to_string();

        let mut cmd = Command::new("yolo");
        cmd.current_dir(&project_path)  // Run from project directory
            .args([
            "detect", "train",
            &format!("data={}", data_yaml),
            &format!("model={}", request.base_model),
            &format!("epochs={}", request.epochs),
            &format!("batch={}", request.batch_size),
            &format!("imgsz={}", request.image_size),
            &format!("device={}", request.device_id),
            &format!("workers={}", request.workers),
            &format!("optimizer={}", request.optimizer),
            &format!("lr0={}", request.lr0),
            &format!("lrf={}", request.lrf),
            &format!("momentum={}", request.momentum),
            &format!("weight_decay={}", request.weight_decay),
            &format!("warmup_epochs={}", request.warmup_epochs),
            &format!("warmup_bias_lr={}", request.warmup_bias_lr),
            &format!("warmup_momentum={}", request.warmup_momentum),
            &format!("hsv_h={}", request.hsv_h),
            &format!("hsv_s={}", request.hsv_s),
            &format!("hsv_v={}", request.hsv_v),
            &format!("translate={}", request.translate),
            &format!("scale={}", request.scale),
            &format!("shear={}", request.shear),
            &format!("perspective={}", request.perspective),
            &format!("flipud={}", request.flipud),
            &format!("fliplr={}", request.fliplr),
            &format!("mosaic={}", request.mosaic),
            &format!("mixup={}", request.mixup),
            &format!("copy_paste={}", request.copy_paste),
            &format!("close_mosaic={}", request.close_mosaic),
            &format!("rect={}", request.rect),
            &format!("cos_lr={}", request.cos_lr),
            &format!("single_cls={}", request.single_cls),
            &format!("amp={}", request.amp),
            &format!("save_period={}", request.save_period),
            &format!("cache={}", request.cache),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

        // Check if yolo command exists first
        let check_cmd = Command::new("yolo")
            .args(["version"])
            .output()
            .await;

        if let Ok(output) = check_cmd {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("YOLO command check failed: {}", stderr));
            }
            let version = String::from_utf8_lossy(&output.stdout);
            eprintln!("[YOLO] Version: {}", version);
        }

        let mut child = cmd.spawn().map_err(|e| format!("Failed to start training process: {}. Is yolo CLI installed? Run 'pip install ultralytics' first.", e))?;

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
            let mut startup_errors = Vec::new();

            loop {
                tokio::select! {
                    _ = stop_rx.recv() => {
                        break;
                    }
                    line = stdout_reader.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                eprintln!("[YOLO stdout] {}", line);
                                if let Some(proc) = processes.write().await.get_mut(&process_id) {
                                    Self::parse_output(&line, &mut proc.status);
                                }
                            }
                            Ok(None) => {
                                // stdout closed - check for startup errors
                                if let Some(proc) = processes.write().await.get_mut(&process_id) {
                                    if proc.status.epoch == 0 && !startup_errors.is_empty() {
                                        proc.status.running = false;
                                        let _ = proc.stop_tx.take();
                                    }
                                }
                                break;
                            }
                            Err(_) => break,
                        }
                    }
                    line = stderr_reader.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                eprintln!("[YOLO stderr] {}", line);
                                // Collect potential error messages during startup
                                let lower = line.to_lowercase();
                                if lower.contains("error") || lower.contains("failed") || lower.contains("warning") {
                                    startup_errors.push(line.clone());
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
