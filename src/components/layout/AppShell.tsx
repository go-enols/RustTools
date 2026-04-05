import { ReactNode } from 'react';
import ActivityBar from './ActivityBar';
import Sidebar from './Sidebar';
import StatusBar from './StatusBar';
import { PageType } from '../../App';
import { useWorkspaceStore } from '../../stores/workspaceStore';

interface AppShellProps {
  currentPage: PageType;
  onNavigate: (page: PageType) => void;
  onNewProject: () => void;
  onShowHelp: (type: 'shortcuts' | 'docs' | 'update' | 'about') => void;
  activeSidebar: 'explorer' | 'search' | 'none';
  onSidebarChange: (sidebar: 'explorer' | 'search' | 'none') => void;
  children?: ReactNode;
}

export default function AppShell({
  currentPage,
  onNavigate,
  onNewProject,
  onShowHelp: _onShowHelp,
  activeSidebar,
  onSidebarChange,
  children
}: AppShellProps) {
  const currentProject = useWorkspaceStore((state) => state.currentProject);

  return (
    <div className="app-shell">
      {/* Left: Activity Bar + Sidebar */}
      <div className="app-left">
        <ActivityBar
          currentPage={currentPage}
          onNavigate={onNavigate}
          activeSidebar={activeSidebar}
          onSidebarChange={onSidebarChange}
        />
        <Sidebar
          currentPage={currentPage}
          activeSidebar={activeSidebar}
          onNewProject={onNewProject}
        />
      </div>

      {/* Center: Main Content */}
      <div className="app-main">
        <main className="main-content">
          {children}
        </main>
        {/* Status Bar at bottom of main area */}
        <StatusBar currentProject={currentProject} currentPage={currentPage} />
      </div>
    </div>
  );
}
