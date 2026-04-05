import { useState, useEffect } from 'react';
import AppShell from './components/layout/AppShell';
import TitleBar from './components/layout/TitleBar';
import HubPage from './components/layout/HubPage';
import NewProjectModal from './components/workspace/NewProjectModal';
import HelpModal, { HelpType } from './components/ui/HelpModal';
import AnnotationPage from './components/annotation/AnnotationPage';
import TrainingPage from './components/training/TrainingPage';
import ResultsPage from './components/results/ResultsPage';
import VideoPage from './components/video/VideoPage';
import DevicePage from './components/device/DevicePage';
import ToolsPage from './components/tools/ToolsPage';
import SettingsPage from './components/settings/SettingsPage';
import { useSettingsStore } from './stores/settingsStore';
import { useWorkspaceStore } from './stores/workspaceStore';
import { useRouterStore, PageType } from './stores/routerStore';
import { registerYoloModule } from './modules/yolo/manifest';

// Re-export PageType for components that import from App
export type { PageType } from './stores/routerStore';

export default function App() {
  const [showNewProject, setShowNewProject] = useState(false);
  const [helpType, setHelpType] = useState<HelpType | null>(null);
  const [isReady, setIsReady] = useState(false);
  const [activeSidebar, setActiveSidebar] = useState<'explorer' | 'search' | 'none'>('explorer');
  const theme = useSettingsStore((state) => state.settings.theme);
  const { currentProject, loadRecentProjects, loadCurrentProject } = useWorkspaceStore();
  const { activePage, navigateToPage } = useRouterStore();

  // Initialize stores on mount
  useEffect(() => {
    const init = async () => {
      // 注册模块
      registerYoloModule();
      // 加载设置
      await useSettingsStore.getState().loadSettings();
      await loadRecentProjects();
      loadCurrentProject();
      setIsReady(true);
    };
    init();
  }, [loadRecentProjects, loadCurrentProject]);

  // Sync theme changes to DOM
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
  }, [theme]);

  const handleNavigate = (page: PageType) => {
    navigateToPage(page);
  };

  const handleSidebarChange = (sidebar: 'explorer' | 'search' | 'none') => {
    setActiveSidebar(sidebar);
  };

  const handleProjectCreated = () => {
    setShowNewProject(false);
    navigateToPage('annotation');
  };

  const renderPage = () => {
    switch (activePage) {
      case 'hub':
        return <HubPage />;
      case 'yolo':
      case 'annotation':
        return <AnnotationPage />;
      case 'training':
        return <TrainingPage />;
      case 'results':
        return <ResultsPage />;
      case 'video':
        return <VideoPage />;
      case 'device':
        return <DevicePage />;
      case 'tools':
        return <ToolsPage />;
      case 'settings':
        return <SettingsPage />;
      default:
        return <AnnotationPage />;
    }
  };

  if (!isReady) {
    return (
      <div
        style={{
          height: '100vh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: 'var(--bg-surface)',
          color: 'var(--text-primary)',
        }}
      >
        <div style={{ textAlign: 'center' }}>
          <div
            style={{
              width: 40,
              height: 40,
              border: '3px solid var(--border-default)',
              borderTopColor: 'var(--accent-primary)',
              borderRadius: '50%',
              animation: 'spin 1s linear infinite',
              margin: '0 auto 16px',
            }}
          />
          <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
          加载中...
        </div>
      </div>
    );
  }

  // No project open - show Hub page
  if (!currentProject) {
    return (
      <>
        <TitleBar />
        <HubPage />
        {showNewProject && (
          <NewProjectModal
            onClose={() => setShowNewProject(false)}
            onCreated={handleProjectCreated}
          />
        )}
      </>
    );
  }

  return (
    <>
      <AppShell
        currentPage={activePage}
        onNavigate={handleNavigate}
        onNewProject={() => setShowNewProject(true)}
        onShowHelp={(type) => setHelpType(type)}
        activeSidebar={activeSidebar}
        onSidebarChange={handleSidebarChange}
      >
        {renderPage()}
      </AppShell>
      {showNewProject && (
        <NewProjectModal
          onClose={() => setShowNewProject(false)}
          onCreated={handleProjectCreated}
        />
      )}
      {helpType && (
        <HelpModal type={helpType} onClose={() => setHelpType(null)} />
      )}
    </>
  );
}
