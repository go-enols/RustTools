//! YOLO训练服务 - 纯Rust实现，无Python依赖
//! 
//! 使用 Burn 框架进行深度学习训练
//! 支持 CUDA (burn-cudarc) 和 CPU (burn-ndarray) 后端

use crate::modules::yolo::models::training::{TrainingRequest, TrainingStatus, TrainingMetrics};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};

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
    is_paused: bool,
    pause_tx: Option<oneshot::Sender<()>>,
    pause_rx: Option<oneshot::Receiver<()>>,
}

// 重新导出burn_trainer中的TrainingEvent
pub use crate::modules::yolo::services::burn_trainer::TrainingEvent;

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
        let mut rng = rand::rng();
        let bytes: [u8; 16] = rng.random();
        hex::encode(bytes)
    }
    
    /// 暂停训练
    pub async fn pause_training(&self, training_id: &str) -> Result<(), String> {
        eprintln!("[Trainer] Pausing training: {}", training_id);
        
        let mut processes = self.processes.write().await;
        if let Some(handle) = processes.get_mut(training_id) {
            if handle.is_paused {
                return Err("Training already paused".to_string());
            }
            if let Some(pause_tx) = handle.pause_tx.take() {
                let _ = pause_tx.send(());
                handle.is_paused = true;
                handle.status.paused = true;
                handle.status.running = false;
                eprintln!("[Trainer] Training paused: {}", training_id);
                Ok(())
            } else {
                Err("Pause signal not available".to_string())
            }
        } else {
            Err("Training not found".to_string())
        }
    }
    
    /// 恢复训练
    pub async fn resume_training(&self, training_id: &str) -> Result<(), String> {
        eprintln!("[Trainer] Resuming training: {}", training_id);
        
        let mut processes = self.processes.write().await;
        if let Some(handle) = processes.get_mut(training_id) {
            if !handle.is_paused {
                return Err("Training not paused".to_string());
            }
            handle.is_paused = false;
            handle.status.paused = false;
            handle.status.running = true;
            eprintln!("[Trainer] Training resumed: {}", training_id);
            Ok(())
        } else {
            Err("Training not found".to_string())
        }
    }
    
    /// 检查模型是否存在
    pub async fn check_model(&self, model_name: &str) -> Result<(bool, Option<String>), String> {
        let models = self.get_trained_models().await?;
        
        for model in models {
            if model.project_name == model_name {
                let path = Some(model.model_path.clone());
                return Ok((true, path));
            }
        }
        
        // 检查模型文件是否存在
        let models_dir = Self::get_models_dir();
        let model_path = models_dir.join(format!("{}.pt", model_name));
        
        if model_path.exists() {
            return Ok((true, Some(model_path.to_string_lossy().to_string())));
        }
        
        Ok((false, None))
    }
    
    /// 获取所有可用的预训练模型列表
    /// 
    /// 注意: 只有 YOLO11n 系列模型在 GitHub 上可用 (v8.4.0)
    /// 其他模型 (yolo11s/m/l/x, yolov8系列) 目前返回404
    pub fn get_available_models() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            // YOLO11n系列 (v8.4.0) - 使用 ghproxy.com 镜像加速
            // 检测模型
            ("yolo11n", "检测", "https://ghproxy.com/https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11n.onnx"),
            // 分割模型
            ("yolo11n-seg", "分割", "https://ghproxy.com/https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11n-seg.onnx"),
            // 姿态估计模型
            ("yolo11n-pose", "姿态估计", "https://ghproxy.com/https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11n-pose.onnx"),
            // 旋转边界框模型
            ("yolo11n-obb", "旋转边界框", "https://ghproxy.com/https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11n-obb.onnx"),
            // 分类模型
            ("yolo11n-cls", "分类", "https://ghproxy.com/https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11n-cls.onnx"),
        ]
    }
    
    /// 获取所有可用的模型名称（带描述）
    pub fn list_available_models() -> Vec<(String, String)> {
        Self::get_available_models()
            .iter()
            .map(|(name, desc, _)| (name.to_string(), desc.to_string()))
            .collect()
    }
    
    /// 下载预训练模型
    pub async fn download_model<F>(&self, model_name: &str, progress_callback: F) -> Result<String, String> 
    where
        F: Fn(String) + Send + 'static,
    {
        use futures_util::StreamExt;
        
        progress_callback(format!("开始下载模型: {}", model_name));
        
        // 获取模型URL
        let model_urls = Self::get_available_models();
        let model_key = model_name.to_lowercase();
        let url = model_urls.iter()
            .find(|(name, _, _)| *name == model_key)
            .map(|(_, _, url)| url)
            .ok_or_else(|| {
                let available: Vec<String> = model_urls.iter()
                    .map(|(name, desc, _)| format!("{} ({})", name, desc))
                    .collect();
                format!("未知的模型: {}\n\n可用的模型:\n{}", model_key, available.join("\n"))
            })?;
        
        progress_callback(format!("下载地址: {}", url));
        
        // 创建下载目录
        let models_dir = Self::get_models_dir();
        fs::create_dir_all(&models_dir)
            .map_err(|e| format!("创建模型目录失败: {}", e))?;
        
        let model_path = models_dir.join(format!("{}.onnx", model_name));
        
        // 如果模型已存在，检查文件完整性（不为空）
        if model_path.exists() {
            let metadata = fs::metadata(&model_path)
                .map_err(|e| format!("读取模型文件失败: {}", e))?;
            
            if metadata.len() > 1024 {  // 文件大于1KB，认为是完整的
                progress_callback(format!("模型已存在，跳过下载: {:?}", model_path));
                return Ok(model_path.to_string_lossy().to_string());
            } else {
                progress_callback(format!("模型文件不完整，将重新下载: {} bytes", metadata.len()));
                // 删除不完整的文件，重新下载
                fs::remove_file(&model_path)
                    .map_err(|e| format!("删除不完整文件失败: {}", e))?;
            }
        }
        
        progress_callback(format!("正在从 GitHub 下载..."));
        
        // 下载模型
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))  // 5分钟超时
            .build()
            .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
        
        let response = client.get(*url)
            .send()
            .await
            .map_err(|e| format!("下载失败: {}\n请检查网络连接或代理设置", e))?;
        
        // 检查HTTP状态码
        if !response.status().is_success() {
            return Err(format!(
                "下载失败，HTTP状态码: {}\nURL: {}\n如果持续失败，请手动下载模型后放到: {:?}",
                response.status(),
                url,
                model_path
            ));
        }
        
        let total_size = response.content_length()
            .ok_or("无法获取文件大小，请检查网络连接")?;
        
        progress_callback(format!("文件大小: {:.2} MB", total_size as f64 / 1024.0 / 1024.0));
        
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::File::create(&model_path)
            .await
            .map_err(|e| format!("创建文件失败: {}", e))?;
        
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut last_progress = 0u32;
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("读取数据失败: {}", e))?;
            file.write_all(&chunk).await
                .map_err(|e| format!("写入文件失败: {}", e))?;
            
            downloaded += chunk.len() as u64;
            
            // 每10%报告一次进度
            let progress = ((downloaded as f64 / total_size as f64) * 100.0) as u32;
            if progress >= last_progress + 10 {
                last_progress = progress;
                progress_callback(format!("下载进度: {}% ({} / {} MB)", 
                    progress, 
                    downloaded / (1024 * 1024),
                    total_size / (1024 * 1024)
                ));
            }
        }
        
        file.flush().await
            .map_err(|e| format!("刷新文件失败: {}", e))?;
        
        // 验证下载的文件
        let final_metadata = fs::metadata(&model_path)
            .map_err(|e| format!("验证文件失败: {}", e))?;
        
        if final_metadata.len() < 1024 {
            return Err(format!("下载的文件不完整: {} bytes", final_metadata.len()));
        }
        
        progress_callback(format!("下载完成: {:?} ({:.2} MB)", model_path, final_metadata.len() as f64 / 1024.0 / 1024.0));
        
        Ok(model_path.to_string_lossy().to_string())
    }
    
    /// 检查模型是否已下载
    pub async fn is_model_downloaded(&self, model_name: &str) -> Result<(bool, Option<String>), String> {
        let model_urls = Self::get_available_models();
        let model_key = model_name.to_lowercase();
        
        // 检查是否是已知的模型
        let is_known = model_urls.iter().any(|(name, _, _)| *name == model_key);
        if !is_known {
            return Err(format!("未知的模型: {}", model_key));
        }
        
        let models_dir = Self::get_models_dir();
        let model_path = models_dir.join(format!("{}.onnx", model_key));
        
        if model_path.exists() {
            let metadata = fs::metadata(&model_path)
                .map_err(|e| format!("读取文件失败: {}", e))?;
            
            if metadata.len() > 1024 {
                return Ok((true, Some(model_path.to_string_lossy().to_string())));
            }
        }
        
        Ok((false, None))
    }
    
    /// 启动纯Rust训练 - 使用Burn框架
    pub async fn start_training(
        &self,
        project_path: String,
        request: TrainingRequest,
        event_tx: mpsc::UnboundedSender<TrainingEvent>,
    ) -> Result<String, String> {
        let training_id = Self::generate_id();
        
        eprintln!("[Trainer] Starting Burn-based training (Pure Rust)");
        eprintln!("[Trainer] Training ID: {}", training_id);
        eprintln!("[Trainer] Project path: {}", project_path);
        
        // 创建停止信号通道
        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        // 创建暂停信号通道
        let (pause_tx, pause_rx) = oneshot::channel::<()>();
        
        // 初始化训练状态
        let initial_status = TrainingStatus {
            running: true,
            paused: false,
            epoch: 0,
            total_epochs: request.epochs,
            progress_percent: 0.0,
            metrics: TrainingMetrics::default(),
            error: None,
        };
        
        // 保存训练句柄
        {
            let mut processes = self.processes.write().await;
            processes.insert(
                training_id.clone(),
                TrainingHandle {
                    status: initial_status,
                    stop_tx: Some(stop_tx),
                    model_path: None,
                    project_name: request.name.clone(),
                    project_path: project_path.clone(),
                    yolo_version: request.base_model.clone(),
                    total_epochs: request.epochs,
                    is_paused: false,
                    pause_tx: Some(pause_tx),
                    pause_rx: Some(pause_rx),
                },
            );
        }
        
        // 发送启动事件
        event_tx.send(TrainingEvent::Started {
            training_id: training_id.clone(),
            total_epochs: request.epochs,
            cuda_available: false, // TODO: 检测CUDA
        }).map_err(|e| format!("Failed to send started event: {}", e))?;
        
        // 在后台spawn训练任务
        let processes_clone = self.processes.clone();
        let training_id_clone = training_id.clone();
        let project_name = request.name.clone();
        let project_path_clone = project_path.clone();
        let epochs_clone = request.epochs;
        let base_model_clone = request.base_model.clone();
        
        tokio::spawn(async move {
            // 使用Burn训练器进行训练
            let result = Self::run_burn_training(
                training_id_clone.clone(),
                project_path_clone.clone(),
                request,
                stop_rx,
                event_tx.clone(),
            ).await;
            
            // 更新训练状态
            let mut processes = processes_clone.write().await;
            if let Some(handle) = processes.get_mut(&training_id_clone) {
                match result {
                    Ok(model_path) => {
                        handle.status.running = false;
                        handle.model_path = Some(model_path.clone());
                        
                        // 保存训练完成的模型信息
                        let model_info = TrainedModelInfo {
                            id: training_id_clone.clone(),
                            project_name: project_name,
                            project_path: project_path_clone,
                            yolo_version: base_model_clone,
                            model_size: "N/A".to_string(),
                            best_epoch: epochs_clone,
                            total_epochs: epochs_clone,
                            map50: 0.0,
                            map50_95: 0.0,
                            model_path: model_path,
                            created_at: chrono::Utc::now().to_rfc3339(),
                        };
                        
                        // 注意：这里需要通过某种方式保存model_info
                        // 实际实现中应该调用save_trained_model
                    }
                    Err(e) => {
                        handle.status.running = false;
                        handle.status.error = Some(e.clone());
                        let _ = event_tx.send(TrainingEvent::Error {
                            error: e,
                        });
                    }
                }
            }
        });
        
        Ok(training_id)
    }
    
    /// 运行Burn训练的核心逻辑
    async fn run_burn_training(
        training_id: String,
        project_path: String,
        request: TrainingRequest,
        stop_rx: oneshot::Receiver<()>,
        event_tx: mpsc::UnboundedSender<TrainingEvent>,
    ) -> Result<String, String> {
        use crate::modules::yolo::services::burn_trainer::{TrainingConfig, BurnTrainer};
        
        eprintln!("[Trainer] Initializing Burn training framework...");
        
        // 创建训练配置
        let config = TrainingConfig {
            project_name: request.name.clone(),
            epochs: request.epochs,
            batch_size: request.batch_size,
            image_size: request.image_size as usize,
            num_classes: 80, // TODO: 从data.yaml读取
            optimizer: request.optimizer.clone(),
            learning_rate: request.lr0,
            weight_decay: request.weight_decay,
            momentum: request.momentum,
            warmup_epochs: request.warmup_epochs as u32,
            device: if request.device_id >= 0 { "cuda".to_string() } else { "cpu".to_string() },
            workers: request.workers,
            save_period: 10, // 每10个epoch保存一次
        };
        
        // 创建Burn训练器
        let trainer = BurnTrainer::new();
        
        // 启动异步训练
        match trainer.train_async(training_id.clone(), config, event_tx.clone()).await {
            Ok(model_path) => {
                eprintln!("[Trainer] Burn training completed successfully");
                Ok(model_path)
            }
            Err(e) => {
                eprintln!("[Trainer] Burn training failed: {}", e);
                Err(e)
            }
        }
    }
    
    pub async fn stop_training(&self, training_id: &str) -> Result<(), String> {
        eprintln!("[Trainer] Stopping training: {}", training_id);
        
        let mut processes = self.processes.write().await;
        if let Some(handle) = processes.get_mut(training_id) {
            if let Some(stop_tx) = handle.stop_tx.take() {
                let _ = stop_tx.send(());
                handle.status.running = false;
                handle.status.paused = false;
                eprintln!("[Trainer] Stop signal sent to training: {}", training_id);
                Ok(())
            } else {
                Err("Training already stopped".to_string())
            }
        } else {
            Err("Training not found".to_string())
        }
    }
    
    pub async fn get_training_status(&self, training_id: &str) -> Option<TrainingStatus> {
        let processes = self.processes.read().await;
        processes.get(training_id).map(|h| h.status.clone())
    }
    
    pub async fn list_trainings(&self) -> Vec<(String, TrainingStatus)> {
        let processes = self.processes.read().await;
        processes.iter()
            .map(|(id, handle)| (id.clone(), handle.status.clone()))
            .collect()
    }
}
