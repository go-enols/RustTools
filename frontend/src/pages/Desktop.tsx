import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Monitor, Play, Square, Zap, Settings2, BrainCircuit, ArrowRightLeft, Loader2, CheckCircle2, AlertCircle } from "lucide-react";
import ModelSelector from "../components/ModelSelector";
import { usePtExport } from "../hooks/usePtExport";

interface OnnxDetection {
  class_id: number;
  confidence: number;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

interface CaptureState {
  running: boolean;
  fps: number;
  last_frame_base64: string | null;
  detections: OnnxDetection[];
}

const COLORS = [
  "#ef4444", "#22c55e", "#3b82f6", "#f59e0b", "#8b5cf6",
  "#ec4899", "#06b6d4", "#f97316", "#84cc16", "#6366f1",
];

export default function Desktop() {
  const [captureState, setCaptureState] = useState<CaptureState | null>(null);
  const [frame, setFrame] = useState<string | null>(null);
  const [latency, setLatency] = useState(0);
  const [showBoxes, setShowBoxes] = useState(true);
  const [showLabels, setShowLabels] = useState(true);
  const [confThreshold, setConfThreshold] = useState(0.25);
  const [selectedModel, setSelectedModel] = useState("");
  const { needsExport, exporting, exportError, exportSuccess, checkModel, doExport } = usePtExport();
  const imgRef = useRef<HTMLImageElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rafRef = useRef<number>();
  const lastFetchRef = useRef<number>(0);
  const isRunningRef = useRef(false);

  const refreshState = useCallback(async () => {
    const start = performance.now();
    try {
      const state = await invoke<CaptureState>("get_capture_state");
      setCaptureState(state);
      if (state.last_frame_base64) {
        setFrame(`data:image/jpeg;base64,${state.last_frame_base64}`);
      }
      setLatency(performance.now() - start);
    } catch (e) {
      console.error(e);
    }
  }, []);

  const handleSelectModel = async (_name: string, path: string) => {
    setSelectedModel(path);
    await checkModel(path);
  };

  const startCapture = async () => {
    if (!selectedModel) {
      alert("请先选择一个模型");
      return;
    }
    let modelPath = selectedModel;
    if (needsExport) {
      try {
        await doExport(selectedModel);
      } catch {
        return;
      }
    }
    try {
      await invoke("start_capture", { modelPath, confThreshold: confThreshold });
      await refreshState();
    } catch (e) {
      console.error(e);
    }
  };

  const stopCapture = async () => {
    try {
      await invoke("stop_capture");
      setFrame(null);
      await refreshState();
    } catch (e) {
      console.error(e);
    }
  };

  // 使用 requestAnimationFrame + 节流 替代 setInterval
  useEffect(() => {
    refreshState();
  }, [refreshState]);

  useEffect(() => {
    isRunningRef.current = captureState?.running ?? false;
    if (!captureState?.running) {
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
      return;
    }

    const loop = () => {
      const now = performance.now();
      // 30 FPS 轮询
      if (now - lastFetchRef.current >= 33) {
        lastFetchRef.current = now;
        refreshState();
      }
      if (isRunningRef.current) {
        rafRef.current = requestAnimationFrame(loop);
      }
    };

    rafRef.current = requestAnimationFrame(loop);
    return () => {
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
    };
  }, [captureState?.running, refreshState]);

  // 绘制检测框
  useEffect(() => {
    if (!frame || !showBoxes || !captureState?.detections?.length || !imgRef.current || !canvasRef.current) return;
    const img = imgRef.current;
    const canvas = canvasRef.current;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const draw = () => {
      canvas.width = img.naturalWidth || img.clientWidth;
      canvas.height = img.naturalHeight || img.clientHeight;
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      for (const det of captureState.detections) {
        // 后端返回的是绝对像素坐标，直接使用
        const x = det.x1;
        const y = det.y1;
        const w = det.x2 - det.x1;
        const h = det.y2 - det.y1;
        const color = COLORS[det.class_id % COLORS.length];
        ctx.strokeStyle = color;
        ctx.lineWidth = 2;
        ctx.strokeRect(x, y, w, h);
        if (showLabels) {
          ctx.fillStyle = color;
          ctx.font = "12px monospace";
          const label = `ID:${det.class_id} ${(det.confidence * 100).toFixed(0)}%`;
          const tw = ctx.measureText(label).width;
          ctx.fillRect(x, y - 16, tw + 6, 16);
          ctx.fillStyle = "#fff";
          ctx.fillText(label, x + 3, y - 4);
        }
      }
    };

    if (img.complete) {
      draw();
    } else {
      img.onload = draw;
    }
  }, [frame, captureState?.detections, showBoxes, showLabels]);

  return (
    <div className="min-h-full p-8 mx-auto">
      <h1 className="text-2xl font-bold text-gray-900 dark:text-white mb-1">桌面捕获</h1>
      <p className="text-sm text-gray-500 dark:text-gray-400 mb-8">实时屏幕捕获与 YOLO 目标检测</p>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-5">
        {/* 预览区 */}
        <div className="lg:col-span-8">
          <div className="bg-black rounded-2xl overflow-hidden aspect-video relative flex items-center justify-center shadow-lg">
            {frame ? (
              <>
                <img ref={imgRef} src={frame} alt="capture" className="w-full h-full object-contain" crossOrigin="anonymous" />
                <canvas ref={canvasRef} className="absolute inset-0 w-full h-full object-contain pointer-events-none" />
              </>
            ) : (
              <div className="text-center">
                <Monitor className="w-14 h-14 text-gray-300 mx-auto mb-4" />
                <p className="text-sm text-gray-400 font-medium">未开始捕获</p>
                <p className="text-xs text-gray-500 mt-1">选择模型后点击开始按钮启动屏幕捕获</p>
              </div>
            )}

            {/* FPS 覆盖层 */}
            {captureState?.running && (
              <div className="absolute top-4 left-4 flex items-center gap-2">
                <span className="px-2.5 py-1 rounded-lg bg-black/70 backdrop-blur text-white text-[10px] font-mono flex items-center gap-1.5">
                  <Zap className="w-3 h-3 text-amber-400" />
                  {captureState.fps.toFixed(1)} FPS
                </span>
                <span className="px-2.5 py-1 rounded-lg bg-black/70 backdrop-blur text-white text-[10px] font-mono">
                  {latency.toFixed(0)} ms
                </span>
                <span className="px-2.5 py-1 rounded-lg bg-black/70 backdrop-blur text-white text-[10px] font-mono">
                  {captureState.detections.length} 目标
                </span>
              </div>
            )}
          </div>
        </div>

        {/* 控制面板 */}
        <div className="lg:col-span-4 space-y-5">
          <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
              <Play className="w-4 h-4 text-emerald-500" />
              控制
            </h2>
            <div className="mb-4">
              <ModelSelector
                onSelect={handleSelectModel}
                compact
                loadOnSelect={false}
                ext="onnx,pt"
              />
              {needsExport && (
                <div className="mt-2 px-3 py-2.5 rounded-lg bg-amber-50 dark:bg-amber-950/20 border border-amber-200 dark:border-amber-900/40">
                  <div className="flex items-start gap-2 text-xs text-amber-800 dark:text-amber-300">
                    <ArrowRightLeft className="w-3.5 h-3.5 shrink-0 mt-0.5" />
                    <div>
                      <p className="font-medium">PT 模型需要先转换为 ONNX</p>
                      <p className="text-amber-600 dark:text-amber-400 mt-0.5">转换只需一次，之后可直接使用该 ONNX 模型进行推理</p>
                    </div>
                  </div>
                </div>
              )}
              {exporting && (
                <div className="mt-2 px-3 py-2.5 rounded-lg bg-blue-50 dark:bg-blue-950/20 border border-blue-200 dark:border-blue-900/40">
                  <div className="flex items-center gap-2 text-xs text-blue-700 dark:text-blue-300">
                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                    <span className="font-medium">正在将 PT 模型转换为 ONNX 格式…</span>
                  </div>
                  <p className="text-[10px] text-blue-500 dark:text-blue-400 mt-1 ml-5">转换完成后将自动开始捕获</p>
                </div>
              )}
              {exportSuccess && (
                <div className="mt-2 px-3 py-2.5 rounded-lg bg-emerald-50 dark:bg-emerald-950/20 border border-emerald-200 dark:border-emerald-900/40">
                  <div className="flex items-center gap-2 text-xs text-emerald-700 dark:text-emerald-300">
                    <CheckCircle2 className="w-3.5 h-3.5" />
                    <span className="font-medium">转换完成！</span>
                  </div>
                  <p className="text-[10px] text-emerald-500 dark:text-emerald-400 mt-1 ml-5">已生成 ONNX 模型，后续推理无需再次转换</p>
                </div>
              )}
              {exportError && (
                <div className="mt-2 px-3 py-2.5 rounded-lg bg-red-50 dark:bg-red-950/20 border border-red-200 dark:border-red-900/40">
                  <div className="flex items-start gap-2 text-xs text-red-700 dark:text-red-300">
                    <AlertCircle className="w-3.5 h-3.5 shrink-0 mt-0.5" />
                    <div>
                      <p className="font-medium">转换失败</p>
                      <p className="text-red-500 dark:text-red-400 mt-0.5">{exportError}</p>
                    </div>
                  </div>
                </div>
              )}
            </div>
            {!captureState?.running ? (
              <button
                onClick={startCapture}
                disabled={!selectedModel || exporting}
                className="w-full flex items-center justify-center gap-2 py-2.5 rounded-xl bg-gradient-to-r from-emerald-500 to-teal-500 text-white text-sm font-medium hover:shadow-lg hover:shadow-emerald-500/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {exporting ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" />
                    正在转换 ONNX…
                  </>
                ) : (
                  <>
                    <Play className="w-4 h-4" />
                    开始捕获
                  </>
                )}
              </button>
            ) : (
              <button
                onClick={stopCapture}
                className="w-full flex items-center justify-center gap-2 py-2.5 rounded-xl bg-gradient-to-r from-red-500 to-rose-500 text-white text-sm font-medium hover:shadow-lg hover:shadow-red-500/20 transition-all"
              >
                <Square className="w-4 h-4" />
                停止捕获
              </button>
            )}
          </div>

