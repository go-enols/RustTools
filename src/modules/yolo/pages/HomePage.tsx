import { useState, useEffect } from 'react';
import {
  Plus,
  FolderOpen,
  Clock,
  HelpCircle,
  PenTool,
  Train,
  BarChart3,
  Video,
  Trash2,
  X,
  AlertCircle,
} from 'lucide-react';
import { useWorkspaceStore, Project } from '../../../core/stores/workspaceStore';
import NewProjectModal from '../components/NewProjectModal';
import HelpModal, { HelpType } from '../components/HelpModal';

interface HomePageProps {
  onNavigate: (page: 'annotation' | 'training' | 'results' | 'video') => void;
  onCloseProject?: () => void;
}

export default function HomePage({ onNavigate, onCloseProject }: HomePageProps) {
  const {
    recentProjects,
    clearRecentProjects,
    loadRecentProjects,
    openProject,
    openProjectFromPath,
    currentProject,
    error,
    setError,
  } = useWorkspaceStore();
  const [showNewProject, setShowNewProject] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [helpType, setHelpType] = useState<HelpType | null>(null);
  const [showError, setShowError] = useState(false);

  useEffect(() => {
    loadRecentProjects();
  }, [loadRecentProjects]);

  useEffect(() => {
    if (error) {
      setShowError(true);
    }
  }, [error]);

  const handleOpenProject = async () => {
    setIsLoading(true);
    try {
      // Use workspace store's selectProjectPath
      const { selectProjectPath } = useWorkspaceStore.getState();
      const path = await selectProjectPath();
      if (path) {
        const success = await openProjectFromPath(path);
        if (!success) {
          // Error is already set in store by openProjectFromPath
        }
      }
    } finally {
      setIsLoading(false);
    }
  };

  const handleDismissError = () => {
    setShowError(false);
    setError(null);
  };

  const handleProjectClick = (project: Project) => {
    openProject(project);
  };

  const quickActions = [
    { icon: Plus, label: '新建项目', hint: 'Ctrl+N', action: () => setShowNewProject(true) },
    { icon: FolderOpen, label: '打开项目', hint: 'Ctrl+O', action: handleOpenProject },
    { icon: Clock, label: '最近项目', hint: '', action: () => {} },
    { icon: HelpCircle, label: '使用帮助', hint: 'F1', action: () => setHelpType('shortcuts') },
  ];


  const pageShortcuts = [
    { icon: PenTool, label: '数据标注', desc: '创建和编辑数据集标注', page: 'annotation' as const },
    { icon: Train, label: '模型训练', desc: '训练YOLO目标检测模型', page: 'training' as const },
    { icon: BarChart3, label: '训练结果', desc: '查看和分析训练结果', page: 'results' as const },
    { icon: Video, label: '视频推理', desc: '使用模型对视频进行推理', page: 'video' as const },
  ];

  return (
    <>
      <div className="content-body">
        {/* Error Toast */}
        {showError && error && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 'var(--spacing-md)',
              padding: 'var(--spacing-md) var(--spacing-lg)',
              background: 'var(--status-error)',
              borderRadius: 'var(--radius-md)',
              marginBottom: 'var(--spacing-lg)',
              color: 'white',
            }}
          >
            <AlertCircle size={20} />
            <span style={{ flex: 1, fontSize: 14 }}>{error}</span>
            <button
              onClick={handleDismissError}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                width: 24,
                height: 24,
                border: 'none',
                background: 'transparent',
                color: 'white',
                cursor: 'pointer',
                borderRadius: 'var(--radius-sm)',
                opacity: 0.8,
              }}
              title="关闭"
            >
              <X size={16} />
            </button>
          </div>
        )}

        {/* Current Project Banner */}
        {currentProject && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 'var(--spacing-md)',
              padding: 'var(--spacing-md) var(--spacing-lg)',
              background: 'var(--bg-elevated)',
              borderRadius: 'var(--radius-md)',
              marginBottom: 'var(--spacing-xl)',
              border: '1px solid var(--border-default)',
            }}
          >
            <div
              style={{
                width: 8,
                height: 8,
                borderRadius: '50%',
                background: 'var(--status-success)',
              }}
            />
            <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
              当前项目:
            </span>
            <span style={{ fontSize: 14, color: 'var(--text-primary)', fontWeight: 500 }}>
              {currentProject.name}
            </span>
            <span
              style={{
                fontSize: 12,
                color: 'var(--text-tertiary)',
                padding: '2px 8px',
                background: 'var(--bg-surface)',
                borderRadius: 'var(--radius-full)',
              }}
            >
              {currentProject.yoloVersion.toUpperCase()}
            </span>
            <span style={{ fontSize: 12, color: 'var(--text-tertiary)', marginLeft: 'auto' }}>
              {currentProject.path}
            </span>
            {onCloseProject && (
              <button
                onClick={onCloseProject}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  width: 28,
                  height: 28,
                  border: 'none',
                  background: 'transparent',
                  color: 'var(--text-tertiary)',
                  cursor: 'pointer',
                  borderRadius: 'var(--radius-sm)',
                  marginLeft: 'var(--spacing-sm)',
                }}
                title="关闭项目"
              >
                <X size={16} />
              </button>
            )}
          </div>
        )}

        <div className="quick-actions">
          {quickActions.map((action, i) => {
            const Icon = action.icon;
            return (
              <div
                key={i}
                className="quick-action"
                onClick={action.action}
                style={{ opacity: isLoading && i === 1 ? 0.6 : 1 }}
              >
                <Icon size={24} />
                <div>
                  <div className="quick-action-text">{action.label}</div>
                  {action.hint && <div className="quick-action-hint">{action.hint}</div>}
                </div>
              </div>
            );
          })}
        </div>

        <div style={{ marginBottom: 'var(--spacing-xl)' }}>
          <h3
            style={{
              fontSize: 14,
              color: 'var(--text-secondary)',
              marginBottom: 'var(--spacing-lg)',
              display: 'flex',
              alignItems: 'center',
              gap: 'var(--spacing-sm)',
            }}
          >
            <BarChart3 size={16} />
            快捷入口
          </h3>
          <div className="grid grid-cols-4 gap-md">
            {pageShortcuts.map((item) => {
              const Icon = item.icon;
              return (
                <div
                  key={item.page}
                  className="card"
                  style={{ cursor: 'pointer' }}
                  onClick={() => onNavigate(item.page)}
                >
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 'var(--spacing-md)',
                      marginBottom: 'var(--spacing-sm)',
                    }}
                  >
                    <Icon size={20} style={{ color: 'var(--accent-primary)' }} />
                    <span style={{ fontWeight: 500 }}>{item.label}</span>
                  </div>
                  <p style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>{item.desc}</p>
                </div>
              );
            })}
          </div>
        </div>

        <div>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              marginBottom: 'var(--spacing-lg)',
            }}
          >
            <h3
              style={{
                fontSize: 14,
                color: 'var(--text-secondary)',
                display: 'flex',
                alignItems: 'center',
                gap: 'var(--spacing-sm)',
              }}
            >
              <Clock size={16} />
              最近项目
            </h3>
            {recentProjects.length > 0 && (
              <button
                className="btn btn-ghost"
                style={{ fontSize: 12, padding: '4px 8px' }}
                onClick={clearRecentProjects}
              >
                <Trash2 size={14} />
                清空历史
              </button>
            )}
          </div>

          {recentProjects.length === 0 ? (
            <div
              className="empty-state"
              style={{
                padding: 'var(--spacing-2xl)',
                background: 'var(--bg-surface)',
                borderRadius: 'var(--radius-lg)',
              }}
            >
              <FolderOpen
                size={48}
                style={{ color: 'var(--text-tertiary)', marginBottom: 'var(--spacing-md)' }}
              />
              <p style={{ color: 'var(--text-tertiary)' }}>暂无最近项目</p>
              <button
                className="btn btn-primary mt-md"
                onClick={() => setShowNewProject(true)}
              >
                <Plus size={16} />
                新建项目
              </button>
            </div>
          ) : (
            <div className="recent-list">
              {recentProjects.map((project: Project) => (
                <div
                  key={project.id}
                  className="recent-item"
                  onClick={() => handleProjectClick(project)}
                >
                  <FolderOpen size={20} className="recent-item-icon" />
                  <div className="recent-item-info">
                    <div className="recent-item-name">{project.name}</div>
                    <div className="recent-item-path">{project.path}</div>
                  </div>
                  <span className="badge badge-blue">{project.yoloVersion.toUpperCase()}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {showNewProject && <NewProjectModal onClose={() => setShowNewProject(false)} />}
      {helpType && <HelpModal type={helpType} onClose={() => setHelpType(null)} />}
    </>
  );
}
