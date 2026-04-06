import { useState } from 'react';
import { FolderOpen, X, FileText, Image, Tag } from 'lucide-react';
import { useWorkspaceStore } from '../../../core/stores/workspaceStore';
import { selectFolder } from '../../../core/api';

interface ImportDatasetModalProps {
  onClose: () => void;
  onImported?: () => void;
}

interface DatasetPreview {
  name: string;
  classes: string[];
  trainCount: number;
  valCount: number;
  yoloVersion: string;
}

export default function ImportDatasetModal({ onClose, onImported }: ImportDatasetModalProps) {
  const { importDataset } = useWorkspaceStore();
  const [path, setPath] = useState('');
  const [isImporting, setIsImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [preview, setPreview] = useState<DatasetPreview | null>(null);

  const handleSelectPath = async () => {
    const result = await selectFolder('选择 YOLO 数据集文件夹');
    if (!result.canceled && result.path) {
      setPath(result.path);
      setError(null);
      // Try to parse preview info (basic check)
      setPreview({
        name: result.path.split(/[/\\]/).pop() || '数据集',
        classes: [], // Will be filled after import
        trainCount: 0,
        valCount: 0,
        yoloVersion: 'yolo11',
      });
    }
  };

  const handleImport = async () => {
    if (!path) return;

    setIsImporting(true);
    setError(null);
    try {
      const success = await importDataset(path);
      if (success) {
        onImported?.();
        onClose();
      } else {
        setError('导入失败，请检查数据集格式是否正确');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : '导入失败');
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose} style={{ zIndex: 1100 }}>
      <div className="modal" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 480 }}>
        <div className="modal-header" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <h2 className="modal-title">导入 YOLO 数据集</h2>
          <button className="btn btn-ghost" style={{ padding: 4 }} onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          <div className="form-group">
            <label className="form-label">数据集路径 *</label>
            <div style={{ display: 'flex', gap: 'var(--spacing-sm)' }}>
              <input
                type="text"
                className="input"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                placeholder="选择包含 data.yaml 的 YOLO 数据集文件夹"
                readOnly
              />
              <button className="btn btn-secondary" onClick={handleSelectPath}>
                <FolderOpen size={16} />
                选择
              </button>
            </div>
          </div>

          {path && (
            <div className="info-box" style={{ marginTop: 'var(--spacing-md)' }}>
              <div className="info-box-icon">
                <FileText size={16} />
              </div>
              <div className="info-box-content">
                <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 'var(--spacing-sm)' }}>
                  支持以下数据集格式:
                </p>
                <ul style={{ fontSize: 12, color: 'var(--text-tertiary)', paddingLeft: 'var(--spacing-md)', lineHeight: 1.8 }}>
                  <li>包含 <code>data.yaml</code> 的 ultralytics 标准格式</li>
                  <li>包含 <code>project.yaml</code> 的项目格式</li>
                </ul>
                <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginTop: 'var(--spacing-sm)' }}>
                  数据集应包含: <code>images/train</code>, <code>images/val</code>, <code>labels/train</code>, <code>labels/val</code>
                </p>
              </div>
            </div>
          )}

          {error && (
            <div className="error-box" style={{ marginTop: 'var(--spacing-md)' }}>
              {error}
            </div>
          )}

          {preview && (
            <div className="preview-box" style={{ marginTop: 'var(--spacing-md)', padding: 'var(--spacing-md)', background: 'var(--bg-elevated)', borderRadius: 'var(--radius-md)' }}>
              <h4 style={{ fontSize: 13, marginBottom: 'var(--spacing-sm)', color: 'var(--text-primary)' }}>
                数据集预览
              </h4>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 'var(--spacing-sm)', fontSize: 12 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <Image size={14} style={{ color: 'var(--text-tertiary)' }} />
                  <span style={{ color: 'var(--text-secondary)' }}>名称:</span>
                  <span style={{ color: 'var(--text-primary)' }}>{preview.name}</span>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <Tag size={14} style={{ color: 'var(--text-tertiary)' }} />
                  <span style={{ color: 'var(--text-secondary)' }}>版本:</span>
                  <span style={{ color: 'var(--text-primary)' }}>{preview.yoloVersion}</span>
                </div>
              </div>
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button className="btn btn-ghost" onClick={onClose}>
            取消
          </button>
          <button
            className="btn btn-primary"
            onClick={handleImport}
            disabled={!path || isImporting}
          >
            {isImporting ? '导入中...' : '导入'}
          </button>
        </div>
      </div>
    </div>
  );
}