          <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
              <Settings2 className="w-4 h-4 text-blue-500" />
              设置
            </h2>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-xs text-gray-500 dark:text-gray-400">置信度阈值</span>
                <div className="flex items-center gap-2">
                  <span className="text-[10px] text-gray-400 dark:text-gray-500 w-8 text-right">{confThreshold.toFixed(2)}</span>
                  <input
                    type="range"
                    min={0.01}
                    max={1}
                    step={0.01}
                    value={confThreshold}
                    onChange={(e) => setConfThreshold(parseFloat(e.target.value))}
                    className="w-24"
                  />
                </div>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-xs text-gray-500 dark:text-gray-400">显示检测框</span>
                <input type="checkbox" checked={showBoxes} onChange={(e) => setShowBoxes(e.target.checked)} />
              </div>
              <div className="flex items-center justify-between">
                <span className="text-xs text-gray-500 dark:text-gray-400">显示标签</span>
                <input type="checkbox" checked={showLabels} onChange={(e) => setShowLabels(e.target.checked)} />
              </div>
            </div>
          </div>

          <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
              <BrainCircuit className="w-4 h-4 text-purple-500" />
              状态
            </h2>
            <div className="space-y-3">
              <div className="flex justify-between text-xs">
                <span className="text-gray-400 dark:text-gray-500">运行状态</span>
                <span className={captureState?.running ? "text-emerald-600 dark:text-emerald-400 font-medium" : "text-gray-500 dark:text-gray-400"}>
                  {captureState?.running ? "运行中" : "已停止"}
                </span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-gray-400 dark:text-gray-500">帧率</span>
                <span className="text-gray-700 dark:text-gray-300 font-mono">{captureState?.fps.toFixed(1) ?? "0.0"} FPS</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-gray-400 dark:text-gray-500">延迟</span>
                <span className="text-gray-700 dark:text-gray-300 font-mono">{latency.toFixed(0)} ms</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-gray-400 dark:text-gray-500">检测目标</span>
                <span className="text-gray-700 dark:text-gray-300 font-mono">{captureState?.detections.length ?? 0}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
