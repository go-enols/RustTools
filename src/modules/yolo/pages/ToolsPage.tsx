import {
  Image,
  RefreshCw,
  PenTool,
  Database,
  Cpu,
  Layers,
} from 'lucide-react';

const tools = [
  {
    icon: Image,
    title: '数据集预处理',
    description: '图像缩放、格式转换、数据增强',
    color: 'var(--accent-primary)',
  },
  {
    icon: Database,
    title: '数据格式转换',
    description: 'VOC/JSON/COCO格式互转',
    color: 'var(--status-success)',
  },
  {
    icon: Layers,
    title: '模型导出',
    description: '导出为ONNX/TensorRT格式',
    color: 'var(--status-warning)',
  },
  {
    icon: RefreshCw,
    title: '批量标注',
    description: '使用已有模型进行预标注',
    color: 'var(--accent-secondary)',
  },
  {
    icon: Cpu,
    title: '模型优化',
    description: '模型剪枝、量化、蒸馏',
    color: 'var(--status-error)',
  },
  {
    icon: PenTool,
    title: '标注格式转换',
    description: 'YOLO/VOC/COCO格式互转',
    color: 'var(--accent-tertiary)',
  },
];

export default function ToolsPage() {
  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div className="content-header">
        <h1 className="text-lg font-semibold">工具箱</h1>
        <p className="text-sm text-tertiary mt-sm">数据集处理、模型导出、格式转换等辅助工具</p>
      </div>

      {/* Tools Grid */}
      <div className="content-body">
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 'var(--spacing-lg)' }}>
          {tools.map((tool, i) => {
            const Icon = tool.icon;
            return (
              <div
                key={i}
                className="card"
                style={{ cursor: 'pointer', transition: 'all var(--transition-fast)' }}
              >
                <div
                  style={{
                    width: 48,
                    height: 48,
                    borderRadius: 'var(--radius-md)',
                    background: `${tool.color}15`,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    marginBottom: 'var(--spacing-md)',
                  }}
                >
                  <Icon size={24} style={{ color: tool.color }} />
                </div>
                <h3 style={{ fontSize: 15, fontWeight: 500, color: 'var(--text-primary)', marginBottom: 'var(--spacing-xs)' }}>
                  {tool.title}
                </h3>
                <p style={{ fontSize: 13, color: 'var(--text-tertiary)' }}>
                  {tool.description}
                </p>
              </div>
            );
          })}
        </div>

        {/* Additional Info */}
        <div className="card" style={{ marginTop: 'var(--spacing-2xl)' }}>
          <div className="card-header">
            <span className="card-title">使用提示</span>
          </div>
          <div style={{ fontSize: 13, color: 'var(--text-secondary)', display: 'flex', flexDirection: 'column', gap: 'var(--spacing-sm)' }}>
            <p>• 数据集预处理支持批量处理，可大幅减少人工操作时间</p>
            <p>• 模型导出功能支持主流边缘平台，包括瑞芯微、晶晨、地平线等</p>
            <p>• 批量标注功能需要先训练一个基础模型作为预标注模型</p>
            <p>• 格式转换支持YOLO、VOC、COCO三种主流标注格式互转</p>
          </div>
        </div>
      </div>
    </div>
  );
}
