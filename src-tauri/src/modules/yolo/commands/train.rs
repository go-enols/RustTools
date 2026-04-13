use crate::modules::yolo::models::training::{TrainingMetrics, TrainingRequest};
use crate::modules::yolo::services::TrainerService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingProgressEvent {
    pub training_id: String,
    pub epoch: u32,
    pub total_epochs: u32,
    pub progress_percent: f32,
    pub metrics: TrainingMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingCompleteEvent {
    pub training_id: String,
    pub success: bool,
    pub model_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub base_model: String,
    pub epochs: u32,
    pub batch_size: u32,
    pub image_size: u32,
    pub device_id: i32,
    pub workers: u32,
    pub optimizer: String,
    pub lr0: f32,
    pub lrf: f32,
    pub momentum: f32,
    pub weight_decay: f32,
    pub warmup_epochs: f32,
    pub warmup_bias_lr: f32,
    pub warmup_momentum: f32,
    pub hsv_h: f32,
    pub hsv_s: f32,
    pub hsv_v: f32,
    pub translate: f32,
    pub scale: f32,
    pub shear: f32,
    pub perspective: f32,
    pub flipud: f32,
    pub fliplr: f32,
    pub mosaic: f32,
    pub mixup: f32,
    pub copy_paste: f32,
    pub close_mosaic: u32,
    pub rect: bool,
    pub cos_lr: bool,
    pub single_cls: bool,
    pub amp: bool,
    pub save_period: i32,
    pub cache: bool,
}

#[derive(Debug, Serialize)]
pub struct CommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> CommandResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg),
        }
    }
}

#[tauri::command]
pub async fn training_start(
    app: AppHandle,
    state: State<'_, Arc<TrainerService>>,
    project_path: String,
    config: TrainingConfig,
) -> Result<CommandResponse<String>, String> {
    let request = TrainingRequest {
        base_model: config.base_model,
        epochs: config.epochs,
        batch_size: config.batch_size,
        image_size: config.image_size,
        device_id: config.device_id,
        workers: config.workers,
        optimizer: config.optimizer,
        lr0: config.lr0,
        lrf: config.lrf,
        momentum: config.momentum,
        weight_decay: config.weight_decay,
        warmup_epochs: config.warmup_epochs,
        warmup_bias_lr: config.warmup_bias_lr,
        warmup_momentum: config.warmup_momentum,
        hsv_h: config.hsv_h,
        hsv_s: config.hsv_s,
        hsv_v: config.hsv_v,
        translate: config.translate,
        scale: config.scale,
        shear: config.shear,
        perspective: config.perspective,
        flipud: config.flipud,
        fliplr: config.fliplr,
        mosaic: config.mosaic,
        mixup: config.mixup,
        copy_paste: config.copy_paste,
        close_mosaic: config.close_mosaic,
        rect: config.rect,
        cos_lr: config.cos_lr,
        single_cls: config.single_cls,
        amp: config.amp,
        save_period: config.save_period,
        cache: config.cache,
    };

    match state.start_training(project_path, request).await {
        Ok(training_id) => {
            // Spawn progress event emitter
            let app_clone = app.clone();
            let training_id_clone = training_id.clone();
            let state_inner = Arc::clone(&state);

            tokio::spawn(async move {
                let mut last_epoch = 0u32;

                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                    let is_running = state_inner.is_training(&training_id_clone).await;
                    let error = state_inner.get_error(&training_id_clone).await;
                    let status = state_inner.get_status(&training_id_clone).await;

                    if status.is_none() {
                        eprintln!("[Trainer] Status is None for training_id={}, is_running={}", training_id_clone, is_running);
                    }

                    // Only emit progress if epoch has changed (new data from Python)
                    if let Some(s) = &status {
                        if s.epoch > last_epoch {
                            eprintln!("[Trainer] Emitting progress: epoch={}, total={}, progress={}%, metrics.box_loss={}",
                                s.epoch, s.total_epochs, s.progress_percent, s.metrics.train_box_loss);
                            let event = TrainingProgressEvent {
                                training_id: training_id_clone.clone(),
                                epoch: s.epoch,
                                total_epochs: s.total_epochs,
                                progress_percent: s.progress_percent,
                                metrics: s.metrics.clone(),
                            };
                            let _ = app_clone.emit("training-progress", event);
                            eprintln!("[Trainer] Progress event emitted successfully");
                            last_epoch = s.epoch;  // Update after emitting

                            // Check if training is complete (epoch reached total)
                            if s.epoch >= s.total_epochs && s.total_epochs > 0 && s.epoch > 0 {
                                eprintln!("[Trainer] Training complete: epoch {} >= total {}", s.epoch, s.total_epochs);
                                let model_path = state_inner.get_model_path(&training_id_clone).await;
                                let event = TrainingCompleteEvent {
                                    training_id: training_id_clone.clone(),
                                    success: s.error.is_none(),
                                    model_path: if s.error.is_none() { model_path } else { None },
                                    error: s.error.clone(),
                                };
                                let _ = app_clone.emit("training-complete", event);
                                break;
                            }
                        }
                    }

                    // Check if process is still running
                    if !is_running {
                        eprintln!("[Trainer] Process not running (is_running=false), checking completion status");
                        // Process ended - check if we have valid metrics
                        if let Some(s) = &status {
                            eprintln!("[Trainer] Status check: epoch={}, total_epochs={}, error={:?}", s.epoch, s.total_epochs, s.error);
                            if s.epoch > 0 && s.total_epochs > 0 {
                                // We have some training data, consider it complete
                                let model_path = state_inner.get_model_path(&training_id_clone).await;
                                let event = TrainingCompleteEvent {
                                    training_id: training_id_clone.clone(),
                                    success: s.error.is_none(),
                                    model_path: if s.error.is_none() { model_path } else { None },
                                    error: s.error.clone(),
                                };
                                eprintln!("[Trainer] Sending training-complete event (with data): success={}", s.error.is_none());
                                let _ = app_clone.emit("training-complete", event);
                                break;
                            }
                        }
                        // No training data, emit error
                        eprintln!("[Trainer] Sending training-complete event (error case): error={:?}", error);
                        let event = TrainingCompleteEvent {
                            training_id: training_id_clone.clone(),
                            success: false,
                            model_path: None,
                            error: Some(error.unwrap_or_else(|| "Training process ended unexpectedly".to_string())),
                        };
                        let _ = app_clone.emit("training-complete", event);
                        break;
                    }

                }
            });

            Ok(CommandResponse::ok(training_id))
        }
        Err(e) => Ok(CommandResponse::err(e)),
    }
}

