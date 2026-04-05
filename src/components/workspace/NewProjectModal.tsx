import { useState } from 'react';
import { FolderOpen, X } from 'lucide-react';
import { useWorkspaceStore } from '../../stores/workspaceStore';

interface NewProjectModalProps {
  onClose: () => void;
  onCreated?: () => void;
}

export default function NewProjectModal({ onClose, onCreated }: NewProjectModalProps) {
  const { createProject, selectProjectPath } = useWorkspaceStore();
  const [name, setName] = useState('');
  const [path, setPath] = useState('');
  const [yoloVersion, setYoloVersion] = useState('yolo11');
  const [trainRatio, setTrainRatio] = useState(80);
  const [description, setDescription] = useState('');
  const [classes, setClasses] = useState('person,car,dog,cat');
  const [isCreating, setIsCreating] = useState(false);

  const handleSelectPath = async () => {
    const selectedPath = await selectProjectPath();
    if (selectedPath) {
      setPath(selectedPath);
    }
  };

  const handleCreate = async () => {
    if (!name.trim() || !path) return;

    setIsCreating(true);
    try {
      const classList = classes.split(',').map((c) => c.trim()).filter(Boolean);
      await createProject(name.trim(), path, {
        classes: classList,
        train_split: trainRatio / 100,
        val_split: (100 - trainRatio) / 100,
        image_size: 640,
        yolo_version: yoloVersion,
        description: description.trim(),
      });
      onCreated?.();
      onClose();
    } finally {
      setIsCreating(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose} style={{ zIndex: 1100 }}>
      <div className="modal" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 520 }}>
        <div className="modal-header" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <h2 className="modal-title">新建项目</h2>
          <button className="btn btn-ghost" style={{ padding: 4 }} onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          <div className="form-group">
            <label className="form-label">项目名称 *</label>
            <input
              type="text"
              className="input"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="my-yolo-project"
              autoFocus
            />
          </div>

          <div className="form-group">
            <label className="form-label">存储路径 *</label>
            <div style={{ display: 'flex', gap: 'var(--spacing-sm)' }}>
              <input
                type="text"
                className="input"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                placeholder="D:\demo"
                readOnly
              />
              <button className="btn btn-secondary" onClick={handleSelectPath}>
                <FolderOpen size={16} />
                选择
              </button>
            </div>
          </div>

          <div className="form-group">
            <label className="form-label">YOLO 版本</label>
            <select
              className="select"
              value={yoloVersion}
              onChange={(e) => setYoloVersion(e.target.value)}
              style={{ width: '100%' }}
            >
              <option value="yolo5">YOLOv5</option>
              <option value="yolo8">YOLOv8</option>
              <option value="yolo11">YOLO11</option>
            </select>
          </div>

          <div className="form-group">
            <label className="form-label">检测类别 (逗号分隔)</label>
            <input
              type="text"
              className="input"
              value={classes}
              onChange={(e) => setClasses(e.target.value)}
              placeholder="person,car,dog,cat"
            />
          </div>

          <div className="form-group">
            <label className="form-label">数据集比例: {trainRatio}% / {100 - trainRatio}%</label>
            <input
              type="range"
              min="50"
              max="95"
              step="5"
              value={trainRatio}
              onChange={(e) => setTrainRatio(parseInt(e.target.value))}
              className="slider"
            />
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 12, color: 'var(--text-tertiary)', marginTop: 4 }}>
              <span>训练集</span>
              <span>验证集</span>
            </div>
          </div>

          <div className="form-group">
            <label className="form-label">项目描述 (选填)</label>
            <textarea
              className="textarea"
              rows={3}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="简要描述项目用途..."
            />
          </div>
        </div>

        <div className="modal-footer">
          <button className="btn btn-ghost" onClick={onClose}>
            取消
          </button>
          <button
            className="btn btn-primary"
            onClick={handleCreate}
            disabled={!name.trim() || !path || isCreating}
          >
            {isCreating ? '创建中...' : '确定'}
          </button>
        </div>
      </div>
    </div>
  );
}
