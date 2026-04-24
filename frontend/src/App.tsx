import { useCallback, useEffect, useState } from "react";
import { HashRouter, Routes, Route, useLocation, useNavigate } from "react-router-dom";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { ErrorToast, type AppError } from "./components/ErrorToast";
import TitleBar from "./components/TitleBar";
import { useTheme } from "./hooks/useTheme";
import { AppErrorContext } from "./contexts/AppErrorContext";
import { ProjectProvider } from "./contexts/ProjectContext";
import {
  LayoutGrid,
  FolderOpen,
  Pencil,
  Dumbbell,
  Film,
  Monitor,
  Image,
  Cpu,
  Settings,
  Sun,
  Moon,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-shell";

// 页面
import Welcome from "./pages/Welcome";
import Hub from "./pages/Hub";
import Project from "./pages/Project";
import Annotation from "./pages/Annotation";
import Training from "./pages/Training";
import Video from "./pages/Video";
import Desktop from "./pages/Desktop";
import ImageInference from "./pages/ImageInference";
import SettingsPage from "./pages/Settings";
import Device from "./pages/Device";
import { AgentPage } from "./agent";

// YOLO / 工具类导航
const TOOL_NAV_ITEMS = [
  { path: "/hub", label: "总览", icon: LayoutGrid },
  { path: "/project", label: "项目", icon: FolderOpen },
  { path: "/annotation", label: "标注", icon: Pencil },
  { path: "/training", label: "训练", icon: Dumbbell },
  { path: "/video", label: "视频", icon: Film },
  { path: "/desktop", label: "捕获", icon: Monitor },
  { path: "/image", label: "图片", icon: Image },
  { path: "/device", label: "设备", icon: Cpu },
  { path: "/settings", label: "设置", icon: Settings },
];



function App() {
  const { dark, toggle } = useTheme();
  const [errors, setErrors] = useState<AppError[]>([]);

  const addError = useCallback((err: Omit<AppError, "id">) => {
    const id = `${Date.now()}-${Math.random()}`;
    setErrors((prev) => [...prev, { ...err, id }]);
  }, []);

  const dismissError = useCallback((id: string) => {
    setErrors((prev) => prev.filter((e) => e.id !== id));
  }, []);

  const handleBoundaryError = useCallback(
    (error: Error) => {
      addError({ message: "渲染错误", detail: error.message, type: "error" });
    },
    [addError]
  );

  // 拦截所有外部链接点击，用系统浏览器打开
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      const target = e.composedPath()[0] as HTMLElement;
      const anchor = target.closest("a");
      if (!anchor) return;
      const href = anchor.getAttribute("href");
      if (!href) return;
      // 外部链接：http/https 或 target="_blank"
      if (href.startsWith("http://") || href.startsWith("https://") || anchor.getAttribute("target") === "_blank") {
        e.preventDefault();
        open(href).catch(() => {
          // fallback: 如果 shell open 失败，尝试普通打开
          window.open(href, "_blank");
        });
      }
    };
    document.addEventListener("click", handler);
    return () => document.removeEventListener("click", handler);
  }, []);

  return (
    <ErrorBoundary onError={handleBoundaryError}>
      <AppErrorContext.Provider value={{ addError, dismissError }}>
      <ProjectProvider>
      <HashRouter>
        <div className="h-screen flex flex-col bg-bg dark:bg-bg-dark text-gray-900 dark:text-gray-100 overflow-hidden">
          <TitleBar />
          <div className="flex-1 flex overflow-hidden">
            <Sidebar toggleTheme={toggle} dark={dark} />
            <main className="flex-1 overflow-auto">
              <Routes>
                <Route path="/" element={<Welcome />} />
                <Route path="/hub" element={<Hub />} />
                <Route path="/project" element={<Project />} />
                <Route path="/annotation" element={<Annotation />} />
                <Route path="/training" element={<Training />} />
                <Route path="/video" element={<Video />} />
                <Route path="/desktop" element={<Desktop />} />
                <Route path="/image" element={<ImageInference />} />
                <Route path="/settings" element={<SettingsPage />} />
                <Route path="/device" element={<Device />} />
                <Route path="/agent" element={<AgentPage />} />
              </Routes>
            </main>
          </div>
          <ErrorToast errors={errors} onDismiss={dismissError} />
        </div>
      </HashRouter>
      </ProjectProvider>
      </AppErrorContext.Provider>
    </ErrorBoundary>
  );
}

function Sidebar({ toggleTheme, dark }: { toggleTheme: () => void; dark: boolean }) {
  const location = useLocation();
  const navigate = useNavigate();
  // Welcome 和 Agent 页面不显示 YOLO 工具侧边栏
  const isWelcome = location.pathname === "/" || location.pathname === "/agent";

  const renderNavItem = (item: (typeof TOOL_NAV_ITEMS)[0]) => {
    const Icon = item.icon;
    const active = location.pathname === item.path;
    return (
      <button
        key={item.path}
        onClick={() => navigate(item.path)}
        title={item.label}
        className={`w-[52px] h-[52px] rounded-2xl flex flex-col items-center justify-center gap-[2px] transition-all duration-200 ${
          active
            ? "bg-brand-primary/10 text-brand-primary"
            : "text-gray-400 dark:text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
        }`}
      >
        <Icon className="w-[18px] h-[18px]" strokeWidth={active ? 2.5 : 1.8} />
        <span className="text-[9px] leading-none font-medium">{item.label}</span>
      </button>
    );
  };

  return (
    <aside className="w-[60px] shrink-0 bg-surface dark:bg-surface-dark border-r border-gray-200/50 dark:border-gray-800/50 flex flex-col items-center py-3 gap-0.5 select-none">
      {/* Welcome 页面时隐藏导航项，只保留主题切换 */}
      {!isWelcome && (
        <>
          {/* 工具类导航（YOLO / Project / Training 等） */}
          {TOOL_NAV_ITEMS.map(renderNavItem)}

          <div className="flex-1" />
        </>
      )}

      {isWelcome && <div className="flex-1" />}

      <button
        onClick={toggleTheme}
        title={dark ? "切换浅色" : "切换深色"}
        className="w-[52px] h-[52px] rounded-2xl flex flex-col items-center justify-center gap-[2px] text-gray-400 dark:text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-all duration-200"
      >
        {dark ? <Sun className="w-[18px] h-[18px]" strokeWidth={1.8} /> : <Moon className="w-[18px] h-[18px]" strokeWidth={1.8} />}
        <span className="text-[9px] leading-none font-medium">{dark ? "浅色" : "深色"}</span>
      </button>
    </aside>
  );
}

export default App;
