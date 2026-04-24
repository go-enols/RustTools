import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Play, Square, Zap, Rabbit, Turtle, Gauge, FolderOpen, BrainCircuit } from "lucide-react";
import { useProject } from "../contexts/ProjectContext";
import ModelSelector from "../components/ModelSelector";

interface TrainingStatus {
  running: boolean;
  paused: boolean;
  epoch: number;
  total_epochs: number;
  progress_percent: number;
  metrics: {
    train_box_loss: number;
    train_cls_loss: number;
    train_dfl_loss: number;
    precision: number;
    recall: number;
    map50: number;
    map50_95: number;
  };
  error?: string;
}

const PRESETS: Record<string, { label: string; icon: React.ReactNode; epochs: number; batch_size: number; lr0: number; desc: string }> = {
  fast: {
    label: "快速",
    icon: <Rabbit className="w-4 h-4" />,
    epochs: 50,
    batch_size: 16,
    lr0: 0.01,
    desc: "速度快，精度适中",
  },
  standard: {
    label: "标准",
    icon: <Gauge className="w-4 h-4" />,
    epochs: 100,
    batch_size: 8,
    lr0: 0.01,
    desc: "平衡速度与精度",
  },
  high_quality: {
    label: "高质量",
    icon: <Turtle className="w-4 h-4" />,
    epochs: 300,
    batch_size: 4,
    lr0: 0.001,
    desc: "精度最高，耗时较长",
  },
};

