import { PenTool, Train, BarChart3, Video, FolderOpen, Clock, HelpCircle } from 'lucide-react';
import { useWorkspaceStore } from '../../../core/stores/workspaceStore';
import PythonEnvCheck from '../components/PythonEnvCheck';

interface YoloHomePageProps {
  onNavigate: (page: 'annotation' | 'training' | 'results' | 'video' | 'settings') => void;
  onOpenHelp: () => void;
}

export default function YoloHomePage({ onNavigate, onOpenHelp }: YoloHomePageProps) {
  const { currentProject } = useWorkspaceStore();

  const quickActions = [
    { icon: PenTool, label: '数据标注', desc: '创建和编辑数据集标注', page: 'annotation' as const },
    { icon: Train, label: '模型训练', desc: '训练 YOLO 目标检测模型', page: 'training' as const },
    { icon: BarChart3, label: '训练结果', desc: '查看和分析训练结果', page: 'results' as const },
    { icon: Video, label: '视频推理', desc: '使用模型对视频进行推理', page: 'video' as const },
  ];

  return (
    <div className="yolo-home" style={{ padding: 'var(--spacing-xl)', height: '100%', overflow: 'auto' }}>
      {/* Welcome Header */}
      <div style={{ marginBottom: 'var(--spacing-2xl)' }}>
        <h1 style={{ fontSize: 24, fontWeight: 600, marginBottom: 'var(--spacing-sm)' }}>
          欢迎使用 YOLO 工具
        </h1>
        {currentProject && (
          <p style={{ color: 'var(--text-secondary)', fontSize: 14 }}>
            当前项目: <span style={{ color: 'var(--accent-primary)' }}>{currentProject.name}</span>
          </p>
        )}

      {/* Python Environment Check */}
      <div style={{
        marginBottom: 'var(--spacing-2xl)',
        background: 'var(--bg-elevated)',
        borderRadius: 'var(--radius-lg)',
        border: '1px solid var(--border-default)',
        overflow: 'hidden'
      }}>
        <PythonEnvCheck />
      </div>
      </div>

      {/* Quick Actions */}
      <div style={{ marginBottom: 'var(--spacing-2xl)' }}>
        <h2 style={{ fontSize: 14, color: 'var(--text-secondary)', marginBottom: 'var(--spacing-lg)', display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
          <FolderOpen size={16} />
          快速开始
        </h2>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', gap: 'var(--spacing-md)' }}>
          {quickActions.map((action) => {
            const Icon = action.icon;
            return (
              <div
                key={action.page}
                onClick={() => onNavigate(action.page)}
                style={{
                  padding: 'var(--spacing-lg)',
                  background: 'var(--bg-elevated)',
                  borderRadius: 'var(--radius-lg)',
                  cursor: 'pointer',
                  border: '1px solid var(--border-default)',
                  transition: 'all 0.2s',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  e.currentTarget.style.background = 'var(--bg-surface)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.borderColor = 'var(--border-default)';
                  e.currentTarget.style.background = 'var(--bg-elevated)';
                }}
              >
                <Icon size={24} style={{ color: 'var(--accent-primary)', marginBottom: 'var(--spacing-md)' }} />
                <div style={{ fontSize: 14, fontWeight: 500, marginBottom: 'var(--spacing-xs)' }}>
                  {action.label}
                </div>
                <div style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                  {action.desc}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Project Info */}
      {currentProject && (
        <div style={{ marginBottom: 'var(--spacing-2xl)' }}>
          <h2 style={{ fontSize: 14, color: 'var(--text-secondary)', marginBottom: 'var(--spacing-lg)', display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
            <Clock size={16} />
            项目信息
          </h2>
          <div
            style={{
              padding: 'var(--spacing-lg)',
              background: 'var(--bg-elevated)',
              borderRadius: 'var(--radius-lg)',
              border: '1px solid var(--border-default)',
            }}
          >
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(150px, 1fr))', gap: 'var(--spacing-md)' }}>
              <div>
                <div style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 4 }}>项目名称</div>
                <div style={{ fontSize: 14 }}>{currentProject.name}</div>
              </div>
              <div>
                <div style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 4 }}>YOLO 版本</div>
                <div style={{ fontSize: 14 }}>{currentProject.yoloVersion.toUpperCase()}</div>
              </div>
              <div>
                <div style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 4 }}>类别数</div>
                <div style={{ fontSize: 14 }}>{currentProject.classes.length}</div>
              </div>
              <div>
                <div style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 4 }}>训练集比例</div>
                <div style={{ fontSize: 14 }}>{currentProject.trainSplit}%</div>
              </div>
              <div>
                <div style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 4 }}>验证集比例</div>
                <div style={{ fontSize: 14 }}>{currentProject.valSplit}%</div>
              </div>
              <div>
                <div style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 4 }}>图片尺寸</div>
                <div style={{ fontSize: 14 }}>{currentProject.imageSize}px</div>
              </div>
            </div>
            <div style={{ marginTop: 'var(--spacing-md)', paddingTop: 'var(--spacing-md)', borderTop: '1px solid var(--border-default)' }}>
              <div style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 4 }}>项目路径</div>
              <div style={{ fontSize: 13, fontFamily: 'monospace' }}>{currentProject.path}</div>
            </div>
          </div>
        </div>
      )}

      {/* Help Link */}
      <div style={{ textAlign: 'center', marginTop: 'var(--spacing-2xl)' }}>
        <button
          onClick={onOpenHelp}
          className="btn btn-ghost"
          style={{ display: 'inline-flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}
        >
          <HelpCircle size={16} />
          查看快捷键
        </button>
      </div>
    </div>
  );
}
