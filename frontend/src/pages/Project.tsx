import { useState, useCallback, memo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen, FolderPlus, RefreshCw, ArrowRight, Trash2 } from "lucide-react";
import { useProject } from "../contexts/ProjectContext";

interface ProjectInfo {
  path: string;
  name: string;
  images_count: number;
  labels_count: number;
}

// 创建项目
async function createProject(parent: string, name: string): Promise<string> {
  return await invoke("create_project", { parentDir: parent, name });
}

// 选择文件夹
async function pickFolder(): Promise<string | null> {
  const dir = await open({ directory: true });
  return dir ?? null;
}

// 扫描项目
async function scanProject(path: string): Promise<ProjectInfo> {
  return await invoke("scan_project", { path });
}

// 打开项目
async function openProject(path: string): Promise<void> {
  await invoke("open_project", { path });
}

const MemoizedProjectCard = memo(function ProjectCard({
  project,
  onOpen,
  onDelete,
}: {
  project: ProjectInfo;
  onOpen: (p: ProjectInfo) => void;
  onDelete: (p: ProjectInfo) => void;
}) {
  return (
    <div className="group relative bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm hover:shadow-md transition-all">
      <button
        onClick={() => onDelete(project)}
        className="absolute top-3 right-3 p-1.5 rounded-lg text-gray-300 hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 opacity-0 group-hover:opacity-100 transition-all"
        title="删除"
      >
        <Trash2 className="w-3.5 h-3.5" />
      </button>

      <div className="flex items-center gap-3 mb-3">
        <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-emerald-400 to-cyan-500 flex items-center justify-center text-white text-lg font-bold shrink-0">
          {project.name.charAt(0).toUpperCase()}
        </div>
        <div className="min-w-0">
          <h3 className="text-sm font-semibold text-gray-900 dark:text-white truncate">{project.name}</h3>
          <p className="text-xs text-gray-400 dark:text-gray-500 truncate">{project.path}</p>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 mb-4">
        <div className="bg-gray-50 dark:bg-gray-900/50 rounded-xl px-3 py-2 text-center">
          <p className="text-base font-bold text-gray-900 dark:text-white">{project.images_count}</p>
          <p className="text-[10px] text-gray-400 dark:text-gray-500">图片</p>
        </div>
        <div className="bg-gray-50 dark:bg-gray-900/50 rounded-xl px-3 py-2 text-center">
          <p className="text-base font-bold text-gray-900 dark:text-white">{project.labels_count}</p>
          <p className="text-[10px] text-gray-400 dark:text-gray-500">标签</p>
        </div>
      </div>

      <button
        onClick={() => onOpen(project)}
        className="w-full py-2 rounded-xl bg-gradient-to-r from-emerald-500 to-cyan-500 text-white text-xs font-medium hover:shadow-lg hover:shadow-emerald-500/20 transition-all flex items-center justify-center gap-1"
      >
        打开项目 <ArrowRight className="w-3.5 h-3.5" />
      </button>
    </div>
  );
});

