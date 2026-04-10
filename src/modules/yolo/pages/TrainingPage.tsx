import { useState, useEffect } from 'react';
import {
  Play,
  Square,
  Settings2,
  ChevronDown,
  ChevronUp,
  RefreshCw,
  FolderOpen,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useTrainingStore, TrainingConfig } from '../../../core/stores/trainingStore';
import { useWorkspaceStore } from '../../../core/stores/workspaceStore';
import {
  checkModel,
  downloadModel,
} from '../../../core/api';
import { DialogResult } from '../../../core/api/types';
import { listen } from '@tauri-apps/api/event';
import { DownloadModal } from '../../../shared/components/ui/Modal';

interface PythonEnvInfo {
  python_exists: boolean;
  python_version: string | null;
  torch_exists: boolean;
  torch_version: string | null;
  torchaudio_exists: boolean;
  cuda_available: boolean;
  cuda_version: string | null;
  ultralytics_exists: boolean;
  ultralytics_version: string | null;
  yolo_command_exists: boolean;
}

// YOLO model download URLs
const MODEL_DOWNLOAD_URLS: Record<string, string> = {
  // YOLO11 (最新一代)
  'yolo11n.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolo11n.pt',
  'yolo11s.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolo11s.pt',
  'yolo11m.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolo11m.pt',
  'yolo11l.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolo11l.pt',
  'yolo11x.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolo11x.pt',
  // YOLOv10
  'yolo10n.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolo10n.pt',
  'yolo10s.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolo10s.pt',
  'yolo10m.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolo10m.pt',
  'yolo10b.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolo10b.pt',
  'yolo10l.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolo10l.pt',
  'yolo10x.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolo10x.pt',
  // YOLOv9
  'yolov9t.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolov9t.pt',
  'yolov9s.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolov9s.pt',
  'yolov9m.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolov9m.pt',
  'yolov9c.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolov9c.pt',
  'yolov9e.pt': 'https://github.com/ultralytics/assets/releases/download/v0.0.0/yolov9e.pt',
  // YOLOv8 (8.3.0)
  'yolov8n.pt': 'https://github.com/ultralytics/assets/releases/download/v8.3.0/yolov8n.pt',
  'yolov8s.pt': 'https://github.com/ultralytics/assets/releases/download/v8.3.0/yolov8s.pt',
  'yolov8m.pt': 'https://github.com/ultralytics/assets/releases/download/v8.3.0/yolov8m.pt',
  'yolov8l.pt': 'https://github.com/ultralytics/assets/releases/download/v8.3.0/yolov8l.pt',
  'yolov8x.pt': 'https://github.com/ultralytics/assets/releases/download/v8.3.0/yolov8x.pt',
  'yolov8n6.pt': 'https://github.com/ultralytics/assets/releases/download/v8.3.0/yolov8n6.pt',
  'yolov8s6.pt': 'https://github.com/ultralytics/assets/releases/download/v8.3.0/yolov8s6.pt',
  'yolov8m6.pt': 'https://github.com/ultralytics/assets/releases/download/v8.3.0/yolov8m6.pt',
  'yolov8l6.pt': 'https://github.com/ultralytics/assets/releases/download/v8.3.0/yolov8l6.pt',
  'yolov8x6.pt': 'https://github.com/ultralytics/assets/releases/download/v8.3.0/yolov8x6.pt',
  // YOLOv6
  'yolov6n.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov6n.pt',
  'yolov6s.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov6s.pt',
  'yolov6m.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov6m.pt',
  'yolov6l.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov6l.pt',
  'yolov6x.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov6x.pt',
  // YOLOv5
  'yolov5n.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov5n.pt',
  'yolov5s.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov5s.pt',
  'yolov5m.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov5m.pt',
  'yolov5l.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov5l.pt',
  'yolov5x.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov5x.pt',
  'yolov5n6.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov5n6.pt',
  'yolov5s6.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov5s6.pt',
  'yolov5m6.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov5m6.pt',
  'yolov5l6.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov5l6.pt',
  'yolov5x6.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov5x6.pt',
  // YOLOv3
  'yolov3u.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov3u.pt',
  'yolov3-spp.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov3-spp.pt',
  'yolov3-tiny.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov3-tiny.pt',
  'yolov3-son.pt': 'https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov3-son.pt',
};

