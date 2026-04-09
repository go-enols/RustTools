// Shared API Response Type
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

export interface DialogResult {
  canceled: boolean;
  path?: string;
  paths?: string[];
}

// Project Types
export interface DatasetPaths {
  train: string;
  val: string;
}

export interface ProjectConfig {
  name: string;
  path: string;
  yolo_version: 'yolo5' | 'yolo8' | 'yolo11';
  classes: string[];
  train_split: number;
  val_split: number;
  image_size: number;
  description?: string;
  images: DatasetPaths;
  labels: DatasetPaths;
}

// Training Types
export interface TrainingConfig {
  base_model: string;
  epochs: number;
  batch_size: number;
  image_size: number;
  device_id: number;
  workers: number;
  optimizer: string;
  lr0: number;
  lrf: number;
  momentum: number;
  weight_decay: number;
  warmup_epochs: number;
  warmup_bias_lr: number;
  warmup_momentum: number;
  hsv_h: number;
  hsv_s: number;
  hsv_v: number;
  translate: number;
  scale: number;
  shear: number;
  perspective: number;
  flipud: number;
  fliplr: number;
  mosaic: number;
  mixup: number;
  copy_paste: number;
  close_mosaic: number;
  rect: boolean;
  cos_lr: boolean;
  single_cls: boolean;
  amp: boolean;
  save_period: number;
  cache: boolean;
}

export interface TrainingProgress {
  epoch: number;
  total_epochs: number;
  train_box_loss: number;
  train_cls_loss: number;
  train_dfl_loss: number;
  val_box_loss: number;
  val_cls_loss: number;
  val_dfl_loss: number;
  precision: number;
  recall: number;
  map50: number;
  map50_95: number;
  gpu_memory: number;
  inference_speed: number;
  learning_rate: number;
}

export interface TrainedModel {
  id: string;
  project_name: string;
  yolo_version: string;
  model_size: string;
  best_epoch: number;
  total_epochs: number;
  map50: number;
  map50_95: number;
  model_path: string;
  created_at: string;
}

// Convert Types
export interface ConvertConfig {
  model_path: string;
  model_type: 'yolo5' | 'yolo8' | 'yolo11';
  target_platform: 'rk3588' | 'rk3568' | 'rk3566' | 'aml-s905x' | 'aml-s912' | 'hisi-3519' | 'horizon-j3' | 'tegra';
  output_path?: string;
}

export interface ConvertProgress {
  progress: number;
  status: 'converting' | 'completed' | 'failed';
  message?: string;
  output_path?: string;
}

// Device Types
export interface DeviceInfo {
  id: number;
  name: string;
  type: 'GPU' | 'CPU';
  memory_total: number;
  memory_used: number;
  memory_free: number;
  driver_version?: string;
  cuda_version?: string;
  compute_capability?: string;
}

// Video Types
export interface VideoInferenceConfig {
  video_path: string;
  model_path: string;
  confidence: number;
  iou_threshold: number;
  device: string;         // "0" = GPU 0, "cpu" = CPU
  output_dir: string;     // Directory for inference output
  frame_interval: number; // Process every N frames
}

// Annotation Types
export interface AnnotationBox {
  id: string;
  class_id: number;
  class_name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  confidence?: number;
}

export interface AnnotationImage {
  id: string;
  path: string;
  width: number;
  height: number;
  annotations: AnnotationBox[];
  is_labeled: boolean;
}

export interface DatasetStats {
  total_images: number;
  labeled_images: number;
  unlabeled_images: number;
  total_annotations: number;
  class_distribution: Record<string, number>;
}
