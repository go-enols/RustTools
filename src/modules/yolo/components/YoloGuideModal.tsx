import { Sparkles, FolderOpen, Plus, ArrowRight, X, Clock } from 'lucide-react';
import type { Project } from '../../../core/stores/workspaceStore';

interface YoloGuideModalProps {
  onClose: () => void;
  onOpenProject: () => void;
  onNewProject: () => void;
  recentProjects?: Project[];
  onSelectRecentProject?: (project: Project) => void;
}

export default function YoloGuideModal({
  onClose,
  onOpenProject,
  onNewProject,
  recentProjects = [],
  onSelectRecentProject,
}: YoloGuideModalProps) {
  return (
    <div className="guide-modal-overlay" onClick={onClose}>
      <div className="guide-modal" onClick={(e) => e.stopPropagation()}>
        <button className="guide-modal-close" onClick={onClose}>
          <X size={20} />
        </button>

        <div className="guide-modal-header">
          <div className="guide-modal-icon">
            <Sparkles size={28} />
          </div>
          <h2>YOLO 工具</h2>
          <p>选择项目或创建一个新项目开始</p>
        </div>

        <div className="guide-modal-actions">
          <button className="guide-action-btn primary" onClick={onNewProject}>
            <div className="guide-action-icon">
              <Plus size={22} />
            </div>
            <div className="guide-action-content">
              <span className="guide-action-title">新建项目</span>
              <span className="guide-action-desc">创建一个全新的 YOLO 项目</span>
            </div>
            <ArrowRight size={18} className="guide-action-arrow" />
          </button>

          <button className="guide-action-btn" onClick={onOpenProject}>
            <div className="guide-action-icon">
              <FolderOpen size={22} />
            </div>
            <div className="guide-action-content">
              <span className="guide-action-title">打开项目</span>
              <span className="guide-action-desc">选择已有的项目文件夹</span>
            </div>
            <ArrowRight size={18} className="guide-action-arrow" />
          </button>
        </div>

        {recentProjects.length > 0 && (
          <div className="guide-modal-recent">
            <div className="guide-recent-header">
              <Clock size={14} />
              <span>最近项目</span>
            </div>
            <div className="guide-recent-list">
              {recentProjects.slice(0, 5).map((project) => (
                <button
                  key={project.id}
                  className="guide-recent-item"
                  onClick={() => onSelectRecentProject?.(project)}
                >
                  <div className="guide-recent-name">{project.name}</div>
                  <div className="guide-recent-path">{project.path}</div>
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
