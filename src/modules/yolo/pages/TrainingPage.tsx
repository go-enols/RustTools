import { useState, useEffect } from 'react';
import { Settings2 } from 'lucide-react';
import { useTrainingStore, TrainingConfig } from '../../../core/stores/trainingStore';
import { useWorkspaceStore } from '../../../core/stores/workspaceStore';
import { useToast } from '../../../shared/hooks/useToast';
import {
  checkModel,
  downloadModel,
} from '../../../core/api';
import { listen } from '@tauri-apps/api/event';
import { DownloadModal } from '../../../shared/components/ui/Modal';

import TrainingHeader from '../components/training/TrainingHeader';
import TrainingProgress from '../components/training/TrainingProgress';
import TrainingConfigForm from '../components/training/TrainingConfigForm';
import TrainingLogs from '../components/training/TrainingLogs';
import AdvancedConfig from '../components/training/AdvancedConfig';

import styles from './TrainingPage.module.css';

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
  const { isTraining, currentEpoch, totalEpochs, metrics, error, startTraining, stopTraining, clearError } =
    useTrainingStore();
  const [config, setConfig] = useState<TrainingConfig>(defaultConfig);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [elapsedTime, setElapsedTime] = useState('00:00:00');
  const [remainingTime, setRemainingTime] = useState('--:--:--');
  const [showDownloadModal, setShowDownloadModal] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState('');
  const [downloadError, setDownloadError] = useState('');
  const [cudaAvailable, setCudaAvailable] = useState(true);

  const toast = useToast();

  // Listen for model download progress
  useEffect(() => {
    const unlisten = listen<{ model: string; message: string }>('model-download-progress', (event) => {
      setDownloadProgress(event.payload.message);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for training started event (includes CUDA info from Python sidecar)
  useEffect(() => {
    const unlisten = listen<{
      training_id: string;
      cuda_available: boolean;
      cuda_version: string | null;
    }>('training-started', (event) => {
      const { cuda_available, cuda_version } = event.payload;
      console.log(
        '[TrainingPage] Received training-started event: cuda_available=%s, cuda_version=%s',
        cuda_available,
        cuda_version
      );
      setCudaAvailable(cuda_available);
    });
    return () => {
      unlisten.then((fn) => fn());
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
        setElapsedTime(
          `${hours.toString().padStart(2, '0')}:${mins.toString().padStart(2, '0')}:${secs
            .toString()
            .padStart(2, '0')}`
        );

        if (currentEpoch > 0) {
          const avgTimePerEpoch = diff / currentEpoch;
          const remaining = Math.floor(avgTimePerEpoch * (totalEpochs - currentEpoch));
          const remHours = Math.floor(remaining / 3600);
          const remMins = Math.floor((remaining % 3600) / 60);
          const remSecs = remaining % 60;
          setRemainingTime(
            `${remHours.toString().padStart(2, '0')}:${remMins.toString().padStart(2, '0')}:${remSecs
              .toString()
              .padStart(2, '0')}`
          );
        }
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [isTraining, currentEpoch, totalEpochs]);

  const handleStartTraining = async () => {
    // Check if a project is open
    const { currentProject } = useWorkspaceStore.getState();
    if (!currentProject) {
      toast.warning('请先打开一个项目才能开始训练');
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

    // Model exists, use the full path
    if (checkResult.success && checkResult.data?.exists && checkResult.data.path) {
      console.log(`[Training] 模型已存在: ${checkResult.data.path}`);
      modelPath = checkResult.data.path;
    }
    // Model doesn't exist or check failed, download it
    else {
      console.log(`[Training] 模型不存在或检查失败，开始下载: ${modelName}`);

      if (checkResult.error) {
        console.warn(`[Training] 检查模型失败: ${checkResult.error}，将尝试下载`);
      }

      setShowDownloadModal(true);
      setDownloadProgress(`正在下载 ${modelName}...`);
      setDownloadError('');

      const downloadResult = await downloadModel(modelName);

      if (downloadResult.success && downloadResult.data?.success) {
        console.log(`[Training] 模型下载成功: ${downloadResult.data.path}`);
        setShowDownloadModal(false);
        if (downloadResult.data?.path) {
          modelPath = downloadResult.data.path;
        }
      } else {
        const errorMsg = downloadResult.error || downloadResult.data?.error || '模型下载失败';
        console.error(`[Training] 模型下载失败: ${errorMsg}`);
        setDownloadError(errorMsg);
        return;
      }
    }

    // Start training with the resolved model path
    startTraining({ ...config, base_model: modelPath });
  };

  // 手动下载模型 - 统一使用后端镜像下载
  const handleManualDownload = async () => {
    const modelName = config.base_model;

    if (modelName.startsWith('custom:')) {
      toast.info('自定义模型无需下载');
      return;
    }

    // 直接调用后端下载，使用镜像源
    setShowDownloadModal(true);
    setDownloadProgress(`正在下载 ${modelName}...`);
    setDownloadError('');

    const result = await downloadModel(modelName);

    if (result.success && result.data?.success) {
      console.log(`[Training] 模型下载成功: ${result.data.path}`);
      setShowDownloadModal(false);
      toast.success(`模型下载成功！保存路径: ${result.data.path}`);
    } else {
      const errorMsg = result.error || result.data?.error || '模型下载失败';
      console.error(`[Training] 模型下载失败: ${errorMsg}`);
      setDownloadError(errorMsg);
      // 保持弹窗打开，让用户选择重试或关闭
    }
  };

  // 关闭下载弹窗
  const handleCloseDownloadModal = () => {
    setShowDownloadModal(false);
    setDownloadProgress('');
    setDownloadError('');
  };

  // 重试下载
  const handleRetryDownload = () => {
    handleManualDownload();
  };

  // Refresh CUDA status

  const batchProgress = useTrainingStore((s) => s.batchProgress);

  return (
    <div className={styles.pageContainer}>
      {/* Header */}
      <TrainingHeader
        isTraining={isTraining}
        onStartTraining={handleStartTraining}
        onStopTraining={stopTraining}
      />

      <div className={styles.mainContent}>
        <div className={styles.leftPanel}>
          {/* Training Controls */}
          <TrainingConfigForm
            config={config}
            onConfigChange={setConfig}
          />

          {/* Progress */}
          {isTraining && (
            <TrainingProgress
              currentEpoch={currentEpoch}
              totalEpochs={totalEpochs}
              elapsedTime={elapsedTime}
              remainingTime={remainingTime}
              metrics={metrics}
              batchProgress={batchProgress}
            />
          )}

          {/* Error Display */}
          {error && (
            <div className={styles.errorCard}>
              <div className={styles.errorHeader}>
                <div>
                  <div className={styles.errorTitle}>训练错误</div>
                  <div className={styles.errorMessage}>{error}</div>
                </div>
                <button onClick={clearError} className={styles.errorCloseBtn}>
                  ✕
                </button>
              </div>
            </div>
          )}

          {/* Log Panel */}
          <TrainingLogs metrics={metrics} />
        </div>

        {/* Right Panel */}
        <div className={styles.rightPanel}>
          {/* Basic Parameters */}
          <div className={styles.panelSection}>
            <div className={styles.panelSectionTitle}>
              <Settings2 size={14} />
              基础参数
            </div>
            <div className={styles.panelSectionContent}>
              <div>
                <label className={styles.inlineLabel}>图像大小</label>
                <input
                  type="number"
                  className={styles.input}
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
                <label className={styles.inlineLabel}>设备</label>
                <select
                  className={styles.select}
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

          {/* Advanced Configuration */}
          <AdvancedConfig
            config={config}
            showAdvanced={showAdvanced}
            onToggleAdvanced={() => setShowAdvanced(!showAdvanced)}
            onConfigChange={setConfig}
          />

          {/* Download Modal */}
          <DownloadModal
            isOpen={showDownloadModal}
            title="下载模型"
            message={`正在下载 ${config.base_model}...`}
            progress={downloadProgress}
            error={downloadError}
            onClose={handleCloseDownloadModal}
            onRetry={handleRetryDownload}
          />
        </div>
      </div>
    </div>
  );
}