export default function Project() {
  const [projects, setProjects] = useState<ProjectInfo[]>(() => {
    const raw = localStorage.getItem("recent_projects");
    if (!raw) return [];
    try {
      const list = JSON.parse(raw) as unknown[];
      return list.filter((item): item is ProjectInfo => {
        const p = item as Record<string, unknown>;
        return (
          p !== null &&
          typeof p === "object" &&
          typeof p.name === "string" &&
          typeof p.path === "string" &&
          typeof p.images_count === "number" &&
          typeof p.labels_count === "number"
        );
      });
    } catch {
      return [];
    }
  });
  const [creating, setCreating] = useState(false);
  const [createName, setCreateName] = useState("");
  const [createDir, setCreateDir] = useState("");
  const [scanning, setScanning] = useState(false);
  const { refresh } = useProject();

  const saveProjects = useCallback((list: ProjectInfo[]) => {
    setProjects(list);
    localStorage.setItem("recent_projects", JSON.stringify(list));
  }, []);

  const handlePickFolder = async () => {
    const dir = await pickFolder();
    if (dir) setCreateDir(dir);
  };

  const handleCreate = async () => {
    if (!createName.trim() || !createDir) return;
    setCreating(true);
    try {
      const path = await createProject(createDir, createName.trim());
      const info = await scanProject(path);
      await openProject(path);
      saveProjects([info, ...projects.filter((p) => p.path !== info.path)]);
      await refresh();
      setCreateName("");
      setCreateDir("");
    } catch (e) {
      console.error(e);
    } finally {
      setCreating(false);
    }
  };

  const handleOpen = async (project: ProjectInfo) => {
    try {
      await openProject(project.path);
      saveProjects([project, ...projects.filter((p) => p.path !== project.path)]);
      await refresh();
    } catch (e) {
      console.error(e);
    }
  };

  const handleDelete = (project: ProjectInfo) => {
    const list = projects.filter((p) => p.path !== project.path);
    saveProjects(list);
  };

  const handleScan = async () => {
    const dir = await pickFolder();
    if (!dir) return;
    setScanning(true);
    try {
      const info = await scanProject(dir);
      await openProject(dir);
      saveProjects([info, ...projects.filter((p) => p.path !== info.path)]);
      await refresh();
    } catch (e) {
      console.error(e);
    } finally {
      setScanning(false);
    }
  };

  return (
    <div className="min-h-full p-8">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white mb-1">项目管理</h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">创建、扫描与管理你的 YOLO 项目</p>
        </div>
        <button
          onClick={handleScan}
          disabled={scanning}
          className="px-4 py-2 rounded-xl bg-white dark:bg-surface-dark border border-gray-200 dark:border-gray-700 text-sm text-gray-700 dark:text-gray-200 hover:border-gray-300 dark:hover:border-gray-600 transition-colors flex items-center gap-2"
        >
          <RefreshCw className={`w-4 h-4 ${scanning ? "animate-spin" : ""}`} />
          扫描现有项目
        </button>
      </div>

      {/* 创建项目 */}
      <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm mb-8">
        <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
          <FolderPlus className="w-4 h-4 text-emerald-500" />
          新建项目
        </h2>
        <div className="flex gap-3 items-end">
          <div className="flex-1 min-w-0">
            <label className="text-xs text-gray-500 dark:text-gray-400 mb-1 block">项目名称</label>
            <input
              value={createName}
              onChange={(e) => setCreateName(e.target.value)}
              placeholder="输入项目名称"
              className="w-full px-3 py-2.5 rounded-xl border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50 text-sm text-gray-900 dark:text-white placeholder-gray-400 focus:outline-none focus:border-emerald-400 dark:focus:border-emerald-500 focus:ring-2 focus:ring-emerald-500/10 transition-all"
            />
          </div>
          <div className="flex-[2] min-w-0">
            <label className="text-xs text-gray-500 dark:text-gray-400 mb-1 block">保存位置</label>
            <div className="flex gap-2">
              <div
                onClick={handlePickFolder}
                className="flex-1 px-3 py-2.5 rounded-xl border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50 text-sm text-gray-600 dark:text-gray-300 cursor-pointer hover:border-gray-300 dark:hover:border-gray-600 transition-colors truncate"
              >
                {createDir || "点击选择文件夹"}
              </div>
            </div>
          </div>
          <button
            onClick={handleCreate}
            disabled={creating || !createName.trim() || !createDir}
            className="px-5 py-2.5 rounded-xl bg-gradient-to-r from-emerald-500 to-cyan-500 text-white text-sm font-medium hover:shadow-lg hover:shadow-emerald-500/20 disabled:opacity-50 disabled:cursor-not-allowed transition-all flex items-center gap-2 shrink-0"
          >
            <FolderPlus className="w-4 h-4" />
            创建
          </button>
        </div>
      </div>

      {/* 最近项目 */}
      {projects.length > 0 && (
        <div>
          <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
            <FolderOpen className="w-4 h-4 text-blue-500" />
            最近项目
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
            {projects.map((project) =>
              project && typeof project.name === "string" ? (
                <MemoizedProjectCard
                  key={project.path}
                  project={project}
                  onOpen={handleOpen}
                  onDelete={handleDelete}
                />
              ) : null
            )}
          </div>
        </div>
      )}
    </div>
  );
}
