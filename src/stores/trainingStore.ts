import { create } from 'zustand';
import {
  startTraining as apiStartTraining,
  stopTraining as apiStopTraining,
  pauseTraining as apiPauseTraining,
  resumeTraining as apiResumeTraining,
  getTrainedModels as apiGetTrainedModels,
  deleteModel as apiDeleteModel,
  getBaseModels as apiGetBaseModels,
  TrainingConfig as ApiTrainingConfig,
  TrainingProgress,
  TrainedModel as ApiTrainedModel,
} from '../api';

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
  baseModel: string;
  epochs: number;
  batchSize: number;
  imageSize: number;
  deviceId: number;
  workers: number;
  trainSplit: number;
  valSplit: number;
  hsvH: number;
  hsvS: number;
  hsvV: number;
  translate: number;
  scale: number;
  shear: number;
  perspective: number;
  flipud: number;
  fliplr: number;
  mosaic: number;
  mixup: number;
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

  startTraining: (config: TrainingConfig) => Promise<void>;
  stopTraining: () => Promise<void>;
  pauseTraining: () => Promise<void>;
  resumeTraining: () => Promise<void>;
  updateMetrics: (metrics: TrainingMetrics) => void;
  addTrainedModel: (model: Omit<TrainedModel, 'id' | 'createdAt'>) => void;
  removeTrainedModel: (id: string) => Promise<void>;
  loadTrainedModels: () => Promise<void>;
  loadBaseModels: () => Promise<void>;
}

// Convert frontend config to API config
function toApiConfig(config: TrainingConfig): ApiTrainingConfig {
  return {
    baseModel: config.baseModel,
    epochs: config.epochs,
    batchSize: config.batchSize,
    imageSize: config.imageSize,
    deviceId: config.deviceId,
    workers: config.workers,
    trainSplit: config.trainSplit,
    valSplit: config.valSplit,
    hsvH: config.hsvH,
    hsvS: config.hsvS,
    hsvV: config.hsvV,
    translate: config.translate,
    scale: config.scale,
    shear: config.shear,
    perspective: config.perspective,
    flipud: config.flipud,
    fliplr: config.fliplr,
    mosaic: config.mosaic,
    mixup: config.mixup,
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

  startTraining: async (config) => {
    const { currentTrainingId } = get();
    if (currentTrainingId) {
      console.warn('Training already in progress');
      return;
    }

    try {
      // Convert progress callback to update metrics
      const onProgress = (progress: TrainingProgress) => {
        const metrics: TrainingMetrics = {
          epoch: progress.epoch,
          trainBoxLoss: progress.train_box_loss,
          trainClsLoss: progress.train_cls_loss,
          trainDflLoss: progress.train_dfl_loss,
          valBoxLoss: progress.val_box_loss,
          valClsLoss: progress.val_cls_loss,
          valDflLoss: progress.val_dfl_loss,
          precision: progress.precision,
          recall: progress.recall,
          map50: progress.map50,
          map50_95: progress.map50_95,
        };
        get().updateMetrics(metrics);
      };

      const response = await apiStartTraining('', toApiConfig(config), onProgress);

      if (!response.success || !response.data) {
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
      set({ isTraining: false, currentTrainingId: null });
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
}));
