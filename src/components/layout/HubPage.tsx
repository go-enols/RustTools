import { useState } from 'react';
import { useRouterStore, moduleRegistry } from '../../modules';
import { useWorkspaceStore } from '../../stores/workspaceStore';
import YoloGuideModal from '../yolo/YoloGuideModal';
import NewProjectModal from '../workspace/NewProjectModal';
import { Lock, Sparkles, ArrowRight } from 'lucide-react';

export default function HubPage() {
  const modules = moduleRegistry.getAllModules();
  const { navigateToModule } = useRouterStore();
  const { currentProject, selectProjectPath, openProjectFromPath } = useWorkspaceStore();
  const [showYoloGuide, setShowYoloGuide] = useState(false);
  const [showNewProject, setShowNewProject] = useState(false);

  const handleModuleClick = (moduleId: string) => {
    if (moduleId === 'yolo') {
      if (currentProject) {
        navigateToModule(moduleId);
      } else {
        setShowYoloGuide(true);
      }
    } else {
      console.warn(`模块 ${moduleId} 暂未开放`);
    }
  };

  const handleOpenProject = async () => {
    const path = await selectProjectPath();
    if (path) {
      await openProjectFromPath(path);
      navigateToModule('yolo');
    }
  };

  const handleNewProject = () => {
    setShowNewProject(true);
  };

  const handleProjectCreated = () => {
    navigateToModule('yolo');
  };

  return (
    <div className="hub-page">
      {/* Header */}
      <div className="hub-header">
        <div className="hub-logo">
          <div className="hub-logo-icon">
            <Sparkles size={32} />
          </div>
          <h1 className="hub-logo-text">YOLO-Flow</h1>
        </div>
        <p className="hub-subtitle">模块化 AI 工作流平台</p>
      </div>

      {/* Module Grid - 居中展示 */}
      <div className="hub-module-grid">
        {modules.map((module) => {
          const Icon = module.iconComponent;
          const isYolo = module.manifest.id === 'yolo';

          return (
            <div
              key={module.manifest.id}
              className={`hub-module-card ${isYolo ? 'active' : ''} ${!isYolo ? 'locked' : ''}`}
              onClick={() => handleModuleClick(module.manifest.id)}
            >
              <div className="hub-module-icon">
                <Icon size={40} />
              </div>
              <h3 className="hub-module-name">{module.manifest.name}</h3>
              <p className="hub-module-desc">{module.manifest.description}</p>
              {isYolo ? (
                <span className="hub-module-badge active">
                  <ArrowRight size={14} />
                  进入
                </span>
              ) : (
                <span className="hub-module-badge locked">
                  <Lock size={12} />
                  即将推出
                </span>
              )}
            </div>
          );
        })}
      </div>

      {/* YOLO Guide Modal - 新建项目时保持显示 */}
      {showYoloGuide && (
        <YoloGuideModal
          onClose={() => setShowYoloGuide(false)}
          onOpenProject={handleOpenProject}
          onNewProject={handleNewProject}
        />
      )}

      {/* New Project Modal - 叠在 YoloGuideModal 上方 */}
      {showNewProject && (
        <NewProjectModal
          onClose={() => setShowNewProject(false)}
          onCreated={handleProjectCreated}
        />
      )}
    </div>
  );
}
