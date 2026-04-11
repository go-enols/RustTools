import {
  FileText,
  PenTool,
  Train,
  BarChart3,
  Video,
  Monitor,
  Cpu,
  Wrench,
  Settings,
  FolderOpen,
  Layout,
} from 'lucide-react';
import { PageType } from '../../../../core/stores/routerStore';

interface ActivityBarProps {
  currentPage: PageType;
  onNavigate: (page: PageType) => void;
  activeSidebar: 'explorer' | 'search' | 'none';
  onSidebarChange: (sidebar: 'explorer' | 'search' | 'none') => void;
}

// Page icons for navigation - shown at top of activity bar
const pageIcons: { id: PageType; icon: typeof FileText; label: string }[] = [
  { id: 'annotation', icon: PenTool, label: '数据标注' },
  { id: 'training', icon: Train, label: '模型训练' },
  { id: 'results', icon: BarChart3, label: '训练结果' },
  { id: 'video', icon: Video, label: '视频推理' },
  { id: 'desktop', icon: Monitor, label: '桌面推理' },
  { id: 'device', icon: Cpu, label: '设备管理' },
  { id: 'tools', icon: Wrench, label: '工具' },
];

export default function ActivityBar({
  currentPage,
  onNavigate,
  activeSidebar,
  onSidebarChange,
}: ActivityBarProps) {
  const isPageActive = (pageId: PageType) => currentPage === pageId || (pageId === 'yolo' && currentPage === 'yolo');

  const handlePageClick = (pageId: PageType) => {
    onNavigate(pageId);
    // Ensure explorer sidebar is shown when navigating to a page
    if (activeSidebar === 'none') {
      onSidebarChange('explorer');
    }
  };

  const handleExplorerClick = () => {
    onSidebarChange(activeSidebar === 'explorer' ? 'none' : 'explorer');
  };

  return (
    <div className="activity-bar">
      {/* Page Navigation Icons */}
      <div className="activity-icons">


        {/* Explorer toggle */}
        <div
          className={`activity-icon ${activeSidebar === 'explorer' ? 'active' : ''}`}
          onClick={handleExplorerClick}
          title="资源管理器"
        >
          <FolderOpen size={24} />
        </div>

        <div className="activity-divider" />
        {/* Home/Explorer toggle */}
        <div
          className={`activity-icon ${isPageActive('yolo') ? 'active' : ''}`}
          onClick={() => handlePageClick('yolo')}
          title="项目首页"
        >
          <Layout size={24} />
        </div>
        {/* Page navigation */}
        {pageIcons.map((item) => {
          const Icon = item.icon;
          const isActive = isPageActive(item.id);
          return (
            <div
              key={item.id}
              className={`activity-icon ${isActive ? 'active' : ''}`}
              onClick={() => handlePageClick(item.id)}
              title={item.label}
            >
              <Icon size={24} />
            </div>
          );
        })}
      </div>

      {/* Bottom icons */}
      <div className="activity-bottom">
        <div
          className={`activity-icon ${isPageActive('settings') ? 'active' : ''}`}
          onClick={() => onNavigate('settings')}
          title="设置"
        >
          <Settings size={24} />
        </div>
      </div>
    </div>
  );
}
