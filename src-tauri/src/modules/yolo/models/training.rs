use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRequest {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStatus {
    pub running: bool,
    pub paused: bool,
    pub epoch: u32,
    pub total_epochs: u32,
    pub progress_percent: f32,
    pub metrics: TrainingMetrics,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub train_box_loss: f32,
    pub train_cls_loss: f32,
    pub train_dfl_loss: f32,
    pub val_box_loss: f32,
    pub val_cls_loss: f32,
    pub val_dfl_loss: f32,
    pub precision: f32,
    pub recall: f32,
    pub map50: f32,
    pub map50_95: f32,
    pub learning_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainedModelInfo {
    pub id: String,
    pub name: String,
    pub project_name: String,
    pub yolo_version: String,
    pub model_size: String,
    pub best_epoch: u32,
    pub total_epochs: u32,
    pub map50: f32,
    pub map50_95: f32,
    pub model_path: String,
    pub created_at: String,
}
