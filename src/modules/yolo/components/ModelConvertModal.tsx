import { useState } from 'react';
import { X, FolderOpen, Loader2 } from 'lucide-react';
import { TrainedModel } from '../../../core/stores/trainingStore';

interface ModelConvertModalProps {
  model: TrainedModel;
  onClose: () => void;
}

export default function ModelConvertModal({ model, onClose }: ModelConvertModalProps) {
  const [modelPath, setModelPath] = useState(model.modelPath);
  const [modelType, setModelType] = useState(model.yoloVersion);
  const [targetPlatform, setTargetPlatform] = useState('rk3588');
  const [isConverting, setIsConverting] = useState(false);
  const [progress, setProgress] = useState(0);

  const handleConvert = async () => {
    setIsConverting(true);
    setProgress(0);

    // Simulate conversion progress
    const interval = setInterval(() => {
      setProgress((p) => {
        if (p >= 100) {
          clearInterval(interval);
          setIsConverting(false);
          onClose();
          return 100;
        }
        return p + 10;
      });
    }, 500);
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 440 }}>
        <div className="modal-header" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <h2 className="modal-title">模型转换</h2>
          <button className="btn btn-ghost" style={{ padding: 4 }} onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          <div className="form-group">
            <label className="form-label">模型路径</label>
            <div style={{ display: 'flex', gap: 'var(--spacing-sm)' }}>
              <input
                type="text"
                className="input"
                value={modelPath}
                onChange={(e) => setModelPath(e.target.value)}
                readOnly
              />
              <button className="btn btn-secondary">
                <FolderOpen size={16} />
              </button>
            </div>
          </div>

          <div className="form-group">
            <label className="form-label">模型类型</label>
            <select
              className="select"
              value={modelType}
              onChange={(e) => setModelType(e.target.value)}
              style={{ width: '100%' }}
            >
              <option value="yolo5">YOLOv5</option>
              <option value="yolo8">YOLOv8</option>
              <option value="yolo11">YOLO11</option>
            </select>
          </div>

          <div className="form-group">
            <label className="form-label">边缘平台</label>
            <select
              className="select"
              value={targetPlatform}
              onChange={(e) => setTargetPlatform(e.target.value)}
              style={{ width: '100%' }}
            >
              <option value="rk3588">瑞芯微 RK3588</option>
              <option value="rk3568">瑞芯微 RK3568</option>
              <option value="rk3566">瑞芯微 RK3566</option>
              <option value="aml-s905x">晶晨 S905X</option>
              <option value="aml-s912">晶晨 S912</option>
              <option value="hisi-3519">华为海思 3519</option>
              <option value="地平线">地平线 J3</option>
              <option value="tegra">NVIDIA Jetson</option>
            </select>
          </div>

          {isConverting && (
            <div className="form-group">
              <label className="form-label">转换进度: {progress}%</label>
              <div className="progress-bar" style={{ height: 8, marginTop: 8 }}>
                <div className="progress-fill" style={{ width: `${progress}%` }} />
              </div>
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button className="btn btn-ghost" onClick={onClose} disabled={isConverting}>
            取消
          </button>
          <button
            className="btn btn-primary"
            onClick={handleConvert}
            disabled={isConverting}
          >
            {isConverting ? (
              <>
                <Loader2 size={16} className="animate-spin" />
                转换中...
              </>
            ) : (
              '转换'
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
