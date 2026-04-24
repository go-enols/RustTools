import { useEffect, useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  FolderOpen,
  Pencil,
  Dumbbell,
  Film,
  ArrowRight,
  CheckCircle2,
  XCircle,
  AlertCircle,
  Plus,
  Zap,
  FolderInput,
} from "lucide-react";
import { useNavigate } from "react-router-dom";
import { useProject } from "../contexts/ProjectContext";

interface EnvStatus {
  python_available: boolean;
  python_version?: string;
  torch_available: boolean;
  torch_version?: string;
  cuda_available: boolean;
}

interface RecentProject {
  path: string;
  name: string;
  images_count: number;
  labels_count: number;
}

const YOLO_MODULES = [
  { path: "/project", label: "项目管理", desc: "创建与扫描 YOLO 数据集项目", icon: FolderOpen, color: "from-violet-500 to-purple-500" },
  { path: "/annotation", label: "图像标注", desc: "标注工具与数据集制作", icon: Pencil, color: "from-pink-500 to-rose-500" },
  { path: "/training", label: "模型训练", desc: "超参配置与训练监控", icon: Dumbbell, color: "from-emerald-500 to-teal-500" },
  { path: "/video", label: "视频推理", desc: "视频文件目标检测分析", icon: Film, color: "from-amber-500 to-orange-500" },
];

