import { create } from 'zustand';
import { listen } from '@tauri-apps/api/event';
import {
  startTraining as apiStartTraining,
  stopTraining as apiStopTraining,
  pauseTraining as apiPauseTraining,
  resumeTraining as apiResumeTraining,
  getTrainedModels as apiGetTrainedModels,
  deleteModel as apiDeleteModel,
  getBaseModels as apiGetBaseModels,
  TrainingConfig as ApiTrainingConfig,
  TrainedModel as ApiTrainedModel,
} from '../api';
import { useWorkspaceStore } from './workspaceStore';

export interface TrainedModel {
  id: string;
  projectName: string;
  yoloVersion: string;
  modelSize: string;
  bestEpoch: number;
  totalEpochs: number;
  map50: number;
  map50_95: number;
  modelPath: string;
  createdAt: Date;
}

export interface TrainingMetrics {
  epoch: number;
  trainBoxLoss: number;
  trainClsLoss: number;
  trainDflLoss: number;
  valBoxLoss: number;
  valClsLoss: number;
  valDflLoss: number;
  precision: number;
  recall: number;
  map50: number;
  map50_95: number;
}

export interface TrainingConfig {
  // Model
  base_model: string;
  // Training
  epochs: number;
  patience: number;
  batch_size: number;
  image_size: number;
  // Device & Workers
  device_id: number;
  workers: number;
  // Optimizer
  optimizer: 'SGD' | 'Adam' | 'AdamW';
  lr0: number;
  lrf: number;
  momentum: number;
  weight_decay: number;
  // Warmup
  warmup_epochs: number;
  warmup_bias_lr: number;
  warmup_momentum: number;
  // Data Augmentation
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
  // Advanced
  close_mosaic: number;
  rect: boolean;
  cos_lr: boolean;
  single_cls: boolean;
  amp: boolean;
  save_period: number;
  cache: boolean;
}

export interface BaseModel {
  name: string;
  path: string;
  size: string;
}

interface TrainingState {
  isTraining: boolean;
  isPaused: boolean;
  currentEpoch: number;
  totalEpochs: number;
  startTime: Date | null;
  metrics: TrainingMetrics[];
  trainedModels: TrainedModel[];
  baseModels: BaseModel[];
  currentTrainingId: string | null;
  error: string | null;

  startTraining: (config: TrainingConfig) => Promise<void>;
  stopTraining: () => Promise<void>;
  pauseTraining: () => Promise<void>;
  resumeTraining: () => Promise<void>;
  updateMetrics: (metrics: TrainingMetrics) => void;
  addTrainedModel: (model: Omit<TrainedModel, 'id' | 'createdAt'>) => void;
  removeTrainedModel: (id: string) => Promise<void>;
  loadTrainedModels: () => Promise<void>;
  loadBaseModels: () => Promise<void>;
  clearError: () => void;
}

// Convert frontend config to API config
function toApiConfig(config: TrainingConfig): ApiTrainingConfig {
  return {
    base_model: config.base_model,
    epochs: config.epochs,
    batch_size: config.batch_size,
    image_size: config.image_size,
    device_id: config.device_id,
    workers: config.workers,
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
    optimizer: config.optimizer,
    lr0: config.lr0,
    lrf: config.lrf,
    momentum: config.momentum,
    weight_decay: config.weight_decay,
    warmup_epochs: config.warmup_epochs,
    warmup_bias_lr: config.warmup_bias_lr,
    warmup_momentum: config.warmup_momentum,
    copy_paste: config.copy_paste,
    close_mosaic: config.close_mosaic,
    rect: config.rect,
    cos_lr: config.cos_lr,
    single_cls: config.single_cls,
    amp: config.amp,
    save_period: config.save_period,
    cache: config.cache,
  };
}

// Convert API model to frontend model
function toFrontendModel(apiModel: ApiTrainedModel): TrainedModel {
  return {
    id: apiModel.id,
    projectName: apiModel.project_name,
    yoloVersion: apiModel.yolo_version,
    modelSize: apiModel.model_size,
    bestEpoch: apiModel.best_epoch,
    totalEpochs: apiModel.total_epochs,
    map50: apiModel.map50,
    map50_95: apiModel.map50_95,
    modelPath: apiModel.model_path,
    createdAt: new Date(apiModel.created_at),
  };
}