const defaultConfig: TrainingConfig = {
  base_model: 'yolo11s.pt',
  epochs: 50,
  patience: 50,
  batch_size: 12,
  image_size: 640,
  device_id: 0,
  workers: 8,
  optimizer: 'SGD',
  lr0: 0.01,
  lrf: 0.01,
  momentum: 0.937,
  weight_decay: 0.0005,
  warmup_epochs: 3.0,
  warmup_bias_lr: 0.1,
  warmup_momentum: 0.8,
  hsv_h: 0.25,
  hsv_s: 0.25,
  hsv_v: 0.25,
  translate: 0.1,
  scale: 0.5,
  shear: 0.0,
  perspective: 0.0,
  flipud: 0.0,
  fliplr: 0.5,
  mosaic: 1.0,
  mixup: 0.0,
  copy_paste: 0.0,
  close_mosaic: 10,
  rect: false,
  cos_lr: false,
  single_cls: false,
  amp: true,
  save_period: -1,
  cache: false,
};

export default function TrainingPage() {
  const { isTraining, currentEpoch, totalEpochs, metrics, error, startTraining, stopTraining, clearError } = useTrainingStore();
  const [config, setConfig] = useState<TrainingConfig>(defaultConfig);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [elapsedTime, setElapsedTime] = useState('00:00:00');
  const [remainingTime, setRemainingTime] = useState('--:--:--');
  const [showDownloadModal, setShowDownloadModal] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState('');
  const [downloadError, setDownloadError] = useState('');
  const [cudaAvailable, setCudaAvailable] = useState(true); // 默认启用 GPU，因为用户 PyTorch CUDA 正常

  // Listen for model download progress
  useEffect(() => {
    const unlisten = listen<{ model: string; message: string }>('model-download-progress', (event) => {
      setDownloadProgress(event.payload.message);
    });
    return () => {
      unlisten.then(fn => fn());
    };
  }, []);

  // Listen for training started event (includes CUDA info from Python sidecar)
  useEffect(() => {
    const unlisten = listen<{ training_id: string; cuda_available: boolean; cuda_version: string | null }>('training-started', (event) => {
      const { cuda_available, cuda_version } = event.payload;
      console.log('[TrainingPage] Received training-started event: cuda_available=%s, cuda_version=%s', cuda_available, cuda_version);
      setCudaAvailable(cuda_available);
    });
    return () => {
      unlisten.then(fn => fn());
    };
  }, []);

  // Update time display during training
  useEffect(() => {
    if (!isTraining) return;

    const interval = setInterval(() => {
      // Update elapsed time
      const elapsed = new Date();
      const start = useTrainingStore.getState().startTime;
      if (start) {
        const diff = Math.floor((elapsed.getTime() - start.getTime()) / 1000);
        const hours = Math.floor(diff / 3600);
        const mins = Math.floor((diff % 3600) / 60);
        const secs = diff % 60;
        setElapsedTime(`${hours.toString().padStart(2, '0')}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`);

        if (currentEpoch > 0) {
          const avgTimePerEpoch = diff / currentEpoch;
          const remaining = Math.floor(avgTimePerEpoch * (totalEpochs - currentEpoch));
          const remHours = Math.floor(remaining / 3600);
          const remMins = Math.floor((remaining % 3600) / 60);
          const remSecs = remaining % 60;
          setRemainingTime(`${remHours.toString().padStart(2, '0')}:${remMins.toString().padStart(2, '0')}:${remSecs.toString().padStart(2, '0')}`);
        }
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [isTraining, currentEpoch, totalEpochs]);

  const handleStartTraining = async () => {
    // Check if a project is open
    const { currentProject } = useWorkspaceStore.getState();
    if (!currentProject) {
      alert('请先打开一个项目才能开始训练');
      return;
    }

    const modelName = config.base_model;

    // For custom models, skip check and use directly
    if (modelName.startsWith('custom:')) {
      startTraining(config);
      return;
    }

    // First check if model exists
    const checkResult = await checkModel(modelName);

    let modelPath = modelName;
    if (checkResult.success && checkResult.data) {
      if (!checkResult.data.exists) {
        // Model doesn't exist, need to download
        setShowDownloadModal(true);
        setDownloadProgress(`正在检查模型 ${modelName}...`);
        setDownloadError('');

        const downloadResult = await downloadModel(modelName);

        if (!downloadResult.success || !downloadResult.data?.success) {
          const errorMsg = downloadResult.error || downloadResult.data?.error || '模型下载失败';
          setDownloadError(errorMsg);
          return;
        }

        setShowDownloadModal(false);
        // Use the downloaded model path
        if (downloadResult.data?.path) {
          modelPath = downloadResult.data.path;
        }
      } else if (checkResult.data.path) {
        // Model exists, use the full path
        modelPath = checkResult.data.path;
      }
    } else {
      console.warn('检查模型失败，将尝试直接开始训练:', checkResult.error);
    }

    // Start training with the resolved model path
    startTraining({ ...config, base_model: modelPath });
  };

  const handleManualDownload = () => {
    const url = MODEL_DOWNLOAD_URLS[config.base_model];
    if (url) {
      window.open(url, '_blank');
    }
  };

  const batchProgress = useTrainingStore((s) => s.batchProgress);

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      {/* Header */}
      <div className="content-header">
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div>
            <h1 className="text-lg font-semibold">模型训练</h1>
            <p className="text-sm text-tertiary mt-sm">训练YOLO目标检测模型</p>
          </div>
          <div style={{ display: 'flex', gap: 'var(--spacing-md)', alignItems: 'center' }}>
            {!isTraining ? (
              <button className="btn btn-primary" onClick={handleStartTraining}>
                <Play size={16} />
                开始训练
              </button>
            ) : (
              <button className="btn btn-danger" onClick={stopTraining}>
                <Square size={16} />
                停止训练
              </button>
            )}
          </div>
        </div>
      </div>

      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        <div style={{ flex: 1, overflow: 'auto', padding: 'var(--spacing-lg)' }}>
          {/* Training Controls */}
          <div className="training-controls" style={{ marginBottom: 'var(--spacing-lg)' }}>
            <div style={{ flex: 1, display: 'flex', alignItems: 'center', gap: 'var(--spacing-xl)' }}>
              <div>
                <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>基础模型</span>
                <select
                  className="select"
                  value={config.base_model}
                  onChange={(e) => setConfig({ ...config, base_model: e.target.value })}
                  style={{ marginLeft: 8, minWidth: 200 }}
                >
                  <optgroup label="YOLO11 (最新)">
                    <option value="yolo11n.pt">YOLO11n - 最小/最快</option>
                    <option value="yolo11s.pt">YOLO11s - 小/快</option>
                    <option value="yolo11m.pt">YOLO11m - 中/平衡</option>
                    <option value="yolo11l.pt">YOLO11l - 大/准</option>
                    <option value="yolo11x.pt">YOLO11x - 最大/最准</option>
                  </optgroup>
                  <optgroup label="YOLOv10">
                    <option value="yolo10n.pt">YOLOv10n - 最小/最快</option>
                    <option value="yolo10s.pt">YOLOv10s - 小/快</option>
                    <option value="yolo10m.pt">YOLOv10m - 中/平衡</option>
                    <option value="yolo10b.pt">YOLOv10b - 中/双头</option>
                    <option value="yolo10l.pt">YOLOv10l - 大/准</option>
                    <option value="yolo10x.pt">YOLOv10x - 最大/最准</option>
                  </optgroup>
                  <optgroup label="YOLOv9">
                    <option value="yolov9t.pt">YOLOv9t - 最小/最快</option>
                    <option value="yolov9s.pt">YOLOv9s - 小/快</option>
                    <option value="yolov9m.pt">YOLOv9m - 中/平衡</option>
                    <option value="yolov9c.pt">YOLOv9c - 平衡/紧凑</option>
                    <option value="yolov9e.pt">YOLOv9e - 最大/最准</option>
                  </optgroup>
                  <optgroup label="YOLOv6">
                    <option value="yolov6n.pt">YOLOv6n - 最小/最快</option>
                    <option value="yolov6s.pt">YOLOv6s - 小/快</option>
                    <option value="yolov6m.pt">YOLOv6m - 中/平衡</option>
                    <option value="yolov6l.pt">YOLOv6l - 大/准</option>
                    <option value="yolov6x.pt">YOLOv6x - 最大/最准</option>
                  </optgroup>
                  <optgroup label="YOLOv8">
                    <option value="yolov8n.pt">YOLOv8n - 最小/最快</option>
                    <option value="yolov8s.pt">YOLOv8s - 小/快</option>
                    <option value="yolov8m.pt">YOLOv8m - 中/平衡</option>
                    <option value="yolov8l.pt">YOLOv8l - 大/准</option>
                    <option value="yolov8x.pt">YOLOv8x - 最大/最准</option>
                    <option value="yolov8n6.pt">YOLOv8n6 - 640输入/最小</option>
                    <option value="yolov8s6.pt">YOLOv8s6 - 640输入/小</option>
                    <option value="yolov8m6.pt">YOLOv8m6 - 640输入/中</option>
                    <option value="yolov8l6.pt">YOLOv8l6 - 640输入/大</option>
                    <option value="yolov8x6.pt">YOLOv8x6 - 640输入/最大</option>
                  </optgroup>
                  <optgroup label="YOLOv5">
                    <option value="yolov5n.pt">YOLOv5n - 最小/最快</option>
                    <option value="yolov5s.pt">YOLOv5s - 小/快</option>
                    <option value="yolov5m.pt">YOLOv5m - 中/平衡</option>
                    <option value="yolov5l.pt">YOLOv5l - 大/准</option>
                    <option value="yolov5x.pt">YOLOv5x - 最大/最准</option>
                    <option value="yolov5n6.pt">YOLOv5n6 - 1280输入/最小</option>
                    <option value="yolov5s6.pt">YOLOv5s6 - 1280输入/小</option>
                    <option value="yolov5m6.pt">YOLOv5m6 - 1280输入/中</option>
                    <option value="yolov5l6.pt">YOLOv5l6 - 1280输入/大</option>
                    <option value="yolov5x6.pt">YOLOv5x6 - 1280输入/最大</option>
                  </optgroup>
                  <optgroup label="YOLOv3">
                    <option value="yolov3u.pt">YOLOv3u - 原版升级</option>
                    <option value="yolov3-spp.pt">YOLOv3-spp - SPP层增强</option>
                    <option value="yolov3-tiny.pt">YOLOv3-tiny - 极小/最快</option>
                    <option value="yolov3-son.pt">YOLOv3-son - 改进版</option>
                  </optgroup>
                </select>
              </div>
              {config.base_model.startsWith('custom:') && (
                <input
                  type="text"
                  className="input"
                  placeholder="输入自定义模型路径，如 C:\Users\xxx\model.pt"
                  value={config.base_model.replace('custom:', '')}
                  onChange={(e) => setConfig({ ...config, base_model: `custom:${e.target.value}` })}
                  style={{ marginLeft: 8, minWidth: 300 }}
                />
              )}
              <button
                className="btn btn-secondary"
                onClick={async () => {
                  try {
                    const result = await invoke<DialogResult>('open_file_dialog', {
                      title: '选择模型文件',
                      filters: [{ name: 'YOLO Model', extensions: ['pt', 'pth'] }],
                    });
                    if (!result.canceled && result.paths && result.paths.length > 0) {
                      setConfig({ ...config, base_model: `custom:${result.paths[0]}` });
                    }
                  } catch (e) {
                    console.error('Failed to open file dialog:', e);
                  }
                }}
                style={{ marginLeft: 8 }}
              >
                <FolderOpen size={14} />
              </button>
              <div>
                <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>训练轮次</span>
                <input
                  type="number"
                  className="input"
                  value={config.epochs === 0 ? '' : config.epochs}
                  onChange={(e) => {
                    const val = e.target.value;
                    if (val === '') {
                      setConfig({ ...config, epochs: 0 });
                    } else {
                      const num = parseInt(val);
                      if (!isNaN(num)) {
                        setConfig({ ...config, epochs: num });
                      }
                    }
                  }}
                  onBlur={() => {
                    if (config.epochs === 0) {
                      setConfig({ ...config, epochs: 50 });
                    }
                  }}
                  style={{ width: 80, marginLeft: 8, textAlign: 'center' }}
                />
              </div>
              <div>
                <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>批处理</span>
                <input
                  type="number"
                  className="input"
                  value={config.batch_size === 0 ? '' : config.batch_size}
                  onChange={(e) => {
                    if (e.target.value === '') {
                      setConfig({ ...config, batch_size: 0 });
                    } else {
                      const num = parseInt(e.target.value);
                      if (!isNaN(num)) setConfig({ ...config, batch_size: num });
                    }
                  }}
                  onBlur={() => {
                    if (config.batch_size === 0) setConfig({ ...config, batch_size: 12 });
                  }}
                  style={{ width: 60, marginLeft: 8, textAlign: 'center' }}
                />
              </div>
            </div>
          </div>

          {/* Progress */}
          {isTraining && (
            <div className="card" style={{ marginBottom: 'var(--spacing-lg)', border: '1px solid var(--accent-primary)', background: 'rgba(22, 119, 255, 0.05)' }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 'var(--spacing-sm)' }}>
                <span style={{ fontSize: 14, color: 'var(--text-primary)', fontWeight: 600 }}>
                  总进度 Epoch {currentEpoch} / {totalEpochs}
                </span>
                <span style={{ fontSize: 14, color: 'var(--accent-primary)', fontWeight: 600 }}>
                  {totalEpochs > 0 ? ((currentEpoch / totalEpochs) * 100).toFixed(1) : 0}%
                </span>
              </div>
              <div className="progress-bar" style={{ height: 16 }}>
                <div className="progress-fill" style={{ width: `${totalEpochs > 0 ? (currentEpoch / totalEpochs) * 100 : 0}%` }} />
              </div>

              {batchProgress && batchProgress.batch > 0 && (
                <>
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 'var(--spacing-sm)', marginTop: 'var(--spacing-md)' }}>
                    <span style={{ fontSize: 13, color: 'var(--text-secondary)', fontWeight: 500 }}>
                      当前 Epoch 进度 Batch {batchProgress.batch}{batchProgress.totalBatches > 0 ? ` / ${batchProgress.totalBatches}` : ''}
                    </span>
                    <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
                      {batchProgress.totalBatches > 0
                        ? `${((batchProgress.batch / batchProgress.totalBatches) * 100).toFixed(1)}%`
                        : `${batchProgress.batch} batches`}
                    </span>
                  </div>
                  <div className="progress-bar" style={{ height: 8, background: 'var(--border-default)' }}>
                    <div style={{
                      height: '100%',
                      width: `${batchProgress.totalBatches > 0 ? Math.min((batchProgress.batch / batchProgress.totalBatches) * 100, 100) : 0}%`,
                      background: batchProgress.totalBatches > 0 ? 'var(--accent-secondary, #10b981)' : 'var(--border-default)',
                      borderRadius: 'var(--radius-full)',
                      transition: 'width 0.3s'
                    }} />
                  </div>
                </>
              )}

              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 'var(--spacing-md)', marginTop: 'var(--spacing-lg)' }}>
                <div style={{ textAlign: 'center', padding: 'var(--spacing-md)', background: 'var(--bg-surface)', borderRadius: 8 }}>
                  <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>已用时间</div>
                  <div style={{ fontSize: 18, fontWeight: 600, color: 'var(--text-primary)' }}>{elapsedTime}</div>
                </div>
                <div style={{ textAlign: 'center', padding: 'var(--spacing-md)', background: 'var(--bg-surface)', borderRadius: 8 }}>
                  <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>预计剩余</div>
                  <div style={{ fontSize: 18, fontWeight: 600, color: 'var(--text-primary)' }}>{remainingTime}</div>
                </div>
                {metrics.length > 0 && (
                  <>
                    <div style={{ textAlign: 'center', padding: 'var(--spacing-md)', background: 'var(--bg-surface)', borderRadius: 8 }}>
                      <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>Box Loss</div>
                      <div style={{ fontSize: 18, fontWeight: 600, color: 'var(--text-primary)' }}>{metrics[metrics.length - 1].trainBoxLoss.toFixed(4)}</div>
                    </div>
                    <div style={{ textAlign: 'center', padding: 'var(--spacing-md)', background: 'var(--bg-surface)', borderRadius: 8 }}>
                      <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>Cls Loss</div>
                      <div style={{ fontSize: 18, fontWeight: 600, color: 'var(--text-primary)' }}>{metrics[metrics.length - 1].trainClsLoss.toFixed(4)}</div>
                    </div>
                  </>
                )}
                {batchProgress && (
                  <>
                    <div style={{ textAlign: 'center', padding: 'var(--spacing-md)', background: 'var(--bg-surface)', borderRadius: 8 }}>
                      <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>Box Loss</div>
                      <div style={{ fontSize: 18, fontWeight: 600, color: 'var(--text-primary)' }}>{batchProgress.boxLoss.toFixed(4)}</div>
                    </div>
                    <div style={{ textAlign: 'center', padding: 'var(--spacing-md)', background: 'var(--bg-surface)', borderRadius: 8 }}>
                      <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>Cls Loss</div>
                      <div style={{ fontSize: 18, fontWeight: 600, color: 'var(--text-primary)' }}>{batchProgress.clsLoss.toFixed(4)}</div>
                    </div>
                  </>
                )}
                {metrics.length > 0 && metrics[metrics.length - 1].map50 > 0 && (
                  <>
                    <div style={{ textAlign: 'center', padding: 'var(--spacing-md)', background: 'var(--bg-surface)', borderRadius: 8 }}>
                      <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>mAP50</div>
                      <div style={{ fontSize: 18, fontWeight: 600, color: 'var(--accent-primary)' }}>{(metrics[metrics.length - 1].map50 * 100).toFixed(1)}%</div>
                    </div>
                    <div style={{ textAlign: 'center', padding: 'var(--spacing-md)', background: 'var(--bg-surface)', borderRadius: 8 }}>
                      <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>Precision</div>
                      <div style={{ fontSize: 18, fontWeight: 600, color: 'var(--text-primary)' }}>{(metrics[metrics.length - 1].precision * 100).toFixed(1)}%</div>
                    </div>
                  </>
                )}
              </div>
            </div>
          )}

          {/* Error Display */}
          {error && (
            <div className="card" style={{ marginBottom: 'var(--spacing-lg)', border: '1px solid var(--color-danger, #ff4d4f)', background: 'rgba(255, 77, 79, 0.1)' }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <div>
                  <div style={{ fontSize: 14, color: 'var(--color-danger, #ff4d4f)', fontWeight: 500, marginBottom: 4 }}>
                    训练错误
                  </div>
                  <div style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
                    {error}
                  </div>
                </div>
                <button
                  onClick={clearError}
                  style={{
                    background: 'none',
                    border: 'none',
                    cursor: 'pointer',
                    padding: 8,
                    color: 'var(--text-tertiary)'
                  }}
                >
                  ✕
                </button>
              </div>
            </div>
          )}

          {/* Log Panel */}
          <div className="card" style={{ marginBottom: 'var(--spacing-lg)' }}>
            <div className="card-header">
              <span className="card-title">训练日志</span>
            </div>
            <div className="log-panel" style={{ maxHeight: 300, overflow: 'auto' }}>
              {metrics.length === 0 ? (
                <div className="log-entry">等待开始训练...</div>
              ) : (
                metrics.slice(-20).map((m, i) => (
                  <div key={i} className="log-entry">
                    Epoch {m.epoch}: box_loss={m.trainBoxLoss.toFixed(4)}, cls_loss={m.trainClsLoss.toFixed(4)}, mAP50={m.map50.toFixed(4)}, precision={m.precision.toFixed(4)}, recall={m.recall.toFixed(4)}
                  </div>
                ))
              )}
            </div>
          </div>
        </div>

        {/* Right Panel */}
        <div className="right-panel" style={{ width: 280, overflow: 'auto', borderLeft: '1px solid var(--border-default)', background: 'var(--bg-surface)' }}>
          <div className="panel-section">
            <div className="panel-section-title">
              <Settings2 size={14} />
              基础参数
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>图像大小</label>
                <input
                  type="number"
                  className="input"
                  value={config.image_size === 0 ? '' : config.image_size}
                  onChange={(e) => {
                    if (e.target.value === '') {
                      setConfig({ ...config, image_size: 0 });
                    } else {
                      const num = parseInt(e.target.value);
                      if (!isNaN(num)) setConfig({ ...config, image_size: num });
                    }
                  }}
                  onBlur={() => {
                    if (config.image_size === 0) setConfig({ ...config, image_size: 640 });
                  }}
                  style={{ marginTop: 4 }}
                />
              </div>
              <div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>设备</label>
                  <button
                    onClick={async () => {
                      try {
                        const result = await invoke<{ success: boolean; data: PythonEnvInfo | null }>('check_python_env');
                        if (result.success && result.data) {
                          setCudaAvailable(result.data.cuda_available);
                        }
                      } catch {
                        console.warn('Failed to check CUDA');
                      }
                    }}
                    style={{ background: 'none', border: 'none', cursor: 'pointer', padding: 2, color: 'var(--text-tertiary)', display: 'flex', alignItems: 'center' }}
                    title="重新检测 CUDA"
                  >
                    <RefreshCw size={12} />
                  </button>
                </div>
                <select
                  className="select"
                  value={config.device_id}
                  onChange={(e) => setConfig({ ...config, device_id: parseInt(e.target.value) })}
                  style={{ width: '100%', marginTop: 4 }}
                >
                  {cudaAvailable ? (
                    <>
                      <option value={0}>GPU 0</option>
                      <option value={1}>GPU 1</option>
                    </>
                  ) : (
                    <option value={-1}>CPU (CUDA 不可用)</option>
                  )}
                </select>
              </div>
            </div>
          </div>

          <div className="panel-section">
            <div
              className="panel-section-title"
              style={{ cursor: 'pointer' }}
              onClick={() => setShowAdvanced(!showAdvanced)}
            >
              <Settings2 size={14} />
              数据增强
              {showAdvanced ? <ChevronUp size={14} style={{ marginLeft: 'auto' }} /> : <ChevronDown size={14} style={{ marginLeft: 'auto' }} />}
            </div>
            {showAdvanced && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
                <div>
                  <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>HSV色调</label>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={config.hsv_h}
                    onChange={(e) => setConfig({ ...config, hsv_h: parseFloat(e.target.value) })}
                    className="slider"
                  />
                  <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.hsv_h.toFixed(2)}</span>
                </div>
                <div>
                  <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>HSV饱和度</label>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={config.hsv_s}
                    onChange={(e) => setConfig({ ...config, hsv_s: parseFloat(e.target.value) })}
                    className="slider"
                  />
                  <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.hsv_s.toFixed(2)}</span>
                </div>
                <div>
                  <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>HSV亮度</label>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={config.hsv_v}
                    onChange={(e) => setConfig({ ...config, hsv_v: parseFloat(e.target.value) })}
                    className="slider"
                  />
                  <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.hsv_v.toFixed(2)}</span>
                </div>
                <div>
                  <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>平移</label>
                  <input
                    type="range"
                    min="0"
                    max="0.5"
                    step="0.05"
                    value={config.translate}
                    onChange={(e) => setConfig({ ...config, translate: parseFloat(e.target.value) })}
                    className="slider"
                  />
                  <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.translate.toFixed(2)}</span>
                </div>
                <div>
                  <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>缩放</label>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={config.scale}
                    onChange={(e) => setConfig({ ...config, scale: parseFloat(e.target.value) })}
                    className="slider"
                  />
                  <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.scale.toFixed(2)}</span>
                </div>
                <div>
                  <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>翻转概率</label>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.1"
                    value={config.fliplr}
                    onChange={(e) => setConfig({ ...config, fliplr: parseFloat(e.target.value) })}
                    className="slider"
                  />
                  <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.fliplr.toFixed(1)}</span>
                </div>
                <div>
                  <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>Mosaic</label>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.1"
                    value={config.mosaic}
                    onChange={(e) => setConfig({ ...config, mosaic: parseFloat(e.target.value) })}
                    className="slider"
                  />
                  <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.mosaic.toFixed(1)}</span>
                </div>
                <div>
                  <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>MixUp</label>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.1"
                    value={config.mixup}
                    onChange={(e) => setConfig({ ...config, mixup: parseFloat(e.target.value) })}
                    className="slider"
                  />
                  <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.mixup.toFixed(1)}</span>
                </div>
              </div>
            )}
          </div>

          {/* Optimizer Parameters */}
          <div className="panel-section">
            <div className="panel-section-title">
              <Settings2 size={14} />
              优化器
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>优化器</label>
                <select
                  className="select"
                  value={config.optimizer}
                  onChange={(e) => setConfig({ ...config, optimizer: e.target.value as 'SGD' | 'Adam' | 'AdamW' })}
                  style={{ width: '100%', marginTop: 4 }}
                >
                  <option value="SGD">SGD</option>
                  <option value="Adam">Adam</option>
                  <option value="AdamW">AdamW</option>
                </select>
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>初始学习率</label>
                <input
                  type="number"
                  className="input"
                  value={config.lr0}
                  onChange={(e) => {
                    if (e.target.value === '') {
                      setConfig({ ...config, lr0: 0 });
                    } else {
                      const num = parseFloat(e.target.value);
                      if (!isNaN(num)) setConfig({ ...config, lr0: num });
                    }
                  }}
                  onBlur={() => {
                    if (config.lr0 === 0) setConfig({ ...config, lr0: 0.01 });
                  }}
                  step="0.001"
                  style={{ marginTop: 4 }}
                />
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>最终学习率因子</label>
                <input
                  type="number"
                  className="input"
                  value={config.lrf}
                  onChange={(e) => {
                    if (e.target.value === '') {
                      setConfig({ ...config, lrf: 0 });
                    } else {
                      const num = parseFloat(e.target.value);
                      if (!isNaN(num)) setConfig({ ...config, lrf: num });
                    }
                  }}
                  onBlur={() => {
                    if (config.lrf === 0) setConfig({ ...config, lrf: 0.01 });
                  }}
                  step="0.001"
                  style={{ marginTop: 4 }}
                />
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>动量</label>
                <input
                  type="number"
                  className="input"
                  value={config.momentum}
                  onChange={(e) => {
                    if (e.target.value === '') {
                      setConfig({ ...config, momentum: 0 });
                    } else {
                      const num = parseFloat(e.target.value);
                      if (!isNaN(num)) setConfig({ ...config, momentum: num });
                    }
                  }}
                  onBlur={() => {
                    if (config.momentum === 0) setConfig({ ...config, momentum: 0.937 });
                  }}
                  step="0.001"
                  style={{ marginTop: 4 }}
                />
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>权重衰减</label>
                <input
                  type="number"
                  className="input"
                  value={config.weight_decay}
                  onChange={(e) => {
                    if (e.target.value === '') {
                      setConfig({ ...config, weight_decay: 0 });
                    } else {
                      const num = parseFloat(e.target.value);
                      if (!isNaN(num)) setConfig({ ...config, weight_decay: num });
                    }
                  }}
                  onBlur={() => {
                    if (config.weight_decay === 0) setConfig({ ...config, weight_decay: 0.0005 });
                  }}
                  step="0.0001"
                  style={{ marginTop: 4 }}
                />
              </div>
            </div>
          </div>

          {/* Advanced Settings */}
          <div className="panel-section">
            <div className="panel-section-title">
              <Settings2 size={14} />
              高级设置
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
                <input
                  type="checkbox"
                  id="amp"
                  checked={config.amp}
                  onChange={(e) => setConfig({ ...config, amp: e.target.checked })}
                />
                <label htmlFor="amp" style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>混合精度 (AMP)</label>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
                <input
                  type="checkbox"
                  id="cos_lr"
                  checked={config.cos_lr}
                  onChange={(e) => setConfig({ ...config, cos_lr: e.target.checked })}
                />
                <label htmlFor="cos_lr" style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>余弦学习率</label>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
                <input
                  type="checkbox"
                  id="rect"
                  checked={config.rect}
                  onChange={(e) => setConfig({ ...config, rect: e.target.checked })}
                />
                <label htmlFor="rect" style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>矩形训练</label>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
                <input
                  type="checkbox"
                  id="cache"
                  checked={config.cache}
                  onChange={(e) => setConfig({ ...config, cache: e.target.checked })}
                />
                <label htmlFor="cache" style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>缓存图像</label>
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>早停耐心</label>
                <input
                  type="number"
                  className="input"
                  value={config.patience === 0 ? '' : config.patience}
                  onChange={(e) => {
                    if (e.target.value === '') {
                      setConfig({ ...config, patience: 0 });
                    } else {
                      const num = parseInt(e.target.value);
                      if (!isNaN(num)) setConfig({ ...config, patience: num });
                    }
                  }}
                  onBlur={() => {
                    if (config.patience === 0) setConfig({ ...config, patience: 50 });
                  }}
                  style={{ marginTop: 4 }}
                />
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>Mosaic关闭轮次</label>
                <input
                  type="number"
                  className="input"
                  value={config.close_mosaic === 0 ? '' : config.close_mosaic}
                  onChange={(e) => {
                    if (e.target.value === '') {
                      setConfig({ ...config, close_mosaic: 0 });
                    } else {
                      const num = parseInt(e.target.value);
                      if (!isNaN(num)) setConfig({ ...config, close_mosaic: num });
                    }
                  }}
                  onBlur={() => {
                    if (config.close_mosaic === 0) setConfig({ ...config, close_mosaic: 10 });
                  }}
                  style={{ marginTop: 4 }}
                />
              </div>
            </div>
          </div>

          {/* Download Modal */}
          <DownloadModal
            isOpen={showDownloadModal}
            title="下载模型"
            message={`正在下载模型 ${config.base_model}，请稍候...`}
            progress={downloadProgress}
            error={downloadError}
            downloadUrl={MODEL_DOWNLOAD_URLS[config.base_model]}
            onManualDownload={handleManualDownload}
          />
        </div> </div>
    </div>
  );
}
