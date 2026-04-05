import { useState } from 'react';
import {
  Search,
  Trash2,
  FolderOpen,
  Download,
  CheckCircle,
  XCircle,
} from 'lucide-react';
import { useTrainingStore, TrainedModel } from '../../../core/stores/trainingStore';
import ModelConvertModal from '../components/ModelConvertModal';

export default function ResultsPage() {
  const { trainedModels, removeTrainedModel } = useTrainingStore();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedModel, setSelectedModel] = useState<TrainedModel | null>(null);
  const [showConvert, setShowConvert] = useState(false);

  const filteredModels = trainedModels.filter((m) =>
    m.projectName.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const handleConvert = (model: TrainedModel) => {
    setSelectedModel(model);
    setShowConvert(true);
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div className="content-header">
        <h1 className="text-lg font-semibold">训练结果</h1>
        <p className="text-sm text-tertiary mt-sm">查看和管理训练生成的模型</p>
      </div>

      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        {/* Main Content - Model List */}
        <div style={{ flex: 1, padding: 'var(--spacing-lg)', overflow: 'auto' }}>
          {/* Search Bar */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-md)', marginBottom: 'var(--spacing-lg)' }}>
            <div style={{ position: 'relative', flex: 1, maxWidth: 300 }}>
              <Search
                size={16}
                style={{
                  position: 'absolute',
                  left: 12,
                  top: '50%',
                  transform: 'translateY(-50%)',
                  color: 'var(--text-tertiary)',
                }}
              />
              <input
                type="text"
                className="input"
                placeholder="搜索项目名称..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                style={{ paddingLeft: 36 }}
              />
            </div>
            {trainedModels.length > 0 && (
              <button className="btn btn-ghost">
                <Trash2 size={16} />
                清空历史
              </button>
            )}
          </div>

          {/* Model Table */}
          {filteredModels.length === 0 ? (
            <div className="empty-state">
              <CheckCircle size={48} style={{ color: 'var(--text-tertiary)' }} />
              <p style={{ marginTop: 'var(--spacing-md)' }}>暂无训练结果</p>
              <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginTop: 'var(--spacing-sm)' }}>
                请先在训练页面开始训练任务
              </p>
            </div>
          ) : (
            <table className="table">
              <thead>
                <tr>
                  <th>名称</th>
                  <th>YOLO版本</th>
                  <th>完成时间</th>
                  <th>耗时</th>
                  <th>Best Epoch</th>
                  <th>mAP50</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                {filteredModels.map((model) => (
                  <tr key={model.id}>
                    <td>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
                        <CheckCircle size={16} style={{ color: 'var(--status-success)' }} />
                        {model.projectName}
                      </div>
                    </td>
                    <td>
                      <span className="badge badge-blue">{model.yoloVersion}</span>
                    </td>
                    <td style={{ color: 'var(--text-tertiary)' }}>
                      {model.createdAt.toLocaleString()}
                    </td>
                    <td style={{ color: 'var(--text-tertiary)' }}>--</td>
                    <td>{model.bestEpoch}</td>
                    <td>
                      <span style={{ color: 'var(--status-success)', fontWeight: 500 }}>
                        {(model.map50 * 100).toFixed(1)}%
                      </span>
                    </td>
                    <td>
                      <div style={{ display: 'flex', gap: 'var(--spacing-sm)' }}>
                        <button className="btn btn-ghost" style={{ padding: '4px 8px' }}>
                          <FolderOpen size={14} />
                        </button>
                        <button
                          className="btn btn-primary"
                          style={{ padding: '4px 12px', fontSize: 12 }}
                          onClick={() => handleConvert(model)}
                        >
                          在线转换
                        </button>
                        <button
                          className="btn btn-ghost"
                          style={{ padding: '4px 8px', color: 'var(--status-error)' }}
                          onClick={() => removeTrainedModel(model.id)}
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {/* Right Panel - Model Details */}
        {selectedModel && (
          <div className="right-panel">
            <div className="panel-section">
              <div className="panel-section-title">
                <CheckCircle size={14} />
                Best Epoch 配置
              </div>
              <div style={{ fontSize: 13, color: 'var(--text-secondary)', display: 'flex', flexDirection: 'column', gap: 'var(--spacing-sm)' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <span>基础模型</span>
                  <span style={{ color: 'var(--text-primary)' }}>{selectedModel.yoloVersion}</span>
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <span>Epochs</span>
                  <span style={{ color: 'var(--text-primary)' }}>{selectedModel.totalEpochs}</span>
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <span>BatchSize</span>
                  <span style={{ color: 'var(--text-primary)' }}>12</span>
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <span>Workers</span>
                  <span style={{ color: 'var(--text-primary)' }}>8</span>
                </div>
              </div>
            </div>

            <div className="panel-section">
              <div className="panel-section-title">
                <Download size={14} />
                模型文件
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-sm)' }}>
                <div style={{ padding: 'var(--spacing-sm)', background: 'var(--bg-elevated)', borderRadius: 'var(--radius-sm)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div>
                    <div style={{ fontSize: 13, color: 'var(--text-primary)' }}>best.pt</div>
                    <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>24.5 MB</div>
                  </div>
                  <button className="btn btn-ghost" style={{ padding: 4 }}>
                    <FolderOpen size={14} />
                  </button>
                </div>
                <div style={{ padding: 'var(--spacing-sm)', background: 'var(--bg-elevated)', borderRadius: 'var(--radius-sm)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div>
                    <div style={{ fontSize: 13, color: 'var(--text-primary)' }}>last.pt</div>
                    <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>24.5 MB</div>
                  </div>
                  <button className="btn btn-ghost" style={{ padding: 4 }}>
                    <FolderOpen size={14} />
                  </button>
                </div>
              </div>
            </div>

            <button className="btn btn-primary" style={{ width: '100%' }} onClick={() => setShowConvert(true)}>
              <Download size={16} />
              在线转换
            </button>

            <div className="panel-section" style={{ marginTop: 'var(--spacing-lg)' }}>
              <div className="panel-section-title">
                <XCircle size={14} />
                边缘模型
              </div>
              <div className="empty-state" style={{ padding: 'var(--spacing-lg)' }}>
                <p style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>暂无边缘模型</p>
                <p style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 'var(--spacing-xs)' }}>转换后将显示在这里</p>
              </div>
            </div>
          </div>
        )}
      </div>

      {showConvert && selectedModel && (
        <ModelConvertModal
          model={selectedModel}
          onClose={() => setShowConvert(false)}
        />
      )}
    </div>
  );
}
