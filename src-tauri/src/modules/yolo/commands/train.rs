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
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                    let is_running = state_inner.is_training(&training_id_clone).await;
                    if !is_running {
                        let event = TrainingCompleteEvent {
                            training_id: training_id_clone.clone(),
                            success: true,
                            model_path: Some("runs/detect/train/weights/best.pt".to_string()),
                            error: None,
                        };
                        let _ = app_clone.emit("training-complete", event);
                        break;
                    }

                    if let Some(status) = state_inner.get_status(&training_id_clone).await {
                        let event = TrainingProgressEvent {
                            training_id: training_id_clone.clone(),
                            epoch: status.epoch,
                            total_epochs: status.total_epochs,
                            progress_percent: status.progress_percent,
                            metrics: status.metrics,
                        };
                        let _ = app_clone.emit("training-progress", event);
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