export default function Training() {
  const [config, setConfig] = useState({
    base_model: "yolo11n.pt",
    epochs: 100,
    batch_size: 8,
    image_size: 640,
    device_id: 0,
    workers: 8,
    optimizer: "SGD",
    lr0: 0.01,
    lrf: 0.01,
    momentum: 0.937,
    weight_decay: 0.0005,
    warmup_epochs: 3.0,
  });

  const [trainingId, setTrainingId] = useState<string | null>(null);
  const [status, setStatus] = useState<TrainingStatus | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const logRef = useRef<HTMLDivElement>(null);
  const { project } = useProject();
  const [baseModel, setBaseModel] = useState("yolo11n.pt");
  const [results, setResults] = useState<TrainingResult[]>([]);
  const [resultsLoading, setResultsLoading] = useState(false);
  const prevRunningRef = useRef(false);

  interface TrainingResult {
    name: string;
    path: string;
    has_best: boolean;
    has_last: boolean;
    epochs_completed: number;
    map50: number;
    map50_95: number;
    created_at: string;
  }

  const refreshResults = useCallback(async () => {
    if (!project) return;
    setResultsLoading(true);
    try {
      const list = await invoke<TrainingResult[]>("list_training_results", { projectPath: project.path });
      setResults(list);
    } catch (e) {
      console.error(e);
    } finally {
      setResultsLoading(false);
    }
  }, [project]);

  const applyPreset = (key: keyof typeof PRESETS) => {
    const p = PRESETS[key];
    setConfig((c) => ({ ...c, epochs: p.epochs, batch_size: p.batch_size, lr0: p.lr0 }));
  };

  const startTraining = async () => {
    setLoading(true);
    try {
      const id = await invoke<string>("start_training", {
        config: {
          ...config,
          base_model: baseModel,
          warmup_bias_lr: 0.1,
          warmup_momentum: 0.8,
          hsv_h: 0.015,
          hsv_s: 0.7,
          hsv_v: 0.4,
          translate: 0.1,
          scale: 0.5,
          shear: 0.0,
          perspective: 0.0,
          flipud: 0.0,
          fliplr: 0.5,
          mosaic: 1.0,
          mixup: 0.0,
          copy_paste: 0.0,
          close_mosaic: 10,
          rect: false,
          cos_lr: false,
          single_cls: false,
          amp: true,
          save_period: -1,
          cache: false,
        },
      });
      setTrainingId(id);
      setLogs((prev) => [...prev, `训练已启动: ${id}`]);
    } catch (e: any) {
      setLogs((prev) => [...prev, `错误: ${e}`]);
    } finally {
      setLoading(false);
    }
  };

  const stopTraining = async () => {
    if (!trainingId) return;
    try {
      await invoke("stop_training", { trainingId });
      setLogs((prev) => [...prev, "训练已停止"]);
    } catch (e: any) {
      setLogs((prev) => [...prev, `错误: ${e}`]);
    }
  };

  useEffect(() => {
    if (!trainingId) return;
    prevRunningRef.current = false;
    const interval = setInterval(async () => {
      try {
        const s = await invoke<TrainingStatus>("get_training_status", { trainingId });
        setStatus(s);
        // 训练完成自动刷新历史结果
        if (prevRunningRef.current && !s.running) {
          refreshResults();
        }
        prevRunningRef.current = s.running;
        const newLogs = await invoke<string[]>("list_training_logs", { trainingId });
        if (newLogs.length > 0) {
          setLogs((prev) => [...prev, ...newLogs]);
        }
      } catch (e) {
        console.error(e);
      }
    }, 2000);
    return () => clearInterval(interval);
  }, [trainingId, refreshResults]);

  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [logs]);

  useEffect(() => {
    refreshResults();
  }, [refreshResults]);

  return (
    <div className="min-h-full p-8">
      <h1 className="text-2xl font-bold text-gray-900 dark:text-white mb-1">模型训练</h1>
      <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">配置训练参数并实时监控进度</p>

      {/* 当前项目 */}
      {/* 当前项目 + 模型选择 */}
      <div className="mb-6 flex items-center gap-3">
        {project ? (
          <div className="flex items-center gap-2 px-3 py-2 rounded-xl bg-blue-50 dark:bg-blue-950/20 border border-blue-100 dark:border-blue-900/30 text-xs text-blue-700 dark:text-blue-400">
            <FolderOpen className="w-3.5 h-3.5" />
            <span className="font-medium">{project.name}</span>
            <span className="text-blue-400 dark:text-blue-600">·</span>
            <span className="truncate max-w-xs">{project.path}</span>
          </div>
        ) : (
          <div className="flex items-center gap-2 px-3 py-2 rounded-xl bg-amber-50 dark:bg-amber-950/20 border border-amber-100 dark:border-amber-900/30 text-xs text-amber-700 dark:text-amber-400">
            <Zap className="w-3.5 h-3.5" />
            未打开项目，请先前往项目管理打开一个项目
          </div>
        )}
        <div className="flex items-center gap-2 px-3 py-2 rounded-xl bg-purple-50 dark:bg-purple-950/20 border border-purple-100 dark:border-purple-900/30 text-xs text-purple-700 dark:text-purple-400">
          <BrainCircuit className="w-3.5 h-3.5" />
          <ModelSelector
            ext="pt"
            loadOnSelect={false}
            compact
            onSelect={(_name, path) => setBaseModel(path)}
            autoLoad={false}
          />
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-5">
        {/* 左侧：配置 */}
        <div className="lg:col-span-4 space-y-5">
          {/* 预设方案 */}
          <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
              <Zap className="w-4 h-4 text-amber-500" />
              训练预设
            </h2>
            <div className="space-y-2">
              {Object.entries(PRESETS).map(([key, preset]) => (
                <button
                  key={key}
                  onClick={() => applyPreset(key)}
                  className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-left transition-all ${
                    config.epochs === preset.epochs && config.batch_size === preset.batch_size
                      ? "bg-brand-primary/10 border border-brand-primary/20"
                      : "bg-gray-50 dark:bg-gray-900/50 border border-transparent hover:border-gray-200 dark:hover:border-gray-700"
                  }`}
                >
                  <span className="text-gray-400 dark:text-gray-500">{preset.icon}</span>
                  <div className="flex-1 min-w-0">
                    <p className="text-xs font-medium text-gray-900 dark:text-white">{preset.label}</p>
                    <p className="text-[10px] text-gray-400 dark:text-gray-500">{preset.desc}</p>
                  </div>
                  <span className="text-[10px] text-gray-400 dark:text-gray-500 shrink-0">{preset.epochs}轮</span>
                </button>
              ))}
            </div>
          </div>

          {/* 参数配置 */}
          <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-4">训练参数</h2>
            <div className="space-y-3">
              <ConfigField label="训练轮数" value={config.epochs} onChange={(v) => setConfig((c) => ({ ...c, epochs: Number(v) }))} />
              <ConfigField label="批次大小" value={config.batch_size} onChange={(v) => setConfig((c) => ({ ...c, batch_size: Number(v) }))} />
              <ConfigField label="图像尺寸" value={config.image_size} onChange={(v) => setConfig((c) => ({ ...c, image_size: Number(v) }))} />
              <ConfigField label="学习率" value={config.lr0} onChange={(v) => setConfig((c) => ({ ...c, lr0: Number(v) }))} step={0.001} />
              <ConfigField label="GPU 设备" value={config.device_id} onChange={(v) => setConfig((c) => ({ ...c, device_id: Number(v) }))} />
              <ConfigField label="数据加载线程" value={config.workers} onChange={(v) => setConfig((c) => ({ ...c, workers: Number(v) }))} />
              <div className="flex items-center justify-between">
                <label className="text-xs text-gray-500 dark:text-gray-400">优化器</label>
                <select
                  value={config.optimizer}
                  onChange={(e) => setConfig((c) => ({ ...c, optimizer: e.target.value }))}
                  className="w-32 text-xs px-2.5 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900 text-gray-900 dark:text-white focus:outline-none focus:border-blue-400 dark:focus:border-blue-500 focus:ring-2 focus:ring-blue-500/10 transition-all dark:[color-scheme:dark]"
                >
                  <option value="SGD">SGD</option>
                  <option value="Adam">Adam</option>
                  <option value="AdamW">AdamW</option>
                  <option value="Adamax">Adamax</option>
                  <option value="NAdam">NAdam</option>
                  <option value="RAdam">RAdam</option>
                  <option value="RMSProp">RMSProp</option>
                  <option value="Lion">Lion</option>
                </select>
              </div>
            </div>
          </div>

          {/* 控制按钮 */}
          <div className="flex gap-3">
            <button
              onClick={startTraining}
              disabled={loading || (status?.running ?? false)}
              className="flex-1 flex items-center justify-center gap-2 py-2.5 rounded-xl bg-gradient-to-r from-emerald-500 to-teal-500 text-white text-sm font-medium hover:shadow-lg hover:shadow-emerald-500/20 disabled:opacity-50 disabled:cursor-not-allowed transition-all"
            >
              <Play className="w-4 h-4" />
              开始训练
            </button>
            <button
              onClick={stopTraining}
              disabled={!status?.running}
              className="flex-1 flex items-center justify-center gap-2 py-2.5 rounded-xl bg-gradient-to-r from-red-500 to-rose-500 text-white text-sm font-medium hover:shadow-lg hover:shadow-red-500/20 disabled:opacity-50 disabled:cursor-not-allowed transition-all"
            >
              <Square className="w-4 h-4" />
              停止
            </button>
          </div>
        </div>

        {/* 右侧：监控 */}
        <div className="lg:col-span-8 space-y-5">
          {status && (
            <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-sm font-semibold text-gray-900 dark:text-white">训练进度</h2>
                <span className="text-xs text-gray-500 dark:text-gray-400">
                  第 {status.epoch} / {status.total_epochs} 轮
                </span>
              </div>
              <div className="w-full bg-gray-100 dark:bg-gray-900 rounded-full h-2.5 mb-5">
                <div
                  className="bg-gradient-to-r from-blue-500 to-purple-500 h-2.5 rounded-full transition-all duration-500"
                  style={{ width: `${status.progress_percent}%` }}
                />
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
                <MetricCard label="精确率" value={status.metrics.precision.toFixed(3)} color="text-emerald-600 dark:text-emerald-400" />
                <MetricCard label="召回率" value={status.metrics.recall.toFixed(3)} color="text-blue-600 dark:text-blue-400" />
                <MetricCard label="mAP50" value={status.metrics.map50.toFixed(3)} color="text-purple-600 dark:text-purple-400" />
                <MetricCard label="mAP50-95" value={status.metrics.map50_95.toFixed(3)} color="text-amber-600 dark:text-amber-400" />
              </div>
            </div>
          )}

          {/* 训练结果 */}
          {results.length > 0 && (
            <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
              <div className="flex items-center justify-between mb-3">
                <h2 className="text-sm font-semibold text-gray-900 dark:text-white">历史结果</h2>
                <button
                  onClick={refreshResults}
                  disabled={resultsLoading}
                  className="text-[10px] text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 transition"
                >
                  {resultsLoading ? "刷新中..." : "刷新"}
                </button>
              </div>
              <div className="space-y-2 max-h-48 overflow-auto">
                {results.map((r) => (
                  <div key={r.name} className="flex items-center justify-between px-3 py-2 rounded-xl bg-gray-50 dark:bg-gray-900/50 text-xs">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="font-medium text-gray-700 dark:text-gray-300 truncate">{r.name}</span>
                        {r.has_best && <span className="px-1 py-0.5 rounded bg-emerald-100 dark:bg-emerald-900/30 text-emerald-600 dark:text-emerald-400 text-[9px]">best.pt</span>}
                      </div>
                      <div className="text-[10px] text-gray-400 mt-0.5">
                        {r.epochs_completed} 轮 · mAP50: {(r.map50 * 100).toFixed(1)}% · {r.created_at}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* 日志 */}
          <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-3">训练日志</h2>
            <div
              ref={logRef}
              className="bg-gray-100 dark:bg-gray-950 rounded-xl p-4 h-72 overflow-auto font-mono text-[11px] leading-relaxed"
            >
              {logs.length === 0 && (
                <span className="text-gray-400 dark:text-gray-600">等待训练开始...</span>
              )}
              {logs.map((log, i) => (
                <div key={i} className={log.includes("错误") || log.includes("Error") ? "text-red-600 dark:text-red-400" : "text-gray-600 dark:text-gray-400"}>
                  {log}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function ConfigField({ label, value, onChange, step = 1 }: { label: string; value: number; onChange: (v: string) => void; step?: number }) {
  return (
    <div className="flex items-center justify-between">
      <label className="text-xs text-gray-500 dark:text-gray-400">{label}</label>
      <input
        type="number"
        value={value}
        step={step}
        onChange={(e) => onChange(e.target.value)}
        className="w-24 text-xs px-2.5 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50 text-gray-900 dark:text-white text-right focus:outline-none focus:border-blue-400 dark:focus:border-blue-500 focus:ring-2 focus:ring-blue-500/10 transition-all"
      />
    </div>
  );
}

function MetricCard({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <div className="bg-gray-50 dark:bg-gray-900/50 rounded-xl p-3 text-center">
      <div className={`text-lg font-bold ${color || "text-gray-900 dark:text-white"}`}>{value}</div>
      <div className="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5">{label}</div>
    </div>
  );
}
