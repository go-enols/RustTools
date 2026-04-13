import { Project } from '../../../../core/stores/workspaceStore';
import { PageType } from '../../../../core/stores/routerStore';

interface StatusBarProps {
  currentProject: Project | null;
  currentPage?: PageType;
}

const pageLabels: Record<PageType, string> = {
  hub: '首页',
  yolo: 'YOLO',
  annotation: '数据标注',
  training: '模型训练',
  results: '训练结果',
  video: '视频推理',
  device: '设备管理',
  tools: '工具',
  settings: '设置',
  desktop: '桌面检测',
};

export default function StatusBar({ currentProject, currentPage = 'hub' }: StatusBarProps) {
  return (
    <div className="status-bar">
      <div className="status-bar-left">
        <div className="status-item">
          <span className="status-dot status-dot-success"></span>
          <span>{currentProject ? currentProject.name : '未打开项目'}</span>
        </div>
        {currentProject && (
          <div className="status-item">
            <span className="status-badge">{currentProject.yoloVersion.toUpperCase()}</span>
          </div>
        )}
      </div>

      <div className="status-bar-right">
        <div className="status-item">
          <span>{pageLabels[currentPage]}</span>
        </div>
      </div>
    </div>
  );
}
