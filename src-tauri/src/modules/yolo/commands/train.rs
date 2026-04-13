use crate::modules::yolo::models::training::{TrainingMetrics, TrainingRequest};
use crate::modules::yolo::services::{TrainerService, TrainingEvent};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainedModelResponse {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingProgressEvent {
    pub training_id: String,
    pub epoch: u32,
    pub total_epochs: u32,
    pub progress_percent: f32,
    pub metrics: TrainingMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingBatchProgressEvent {
    pub training_id: String,
    pub epoch: u32,
    pub total_epochs: u32,
    pub batch: u32,
    pub total_batches: u32,
    pub box_loss: f32,
    pub cls_loss: f32,
    pub dfl_loss: f32,
    pub learning_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStartedEvent {
    pub training_id: String,
    pub cuda_available: bool,
    pub cuda_version: Option<String>,
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
    #[serde(default)]
    pub name: Option<String>,
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
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    let name = config.name.unwrap_or_else(|| "yolo_train".to_string());
    let request = TrainingRequest {
        name,
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

    match state.start_training(project_path, request, event_tx).await {
        Ok(training_id) => {
            let app_clone = app.clone();

            tokio::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    match event {
                        TrainingEvent::Started { training_id, total_epochs, cuda_available } => {
                            eprintln!(
                                "[Trainer] Training started: id={}, total_epochs={}",
                                training_id, total_epochs
                            );
                            let event = TrainingStartedEvent {
                                training_id,
                                cuda_available,
                                cuda_version: None, // Burn不提供CUDA版本信息
                            };
                            let _ = app_clone.emit("training-started", event);
                        }
                        TrainingEvent::BatchProgress(state) => {
                            let event = TrainingBatchProgressEvent {
                                training_id: "".to_string(), // TrainingState没有training_id
                                epoch: state.epoch,
                                total_epochs: state.total_epochs,
                                batch: state.batch,
                                total_batches: state.total_batches,
                                box_loss: state.box_loss,
                                cls_loss: state.cls_loss,
                                dfl_loss: state.dfl_loss,
                                learning_rate: state.learning_rate,
                            };
                            let _ = app_clone.emit("training-batch-progress", event);
                        }
                        TrainingEvent::EpochComplete {
                            epoch,
                            box_loss,
                            cls_loss,
                            total_loss,
                            map50,
                        } => {
                            let event = TrainingProgressEvent {
                                training_id: "".to_string(),
                                epoch,
                                total_epochs: 0,
                                progress_percent: 0.0,
                                metrics: TrainingMetrics {
                                    train_box_loss: box_loss,
                                    train_cls_loss: cls_loss,
                                    train_dfl_loss: total_loss,
                                    val_box_loss: 0.0,
                                    val_cls_loss: 0.0,
                                    val_dfl_loss: 0.0,
                                    precision: 0.0,
                                    recall: 0.0,
                                    map50: map50.unwrap_or(0.0),
                                    map50_95: 0.0,
                                    learning_rate: 0.0,
                                },
                            };
                            let _ = app_clone.emit("training-progress", event);
                        }
                        TrainingEvent::Complete { model_path } => {
                            let event = TrainingCompleteEvent {
                                training_id: "".to_string(),
                                success: true,
                                model_path: Some(model_path),
                                error: None,
                            };
                            let _ = app_clone.emit("training-complete", event);
                            break;
                        }
                        TrainingEvent::Error { error } => {
                            let event = TrainingCompleteEvent {
                                training_id: "".to_string(),
                                success: false,
                                model_path: None,
                                error: Some(error),
                            };
                            let _ = app_clone.emit("training-complete", event);
                            break;
                        }
                        TrainingEvent::Stopped => {
                            let event = TrainingCompleteEvent {
                                training_id: "".to_string(),
                                success: false,
                                model_path: None,
                                error: Some("训练已停止".to_string()),
                            };
                            let _ = app_clone.emit("training-complete", event);
                            break;
                        }
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

/// Get list of trained models
#[tauri::command]
pub async fn model_list(
    state: State<'_, Arc<TrainerService>>,
) -> Result<CommandResponse<Vec<TrainedModelResponse>>, String> {
    match state.get_trained_models().await {
        Ok(models) => {
            let response: Vec<TrainedModelResponse> = models
                .into_iter()
                .map(|m| TrainedModelResponse {
                    id: m.id,
                    project_name: m.project_name,
                    project_path: m.project_path,
                    yolo_version: m.yolo_version,
                    model_size: m.model_size,
                    best_epoch: m.best_epoch,
                    total_epochs: m.total_epochs,
                    map50: m.map50,
                    map50_95: m.map50_95,
                    model_path: m.model_path,
                    created_at: m.created_at,
                })
                .collect();
            Ok(CommandResponse::ok(response))
        }
        Err(e) => Ok(CommandResponse::err(e)),
    }
}

/// Delete a trained model
#[tauri::command]
pub async fn model_delete(
    state: State<'_, Arc<TrainerService>>,
    model_id: String,
) -> Result<CommandResponse<()>, String> {
    match state.delete_trained_model(&model_id).await {
        Ok(_) => Ok(CommandResponse::ok(())),
        Err(e) => Ok(CommandResponse::err(e)),
    }
}
