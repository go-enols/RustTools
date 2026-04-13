import { FolderOpen } from 'lucide-react';
import { TrainingConfig } from '../../../../core/stores/trainingStore';
import { selectFile } from '../../../../core/api';
import styles from '../../pages/TrainingPage.module.css';

const YOLO_MODELS = [
  {
    label: 'YOLOv8',
    options: ['yolov8n.pt', 'yolov8s.pt', 'yolov8m.pt', 'yolov8l.pt', 'yolov8x.pt'],
  },
  {
    label: 'YOLO11',
    options: ['yolo11n.pt', 'yolo11s.pt', 'yolo11m.pt', 'yolo11l.pt', 'yolo11x.pt'],
  },
  {
    label: 'YOLO12',
    options: ['yolo12n.pt', 'yolo12s.pt', 'yolo12m.pt', 'yolo12l.pt', 'yolo12x.pt'],
  },
];

interface TrainingConfigFormProps {
  config: TrainingConfig;
  onConfigChange: (config: TrainingConfig) => void;
}

export default function TrainingConfigForm({ config, onConfigChange }: TrainingConfigFormProps) {
  const handleModelChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    onConfigChange({ ...config, base_model: e.target.value });
  };

  const handleCustomModelPathChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    onConfigChange({ ...config, base_model: `custom:${e.target.value}` });
  };

  const handleBrowseModel = async () => {
    const result = await selectFile('选择模型文件', [{ name: 'YOLO Model', extensions: ['pt', 'pth'] }]);
    if (!result.canceled && result.path) {
      onConfigChange({ ...config, base_model: `custom:${result.path}` });
    }
  };

  const handleEpochsChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    if (val === '') {
      onConfigChange({ ...config, epochs: 0 });
    } else {
      const num = parseInt(val);
      if (!isNaN(num)) {
        onConfigChange({ ...config, epochs: num });
      }
    }
  };

  const handleEpochsBlur = () => {
    if (config.epochs === 0) {
      onConfigChange({ ...config, epochs: 50 });
    }
  };

  const handleBatchSizeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.value === '') {
      onConfigChange({ ...config, batch_size: 0 });
    } else {
      const num = parseInt(e.target.value);
      if (!isNaN(num)) onConfigChange({ ...config, batch_size: num });
    }
  };

  const handleBatchSizeBlur = () => {
    if (config.batch_size === 0) {
      onConfigChange({ ...config, batch_size: 12 });
    }
  };

  return (
    <div className={styles.controls}>
      {/* Model Selector */}
      <div className={styles.controlItem}>
        <span className={styles.controlLabel}>基础模型</span>
        <div className={styles.controlRow}>
          <select className={styles.select} value={config.base_model} onChange={handleModelChange}
            style={{ minWidth: 200 }}>
            {YOLO_MODELS.map((group) => (
              <optgroup key={group.label} label={group.label}>
                {group.options.map((model) => (
                  <option key={model} value={model}>
                    {model}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>

          {config.base_model.startsWith('custom:') && (
            <input type="text" className={styles.input}
              placeholder="输入自定义模型路径"
              value={config.base_model.replace('custom:', '')}
              onChange={handleCustomModelPathChange}
              style={{ minWidth: 300 }} />
          )}

          <button className={`${styles.btn} ${styles.btnSecondary}`} onClick={handleBrowseModel}>
            <FolderOpen size={14} />
          </button>
        </div>
      </div>

      {/* Epochs */}
      <div className={styles.controlItem}>
        <span className={styles.controlLabel}>训练轮次</span>
        <input type="number" className={`${styles.input} ${styles.inputSmall}`}
          value={config.epochs === 0 ? '' : config.epochs}
          onChange={handleEpochsChange}
          onBlur={handleEpochsBlur} />
      </div>

      {/* Batch Size */}
      <div className={styles.controlItem}>
        <span className={styles.controlLabel}>批处理</span>
        <input type="number" className={`${styles.input} ${styles.inputXSmall}`}
          value={config.batch_size === 0 ? '' : config.batch_size}
          onChange={handleBatchSizeChange}
          onBlur={handleBatchSizeBlur} />
      </div>
    </div>
  );
}
