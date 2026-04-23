use serde::{Deserialize, Serialize};

// ============================================================================
// Project Models
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct YoloAnnotation {
    pub class_id: usize,
    pub x_center: f64,
    pub y_center: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DatasetPaths {
    pub train: String,
    pub val: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub path: String,
    pub yolo_version: String,
    pub classes: Vec<String>,
    pub train_split: f64,
    pub val_split: f64,
    pub image_size: i32,
    pub description: Option<String>,
    #[serde(default)]
    pub images: DatasetPaths,
    #[serde(default)]
    pub labels: DatasetPaths,
}

#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub success: bool,
    pub data: Option<ProjectConfig>,
    pub error: Option<String>,
}

impl ProjectResponse {
    pub fn ok(data: ProjectConfig) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

// ============================================================================
// Training Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub train_box_loss: f32,
    pub train_cls_loss: f32,
    pub train_dfl_loss: f32,
    pub precision: f32,
    pub recall: f32,
    pub map50: f32,
    pub map50_95: f32,
}

impl Default for TrainingMetrics {
    fn default() -> Self {
        Self {
            train_box_loss: 0.0,
            train_cls_loss: 0.0,
            train_dfl_loss: 0.0,
            precision: 0.0,
            recall: 0.0,
            map50: 0.0,
            map50_95: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStatus {
    pub running: bool,
    pub paused: bool,
    pub epoch: u32,
    pub total_epochs: u32,
    pub progress_percent: f32,
    pub metrics: TrainingMetrics,
    pub error: Option<String>,
}

impl Default for TrainingStatus {
    fn default() -> Self {
        Self {
            running: false,
            paused: false,
            epoch: 0,
            total_epochs: 0,
            progress_percent: 0.0,
            metrics: TrainingMetrics::default(),
            error: None,
        }
    }
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
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

// ============================================================================
// Generic Response
// ============================================================================

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

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}