export const useTrainingStore = create<TrainingState>((set, get) => ({
  isTraining: false,
  isPaused: false,
  currentEpoch: 0,
  totalEpochs: 50,
  startTime: null,
  metrics: [],
  trainedModels: [],
  baseModels: [],
  currentTrainingId: null,
  error: null,

  startTraining: async (config) => {
    set({ error: null });  // Clear previous error
    const { currentTrainingId } = get();
    if (currentTrainingId) {
      console.warn('Training already in progress');
      return;
    }

    const { currentProject } = useWorkspaceStore.getState();
    if (!currentProject) {
      const error = '请先打开一个项目';
      set({ error });
      return;
    }

    try {
      // Listen for training progress events
      const unlistenProgress = await listen<{
        training_id: string;
        epoch: number;
        total_epochs: number;
        progress_percent: number;
        metrics: {
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
          learning_rate: number;
        };
      }>('training-progress', (event) => {
        const { metrics: m } = event.payload;
        const metrics: TrainingMetrics = {
          epoch: event.payload.epoch,
          trainBoxLoss: m.train_box_loss,
          trainClsLoss: m.train_cls_loss,
          trainDflLoss: m.train_dfl_loss,
          valBoxLoss: m.val_box_loss,
          valClsLoss: m.val_cls_loss,
          valDflLoss: m.val_dfl_loss,
          precision: m.precision,
          recall: m.recall,
          map50: m.map50,
          map50_95: m.map50_95,
        };
        get().updateMetrics(metrics);
      });

      // Listen for training complete
      const unlistenComplete = await listen<{
        training_id: string;
        success: boolean;
        model_path?: string;
        error?: string;
      }>('training-complete', (event) => {
        unlistenProgress();
        unlistenComplete();
        const { success, model_path, error } = event.payload;
        if (success && model_path) {
          // Add the trained model to the list
          get().addTrainedModel({
            projectName: useWorkspaceStore.getState().currentProject?.name || 'Unknown',
            yoloVersion: config.base_model.replace('.pt', ''),
            modelSize: '0',
            bestEpoch: get().currentEpoch,
            totalEpochs: get().totalEpochs,
            map50: get().metrics[get().metrics.length - 1]?.map50 || 0,
            map50_95: get().metrics[get().metrics.length - 1]?.map50_95 || 0,
            modelPath: model_path,
          });
        }
        set({
          isTraining: false,
          isPaused: false,
          currentTrainingId: null,
          error: success ? null : (error || '训练失败'),
        });
      });

      const response = await apiStartTraining(currentProject.path, toApiConfig(config));

      if (!response.success || !response.data) {
        unlistenProgress();
        unlistenComplete();
        throw new Error(response.error || '启动训练失败');
      }

      set({
        isTraining: true,
        isPaused: false,
        currentEpoch: 0,
        totalEpochs: config.epochs,
        startTime: new Date(),
        metrics: [],
        currentTrainingId: response.data.training_id,
      });
    } catch (error) {
      console.error('Failed to start training:', error);
      const errorMessage = error instanceof Error ? error.message : '启动训练失败';
      set({ isTraining: false, currentTrainingId: null, error: errorMessage });
    }
  },

  stopTraining: async () => {
    try {
      await apiStopTraining();
      set({
        isTraining: false,
        isPaused: false,
        currentTrainingId: null,
      });
    } catch (error) {
      console.error('Failed to stop training:', error);
    }
  },

  pauseTraining: async () => {
    try {
      await apiPauseTraining();
      set({ isPaused: true });
    } catch (error) {
      console.error('Failed to pause training:', error);
    }
  },

  resumeTraining: async () => {
    try {
      await apiResumeTraining();
      set({ isPaused: false });
    } catch (error) {
      console.error('Failed to resume training:', error);
    }
  },

  updateMetrics: (metrics) => {
    set((state) => ({
      metrics: [...state.metrics, metrics],
      currentEpoch: metrics.epoch,
    }));
  },

  addTrainedModel: (model) => {
    const newModel: TrainedModel = {
      ...model,
      id: crypto.randomUUID(),
      createdAt: new Date(),
    };
    set((state) => ({
      trainedModels: [newModel, ...state.trainedModels],
    }));
  },

  removeTrainedModel: async (id) => {
    try {
      await apiDeleteModel(id);
      set((state) => ({
        trainedModels: state.trainedModels.filter((m) => m.id !== id),
      }));
    } catch (error) {
      console.error('Failed to delete model:', error);
    }
  },

  loadTrainedModels: async () => {
    try {
      const response = await apiGetTrainedModels();
      if (response.success && response.data) {
        const models = response.data.map(toFrontendModel);
        set({ trainedModels: models });
      }
    } catch (error) {
      console.error('Failed to load trained models:', error);
    }
  },

  loadBaseModels: async () => {
    try {
      const response = await apiGetBaseModels();
      if (response.success && response.data) {
        set({ baseModels: response.data });
      }
    } catch (error) {
      console.error('Failed to load base models:', error);
      // Set default models as fallback
      set({
        baseModels: [
          { name: 'yolo11n.pt', path: 'weights/yolo11n.pt', size: '5.9 MB' },
          { name: 'yolo11s.pt', path: 'weights/yolo11s.pt', size: '19.3 MB' },
          { name: 'yolo11m.pt', path: 'weights/yolo11m.pt', size: '42.4 MB' },
          { name: 'yolov8n.pt', path: 'weights/yolov8n.pt', size: '6.2 MB' },
          { name: 'yolov8s.pt', path: 'weights/yolov8s.pt', size: '21.5 MB' },
        ],
      });
    }
  },

  clearError: () => set({ error: null }),
}));