#[tauri::command]
pub async fn training_stop(
    state: State<'_, Arc<TrainerService>>,
    training_id: String,
) -> Result<CommandResponse<()>, String> {
    state.stop_training(&training_id).await?;
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn training_pause(
    state: State<'_, Arc<TrainerService>>,
    training_id: String,
) -> Result<CommandResponse<()>, String> {
    state.pause_training(&training_id).await?;
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn training_resume(
    state: State<'_, Arc<TrainerService>>,
    training_id: String,
) -> Result<CommandResponse<()>, String> {
    state.resume_training(&training_id).await?;
    Ok(CommandResponse::ok(()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelCheckResult {
    pub exists: bool,
    pub model: String,
    pub path: Option<String>,
}

#[tauri::command]
pub async fn yolo_check_model(
    state: State<'_, Arc<TrainerService>>,
    model_name: String,
) -> Result<CommandResponse<ModelCheckResult>, String> {
    match state.check_model(&model_name).await {
        Ok((exists, path)) => Ok(CommandResponse::ok(ModelCheckResult {
            exists,
            model: model_name,
            path,
        })),
        Err(e) => Ok(CommandResponse::err(e)),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelDownloadResult {
    pub success: bool,
    pub model: String,
    pub path: Option<String>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn yolo_download_model(
    app: AppHandle,
    state: State<'_, Arc<TrainerService>>,
    model_name: String,
) -> Result<CommandResponse<ModelDownloadResult>, String> {
    let app_clone = app.clone();
    let model_name_clone = model_name.clone();

    let result = state
        .download_model(&model_name, move |msg| {
            let _ = app_clone.emit("model-download-progress", serde_json::json!({
                "model": model_name_clone,
                "message": msg
            }));
        })
        .await;

    match result {
        Ok(path) => Ok(CommandResponse::ok(ModelDownloadResult {
            success: true,
            model: model_name,
            path: Some(path),
            error: None,
        })),
        Err(e) => Ok(CommandResponse::ok(ModelDownloadResult {
            success: false,
            model: model_name,
            path: None,
            error: Some(e),
        })),
    }
}
