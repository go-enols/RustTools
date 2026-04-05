import { useState, useEffect } from 'react';
import AppShell from './core/components/layout/AppShell';
import TitleBar from './core/components/layout/TitleBar';
import HubPage from './shared/pages/HubPage';
import ToastContainer from './shared/components/ui/ToastContainer';
import NewProjectModal from './modules/yolo/components/NewProjectModal';
import HelpModal, { HelpType } from './modules/yolo/components/HelpModal';
import AnnotationPage from './modules/yolo/pages/AnnotationPage';
import TrainingPage from './modules/yolo/pages/TrainingPage';
import ResultsPage from './modules/yolo/pages/ResultsPage';
import VideoPage from './modules/yolo/pages/VideoPage';
import DevicePage from './modules/yolo/pages/DevicePage';
import ToolsPage from './modules/yolo/pages/ToolsPage';
import SettingsPage from './modules/yolo/pages/SettingsPage';
import YoloActivityBar from './modules/yolo/components/layout/ActivityBar';
import YoloSidebar from './modules/yolo/components/layout/Sidebar';
import { useSettingsStore } from './core/stores/settingsStore';
import { useWorkspaceStore } from './core/stores/workspaceStore';
import { useRouterStore, PageType } from './core/stores/routerStore';
import { registerYoloModule } from './modules/yolo/manifest';

// Re-export PageType for components that import from App
export type { PageType } from './core/stores/routerStore';

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

  // No project open - show Hub page (without AppShell)
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
        <ToastContainer />
      </>
    );
  }

  // With project open - compose YOLO layout
  const yoloSidebar = (
    <>
      <YoloActivityBar
        currentPage={activePage}
        onNavigate={handleNavigate}
        activeSidebar={activeSidebar}
        onSidebarChange={handleSidebarChange}
      />
      <YoloSidebar
        currentPage={activePage}
        activeSidebar={activeSidebar}
        onNewProject={() => setShowNewProject(true)}
      />
    </>
  );

  return (
    <>
      <AppShell sidebar={yoloSidebar}>
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
      <ToastContainer />
    </>
  );
}
