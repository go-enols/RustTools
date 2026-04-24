import { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RefreshCw, Download, CheckCircle2, XCircle, Cpu, HardDrive, Monitor } from "lucide-react";

interface EnvReport {
  uv_installed: boolean;
  uv_version?: string;
  python_installed: boolean;
  python_version?: string;
  torch_available: boolean;
  torch_cuda: boolean;
  ort_available: boolean;
  ort_cuda: boolean;
  system: {
    os: string;
    arch: string;
    cpu_cores: number;
    total_memory_mb: number;
  };
  cuda: {
    available: boolean;
    driver_version?: string;
    runtime_version?: string;
    gpus: Array<{ name: string; memory_mb: number }>;
  };
}

interface EnvStatus {
  python_available: boolean;
  python_version?: string;
  torch_available: boolean;
  torch_version?: string;
  cuda_available: boolean;
  installing: boolean;
  detection_error?: string;
}

export default function Settings() {
  const [report, setReport] = useState<EnvReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);
  const pollRef = useRef<ReturnType<typeof setInterval>>();

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const r = await invoke<EnvReport>("generate_env_report");
      setReport(r);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }, []);

  const forceRefresh = useCallback(async () => {
    setLoading(true);
    try {
      // 同时刷新两个缓存
      await invoke("refresh_env_status");
      const r = await invoke<EnvReport>("refresh_env_report");
      setReport(r);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }, []);

  // 懒加载：页面先显示，环境检测在后台进行
  useEffect(() => {
    refresh();
  }, [refresh]);

  // 安装状态轮询
  useEffect(() => {
    if (!installing) {
      if (pollRef.current) clearInterval(pollRef.current);
      return;
    }
    pollRef.current = setInterval(async () => {
      try {
        const status = await invoke<EnvStatus>("get_env_status");
        if (!status.installing) {
          setInstalling(false);
          setLogs((prev) => [...prev, "安装完成，刷新环境信息..."]);
          refresh();
        }
      } catch (e) {
        console.error(e);
      }
    }, 3000);
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, [installing, refresh]);

  const install = async () => {
    setInstalling(true);
    setLogs(["开始安装环境...", "这可能需要 3-10 分钟，请耐心等待"]);
    try {
      await invoke("install_python_env");
    } catch (e: any) {
      setLogs((prev) => [...prev, `启动安装失败: ${e}`]);
      setInstalling(false);
    }
  };

  return (
    <div className="min-h-full p-8">
      <div className="flex items-center justify-between mb-1">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">环境设置</h1>
        <button
          onClick={forceRefresh}
          disabled={loading}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-gray-100 dark:bg-gray-800 text-xs text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 transition disabled:opacity-50"
        >
          <RefreshCw className={`w-3 h-3 ${loading ? "animate-spin" : ""}`} />
          {loading ? "检测中..." : "刷新检测"}
        </button>
      </div>
      <p className="text-sm text-gray-500 dark:text-gray-400 mb-8">环境检测、安装与配置</p>

      {!report ? (
        <div className="flex items-center justify-center py-20">
          <div className="w-6 h-6 border-2 border-brand-primary border-t-transparent rounded-full animate-spin" />
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
          {/* 左侧：检测与安装 */}
          <div className="space-y-5">
            {/* 系统信息 */}
            <SectionCard title="系统信息" icon={<HardDrive className="w-4 h-4" />}>
              <InfoGrid
                items={[
                  ["操作系统", `${report.system.os} (${report.system.arch})`],
                  ["CPU 核心", `${report.system.cpu_cores}`],
                  ["内存", `${(report.system.total_memory_mb / 1024).toFixed(1)} GB`],
                ]}
              />
            </SectionCard>

            {/* GPU / CUDA */}
            <SectionCard title="GPU / CUDA" icon={<Monitor className="w-4 h-4" />}>
              {report.cuda.available ? (
                <div className="space-y-2">
                  <InfoGrid
                    items={[
                      ["CUDA 版本", report.cuda.runtime_version || "未知"],
                      ["驱动版本", report.cuda.driver_version || "未知"],
                    ]}
                  />
                  {report.cuda.gpus.map((gpu, i) => (
                    <div key={i} className="text-xs text-gray-600 dark:text-gray-400 px-3 py-2 rounded-lg bg-gray-50 dark:bg-gray-900/50">
                      GPU {i}: {gpu.name} · {(gpu.memory_mb / 1024).toFixed(1)} GB
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-xs text-gray-500 dark:text-gray-400">未检测到 NVIDIA GPU / CUDA</p>
              )}
            </SectionCard>

            {/* Python 环境 */}
            <SectionCard title="Python 环境" icon={<Cpu className="w-4 h-4" />}>
              <div className="space-y-2">
                <EnvBadge ok={report.uv_installed} label="uv 包管理器" value={report.uv_version || "未安装"} />
                <EnvBadge ok={report.python_installed} label="Python" value={report.python_version || "未安装"} />
                <EnvBadge ok={report.torch_available} label="PyTorch" value={report.torch_available ? (report.torch_cuda ? "GPU" : "CPU") : "未安装"} />
                <EnvBadge ok={report.ort_available} label="ONNX Runtime" value={report.ort_available ? (report.ort_cuda ? "GPU" : "CPU") : "未安装"} />
              </div>
            </SectionCard>
          </div>

          {/* 右侧：安装与关于 */}
          <div className="space-y-5">
            <SectionCard title="一键安装" icon={<Download className="w-4 h-4" />}>
              <p className="text-xs text-gray-500 dark:text-gray-400 mb-4 leading-relaxed">
                自动检测系统环境并安装 uv、Python 虚拟环境、PyTorch 与 ONNX Runtime。
                首次安装约需 3-10 分钟。
              </p>
              <button
                onClick={install}
                disabled={installing}
                className="w-full py-2.5 rounded-xl bg-gradient-to-r from-blue-500 to-purple-500 text-white text-sm font-medium hover:shadow-lg hover:shadow-blue-500/20 disabled:opacity-50 disabled:cursor-not-allowed transition-all"
              >
                {installing ? (
                  <span className="flex items-center justify-center gap-2">
                    <RefreshCw className="w-4 h-4 animate-spin" />
                    安装中...
                  </span>
                ) : (
                  "🚀 一键安装环境"
                )}
              </button>

              {logs.length > 0 && (
                <div className="mt-3 bg-gray-950 rounded-xl p-3 max-h-40 overflow-auto">
                  {logs.map((log, i) => (
                    <pre key={i} className="text-[11px] text-gray-500 font-mono">{log}</pre>
                  ))}
                </div>
              )}
            </SectionCard>

            <SectionCard title="关于" icon={<CheckCircle2 className="w-4 h-4" />}>
              <div className="space-y-2 text-sm">
                <p className="text-gray-700 dark:text-gray-300">RustTools</p>
                <p className="text-xs text-gray-400 dark:text-gray-500">AI 目标检测工具箱 · v1.0.0</p>
                <a
                  href="https://github.com/go-enols/RustTools"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-xs text-brand-primary hover:underline inline-block mt-1"
                >
                  GitHub 仓库 →
                </a>
              </div>
            </SectionCard>
          </div>
        </div>
      )}
    </div>
  );
}

function SectionCard({ title, icon, children }: { title: string; icon: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
      <div className="flex items-center gap-2 mb-4">
        <span className="text-gray-400 dark:text-gray-500">{icon}</span>
        <h2 className="text-sm font-semibold text-gray-900 dark:text-white">{title}</h2>
      </div>
      {children}
    </div>
  );
}

function InfoGrid({ items }: { items: [string, string][] }) {
  return (
    <div className="grid grid-cols-2 gap-y-2 gap-x-4 text-xs">
      {items.map(([k, v]) => (
        <div key={k} className="contents">
          <span className="text-gray-400 dark:text-gray-500">{k}</span>
          <span className="text-gray-700 dark:text-gray-300 text-right">{v}</span>
        </div>
      ))}
    </div>
  );
}

function EnvBadge({ ok, label, value }: { ok: boolean; label: string; value: string }) {
  return (
    <div className="flex items-center justify-between px-3 py-2 rounded-lg bg-gray-50 dark:bg-gray-900/50">
      <span className="text-xs text-gray-500 dark:text-gray-400">{label}</span>
      <span className={`inline-flex items-center gap-1 text-xs font-medium px-2 py-0.5 rounded-full ${
        ok
          ? "bg-emerald-50 text-emerald-600 dark:bg-emerald-900/20 dark:text-emerald-400"
          : "bg-red-50 text-red-500 dark:bg-red-900/20 dark:text-red-400"
      }`}>
        {ok ? <CheckCircle2 className="w-3 h-3" /> : <XCircle className="w-3 h-3" />}
        {value}
      </span>
    </div>
  );
}