export default function Hub() {
  const [env, setEnv] = useState<EnvStatus | null>(null);
  const [envLoading, setEnvLoading] = useState(false);
  const navigate = useNavigate();
  const { project, scan, openProject: ctxOpenProject } = useProject();

  const recentProject = useMemo<RecentProject | null>(() => {
    const raw = localStorage.getItem("recent_projects");
    if (!raw) return null;
    try {
      const list = JSON.parse(raw) as RecentProject[];
      const item = list[0];
      if (
        item &&
        typeof item.name === "string" &&
        typeof item.path === "string" &&
        typeof item.images_count === "number" &&
        typeof item.labels_count === "number"
      ) {
        return item;
      }
      return null;
    } catch {
      return null;
    }
  }, []);

  useEffect(() => {
    // 懒加载环境状态：页面先渲染，环境检测在后台进行
    setEnvLoading(true);
    invoke<EnvStatus>("get_env_status")
      .then((status) => {
        setEnv(status);
        setEnvLoading(false);
      })
      .catch(() => setEnvLoading(false));
  }, []);

  const openProject = async () => {
    try {
      const path = await invoke<string | null>("pick_folder");
      if (!path) return;
      const info = await invoke<{ path: string; name: string; images_count: number; labels_count: number }>("scan_project", { path });
      await ctxOpenProject(path);
      // 保存到 recent
      const raw = localStorage.getItem("recent_projects");
      const list = raw ? JSON.parse(raw) : [];
      const filtered = list.filter((p: RecentProject) => p.path !== info.path);
      localStorage.setItem("recent_projects", JSON.stringify([info, ...filtered].slice(0, 5)));
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="min-h-full p-8">
      {/* 顶部 */}
      <div className="flex items-start justify-between mb-8">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white tracking-tight">YOLO 工作台</h1>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">一站式高性能 Rust 工具箱</p>
        </div>
        {envLoading ? (
          <div className="px-3 py-1.5 rounded-lg bg-gray-50 dark:bg-gray-800 text-gray-400 dark:text-gray-500 text-xs flex items-center gap-2">
            <div className="w-3 h-3 border border-gray-300 dark:border-gray-600 border-t-transparent rounded-full animate-spin" />
            环境检测中...
          </div>
        ) : env && !env.python_available ? (
          <div className="flex items-center gap-2 px-4 py-2 rounded-xl bg-amber-50 dark:bg-amber-950/30 border border-amber-200 dark:border-amber-900/50 text-amber-700 dark:text-amber-400 text-xs">
            <AlertCircle className="w-4 h-4" />
            Python 环境未就绪，部分功能受限
          </div>
        ) : null}
      </div>

      {/* 项目概览 + 快捷操作 */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5 mb-8">
        {/* 项目概览 */}
        <div className="col-span-2 bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white flex items-center gap-2">
              <Zap className="w-4 h-4 text-amber-500" />
              项目概览
            </h2>
            <button
              onClick={() => navigate("/project")}
              className="text-xs text-brand-primary hover:text-blue-600 flex items-center gap-1 transition"
            >
              查看全部 <ArrowRight className="w-3 h-3" />
            </button>
          </div>

          {project ? (
            <div className="flex items-center gap-4">
              <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-violet-500 to-purple-600 flex items-center justify-center text-white text-lg font-bold shrink-0">
                {project.name.charAt(0).toUpperCase()}
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-semibold text-gray-900 dark:text-white truncate">{project.name}</p>
                <p className="text-xs text-gray-400 dark:text-gray-500 truncate">{project.path}</p>
                <p className="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5">{project.classes.length} 个类别 · {project.yolo_version}</p>
              </div>
              {scan && (
                <div className="flex gap-4 shrink-0">
                  <div className="text-center">
                    <p className="text-base font-bold text-gray-900 dark:text-white">{scan.images}</p>
                    <p className="text-[10px] text-gray-400 dark:text-gray-500">图片</p>
                  </div>
                  <div className="text-center">
                    <p className="text-base font-bold text-gray-900 dark:text-white">{scan.labels}</p>
                    <p className="text-[10px] text-gray-400 dark:text-gray-500">标签</p>
                  </div>
                </div>
              )}
            </div>
          ) : recentProject ? (
            <div className="flex items-center gap-4">
              <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-gray-400 to-gray-500 flex items-center justify-center text-white text-lg font-bold shrink-0">
                {recentProject.name.charAt(0).toUpperCase()}
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-semibold text-gray-900 dark:text-white truncate">{recentProject.name}</p>
                <p className="text-xs text-gray-400 dark:text-gray-500 truncate">{recentProject.path}</p>
                <p className="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5">最近项目（未打开）</p>
              </div>
              <div className="flex gap-4 shrink-0">
                <div className="text-center">
                  <p className="text-base font-bold text-gray-900 dark:text-white">{recentProject.images_count}</p>
                  <p className="text-[10px] text-gray-400 dark:text-gray-500">图片</p>
                </div>
                <div className="text-center">
                  <p className="text-base font-bold text-gray-900 dark:text-white">{recentProject.labels_count}</p>
                  <p className="text-[10px] text-gray-400 dark:text-gray-500">标签</p>
                </div>
              </div>
            </div>
          ) : (
            <div className="flex items-center justify-between">
              <p className="text-xs text-gray-400 dark:text-gray-500">暂无项目</p>
              <div className="flex gap-2">
                <button
                  onClick={openProject}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-gray-100 dark:bg-gray-800 text-xs text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 transition"
                >
                  <FolderInput className="w-3 h-3" />
                  打开项目
                </button>
                <button
                  onClick={() => navigate("/project")}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-brand-primary text-white text-xs font-medium hover:bg-blue-600 transition"
                >
                  <Plus className="w-3 h-3" />
                  新建项目
                </button>
              </div>
            </div>
          )}
        </div>

        {/* 快捷操作 */}
        <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
          <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-4">快捷操作</h2>
          <div className="space-y-2">
            <QuickAction icon={<Pencil className="w-3.5 h-3.5" />} label="打开标注" color="text-pink-500 bg-pink-50 dark:bg-pink-950/20" onClick={() => navigate("/annotation")} />
            <QuickAction icon={<Dumbbell className="w-3.5 h-3.5" />} label="开始训练" color="text-emerald-500 bg-emerald-50 dark:bg-emerald-950/20" onClick={() => navigate("/training")} />
            <QuickAction icon={<Film className="w-3.5 h-3.5" />} label="视频推理" color="text-amber-500 bg-amber-50 dark:bg-amber-950/20" onClick={() => navigate("/video")} />
          </div>
        </div>
      </div>

      {/* 环境状态 */}
      {env && (
        <div className="mb-8">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-sm font-semibold text-gray-700 dark:text-gray-300">运行环境</h2>
            <button
              onClick={() => navigate("/settings")}
              className="text-xs text-brand-primary hover:text-blue-600 flex items-center gap-1 transition"
            >
              去设置 <ArrowRight className="w-3 h-3" />
            </button>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            <StatusBadge ok={env.python_available} label="Python" value={env.python_version || "未安装"} />
            <StatusBadge ok={env.torch_available} label="PyTorch" value={env.torch_version || "未安装"} />
            <StatusBadge ok={env.cuda_available} label="CUDA" value={env.cuda_available ? "可用" : "未检测"} />
          </div>
        </div>
      )}

      {/* YOLO 功能模块 */}
      <div>
        <h2 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-4">YOLO 视觉</h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          {YOLO_MODULES.map((mod) => (
            <button
              key={mod.path}
              onClick={() => navigate(mod.path)}
              className="group relative overflow-hidden rounded-2xl bg-white dark:bg-surface-dark border border-gray-100 dark:border-gray-800 p-5 text-left hover:shadow-lg hover:-translate-y-0.5 transition-all duration-300"
            >
              <div className={`w-10 h-10 rounded-xl bg-gradient-to-br ${mod.color} flex items-center justify-center text-white mb-3 shadow-md group-hover:scale-110 transition-transform duration-300`}>
                <mod.icon className="w-5 h-5" />
              </div>
              <h3 className="text-sm font-semibold text-gray-900 dark:text-white mb-1">{mod.label}</h3>
              <p className="text-xs text-gray-500 dark:text-gray-400">{mod.desc}</p>
              <div className={`absolute -bottom-8 -right-8 w-32 h-32 rounded-full bg-gradient-to-br ${mod.color} opacity-0 group-hover:opacity-[0.07] transition-opacity duration-500 blur-2xl`} />
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

function QuickAction({ icon, label, color, onClick }: { icon: React.ReactNode; label: string; color: string; onClick: () => void }) {
  return (
    <button onClick={onClick} className="w-full flex items-center gap-3 px-3 py-2 rounded-xl hover:bg-gray-50 dark:hover:bg-gray-800 transition text-left">
      <span className={`w-7 h-7 rounded-lg flex items-center justify-center ${color}`}>{icon}</span>
      <span className="text-xs font-medium text-gray-700 dark:text-gray-200">{label}</span>
    </button>
  );
}

function StatusBadge({ ok, label, value }: { ok: boolean; label: string; value: string }) {
  return (
    <div className="flex items-center gap-2.5 px-3 py-2.5 rounded-xl bg-gray-50 dark:bg-gray-900/50 border border-gray-100 dark:border-gray-800">
      {ok ? <CheckCircle2 className="w-4 h-4 text-emerald-500 shrink-0" /> : <XCircle className="w-4 h-4 text-red-400 shrink-0" />}
      <div>
        <div className="text-[10px] text-gray-400 dark:text-gray-500 leading-none">{label}</div>
        <div className={`text-xs font-medium ${ok ? "text-emerald-600 dark:text-emerald-400" : "text-red-500 dark:text-red-400"}`}>{value}</div>
      </div>
    </div>
  );
}
