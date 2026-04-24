import { useState, useRef, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Film,
  ScanEye,
  FileVideo,
  RotateCcw,
  BrainCircuit,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Zap,
  Eye,
  EyeOff,
  ArrowRightLeft,
  Loader2,
  CheckCircle2,
  AlertCircle,
} from "lucide-react";
import ModelSelector from "../components/ModelSelector";
import { usePtExport } from "../hooks/usePtExport";

interface Detection {
  class_id: number;
  confidence: number;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

const COLORS = [
  "#ef4444", "#22c55e", "#3b82f6", "#f59e0b", "#8b5cf6",
  "#ec4899", "#06b6d4", "#f97316", "#84cc16", "#6366f1",
];

export default function Video() {
  const [videoPath, setVideoPath] = useState<string | null>(null);
  const [framePath, setFramePath] = useState<string | null>(null);
  const [detections, setDetections] = useState<Detection[]>([]);
  const [inferring, setInferring] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [selectedModel, setSelectedModel] = useState("");
  const [confThreshold, setConfThreshold] = useState(0.25);
  const [showBoxes, setShowBoxes] = useState(true);
  const [showLabels, setShowLabels] = useState(true);
  const [autoInfer, setAutoInfer] = useState(false);
  const [isPlaying, setIsPlaying] = useState(false);
  const { needsExport, exporting, exportError, exportSuccess, checkModel, doExport } = usePtExport();
  const videoRef = useRef<HTMLVideoElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const autoInferRef = useRef<number>();

  const openVideo = useCallback(async () => {
    const file = await open({
      filters: [{ name: "Video", extensions: ["mp4", "avi", "mov", "mkv", "webm"] }],
    });
    if (file) {
      setVideoPath(file);
      setFramePath(null);
      setDetections([]);
      setAutoInfer(false);
    }
  }, []);

  const handleTimeUpdate = () => {
    const v = videoRef.current;
    if (v) setCurrentTime(v.currentTime);
  };

  const handleLoadedMetadata = () => {
    const v = videoRef.current;
    if (v) setDuration(v.duration);
  };

  const togglePlay = () => {
    const v = videoRef.current;
    if (!v) return;
    if (v.paused) {
      v.play();
      setIsPlaying(true);
    } else {
      v.pause();
      setIsPlaying(false);
    }
  };

  const step = (delta: number) => {
    const v = videoRef.current;
    if (!v) return;
    v.currentTime = Math.max(0, Math.min(v.duration || 0, v.currentTime + delta));
  };

  const handleSelectModel = async (_name: string, path: string) => {
    setSelectedModel(path);
    await checkModel(path);
  };

  const runFrameInference = useCallback(async () => {
    if (!videoPath || !videoRef.current || !selectedModel) {
      if (!selectedModel) alert("请先选择一个模型");
      return;
    }
    if (needsExport) {
      try { await doExport(selectedModel); } catch { return; }
    }
    const time = videoRef.current.currentTime;
    setInferring(true);
    try {
      const frame = await invoke<string>("extract_video_frame", {
        videoPath,
        timestampSec: time,
      });
      setFramePath(frame);
      const result = await invoke<Detection[]>("run_inference_image", {
        modelPath: selectedModel,
        imagePath: frame,
        confThreshold: confThreshold,
      });
      setDetections(result);
    } catch (e) {
      console.error(e);
    } finally {
      setInferring(false);
    }
  }, [videoPath, selectedModel, confThreshold, needsExport, doExport]);

  // Auto inference loop
  useEffect(() => {
    if (!autoInfer || !videoPath || !selectedModel) {
      if (autoInferRef.current) {
        clearInterval(autoInferRef.current);
        autoInferRef.current = undefined;
      }
      return;
    }
    autoInferRef.current = window.setInterval(() => {
      if (videoRef.current && !videoRef.current.paused) {
        runFrameInference();
      }
    }, 2000);
    return () => {
      if (autoInferRef.current) {
        clearInterval(autoInferRef.current);
        autoInferRef.current = undefined;
      }
    };
  }, [autoInfer, videoPath, selectedModel, runFrameInference]);

  // Draw result on canvas
  useEffect(() => {
    if (!framePath || !canvasRef.current) return;
    const canvas = canvasRef.current;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const img = document.createElement("img");
    img.crossOrigin = "anonymous";
    img.onload = () => {
      canvas.width = img.naturalWidth;
      canvas.height = img.naturalHeight;
      ctx.drawImage(img, 0, 0);

      if (showBoxes && detections.length > 0) {
        for (const d of detections) {
          // 后端返回的是绝对像素坐标，直接使用
          const x = d.x1;
          const y = d.y1;
          const w = d.x2 - d.x1;
          const h = d.y2 - d.y1;
          const color = COLORS[d.class_id % COLORS.length];

          ctx.strokeStyle = color;
          ctx.lineWidth = Math.max(2, canvas.width / 400);
          ctx.strokeRect(x, y, w, h);

          if (showLabels) {
            const label = `ID:${d.class_id} ${(d.confidence * 100).toFixed(0)}%`;
            ctx.font = `bold ${Math.max(12, canvas.width / 60)}px sans-serif`;
            const tw = ctx.measureText(label).width;
            const th = Math.max(16, canvas.width / 60);
            ctx.fillStyle = color;
            ctx.fillRect(x, y - th, tw + 8, th);
            ctx.fillStyle = "#fff";
            ctx.fillText(label, x + 4, y - 4);
          }
        }
      }
    };
    img.src = `file://${framePath}`;
  }, [framePath, detections, showBoxes, showLabels]);

  const formatTime = (t: number) => {
    const m = Math.floor(t / 60);
    const s = Math.floor(t % 60);
    return `${m}:${s.toString().padStart(2, "0")}`;
  };

  if (!videoPath) {
    return (
      <div className="min-h-full flex flex-col items-center justify-center p-8">
        <div className="w-20 h-20 rounded-3xl bg-gradient-to-br from-purple-100 to-pink-100 dark:from-purple-900/30 dark:to-pink-900/30 flex items-center justify-center mb-6">
          <Film className="w-10 h-10 text-purple-500" />
        </div>
        <h1 className="text-xl font-bold text-gray-900 dark:text-white mb-2">视频推理</h1>
        <p className="text-sm text-gray-500 dark:text-gray-400 mb-8">选择视频文件，逐帧进行 YOLO 目标检测</p>
        <button
          onClick={openVideo}
          className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-gradient-to-r from-purple-500 to-pink-500 text-white text-sm font-medium hover:shadow-lg hover:shadow-purple-500/20 transition-all"
        >
          <FileVideo className="w-4 h-4" />
          打开视频
        </button>
      </div>
    );
  }

  return (
    <div className="min-h-full p-8">
      <h1 className="text-2xl font-bold text-gray-900 dark:text-white mb-1">视频推理</h1>
      <p className="text-sm text-gray-500 dark:text-gray-400 mb-8 truncate">{videoPath}</p>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-5">
        {/* 播放器 */}
        <div className="lg:col-span-8 space-y-4">
          <div className="bg-black rounded-2xl overflow-hidden aspect-video relative shadow-lg flex items-center justify-center">
            {framePath ? (
              <canvas
                ref={canvasRef}
                className="max-w-full max-h-full object-contain"
              />
            ) : (
              <video
                ref={videoRef}
                src={`file://${videoPath}`}
                className="w-full h-full"
                onTimeUpdate={handleTimeUpdate}
                onLoadedMetadata={handleLoadedMetadata}
                onPlay={() => setIsPlaying(true)}
                onPause={() => setIsPlaying(false)}
              />
            )}
          </div>

          {/* 控制栏 */}
          <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-4 shadow-sm space-y-3">
            {/* 播放控制 */}
            <div className="flex items-center gap-2">
              <button
                onClick={togglePlay}
                className="w-10 h-10 rounded-xl bg-gradient-to-r from-purple-500 to-pink-500 text-white flex items-center justify-center hover:shadow-lg hover:shadow-purple-500/20 transition"
                title={isPlaying ? "暂停" : "播放"}
              >
                {isPlaying ? <Pause className="w-4 h-4" /> : <Play className="w-4 h-4" />}
              </button>
              <button
                onClick={() => step(-1)}
                className="w-9 h-9 rounded-lg bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 flex items-center justify-center hover:bg-gray-200 dark:hover:bg-gray-700 transition"
                title="后退 1 秒"
              >
                <SkipBack className="w-4 h-4" />
              </button>
              <button
                onClick={() => step(1)}
                className="w-9 h-9 rounded-lg bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 flex items-center justify-center hover:bg-gray-200 dark:hover:bg-gray-700 transition"
                title="前进 1 秒"
              >
                <SkipForward className="w-4 h-4" />
              </button>

              <div className="flex-1 mx-3">
                <div className="w-full bg-gray-100 dark:bg-gray-900 rounded-full h-1.5">
                  <div
                    className="bg-gradient-to-r from-purple-500 to-pink-500 h-1.5 rounded-full transition-all"
                    style={{ width: `${duration > 0 ? (currentTime / duration) * 100 : 0}%` }}
                  />
                </div>
                <div className="flex justify-between mt-1">
                  <span className="text-[10px] text-gray-400 font-mono">{formatTime(currentTime)}</span>
                  <span className="text-[10px] text-gray-400 font-mono">{formatTime(duration)}</span>
                </div>
              </div>

              <button
                onClick={() => {
                  setFramePath(null);
                  setDetections([]);
                }}
                className="p-2 rounded-lg bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700 transition"
                title="返回视频"
              >
                <RotateCcw className="w-4 h-4" />
              </button>
            </div>

            {/* 推理控制 */}
            <div className="flex items-center gap-3 pt-2 border-t border-gray-100 dark:border-gray-800">
              <button
                onClick={runFrameInference}
                disabled={inferring || !selectedModel || exporting}
                className="px-4 py-2 rounded-xl bg-gradient-to-r from-purple-500 to-pink-500 text-white text-xs font-medium hover:shadow-lg hover:shadow-purple-500/20 disabled:opacity-50 disabled:cursor-not-allowed transition-all flex items-center gap-2"
              >
                {exporting ? (
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <ScanEye className="w-3.5 h-3.5" />
                )}
                {exporting ? "正在转换 ONNX…" : inferring ? "推理中..." : "当前帧推理"}
              </button>

              <button
                onClick={() => setAutoInfer(!autoInfer)}
                disabled={!selectedModel || exporting}
                className={`px-3 py-2 rounded-xl text-xs font-medium flex items-center gap-1.5 transition-all ${
                  autoInfer
                    ? "bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400"
                    : "bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700"
                } disabled:opacity-50 disabled:cursor-not-allowed`}
              >
                <Zap className="w-3.5 h-3.5" />
                {autoInfer ? "自动推理中" : "自动推理"}
              </button>

              <div className="flex-1" />

              <button
                onClick={() => setShowBoxes(!showBoxes)}
                className="p-2 rounded-lg text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition"
                title={showBoxes ? "隐藏检测框" : "显示检测框"}
              >
                {showBoxes ? <Eye className="w-4 h-4" /> : <EyeOff className="w-4 h-4" />}
              </button>
              <button
                onClick={() => setShowLabels(!showLabels)}
                className="p-2 rounded-lg text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition"
                title={showLabels ? "隐藏标签" : "显示标签"}
              >
                {showLabels ? <span className="text-xs font-bold">Aa</span> : <span className="text-xs text-gray-400 line-through">Aa</span>}
              </button>
            </div>
          </div>
        </div>

        {/* 侧边栏 */}
        <div className="lg:col-span-4 space-y-5">
          <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
              <BrainCircuit className="w-4 h-4 text-purple-500" />
              模型与参数
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
                  <p className="text-[10px] text-blue-500 dark:text-blue-400 mt-1 ml-5">转换完成后将自动开始推理</p>
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
            <div className="flex items-center justify-between mb-4">
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
            <button
              onClick={openVideo}
              className="w-full py-2.5 rounded-xl bg-gray-100 dark:bg-gray-900/50 text-gray-700 dark:text-gray-300 text-xs font-medium hover:bg-gray-200 dark:hover:bg-gray-800 transition-colors"
            >
              打开其他视频
            </button>
          </div>

          <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-4">检测结果</h2>
            {detections.length === 0 ? (
              <p className="text-xs text-gray-400 dark:text-gray-500 text-center py-4">暂无检测结果</p>
            ) : (
              <div className="space-y-2 max-h-64 overflow-auto">
                {detections.map((d, i) => {
                  const color = COLORS[d.class_id % COLORS.length];
                  return (
                    <div key={i} className="flex items-center justify-between px-3 py-2 rounded-xl bg-gray-50 dark:bg-gray-900/50 text-xs">
                      <div className="flex items-center gap-2">
                        <span className="w-2 h-2 rounded-full" style={{ backgroundColor: color }} />
                        <span className="text-gray-600 dark:text-gray-400">类别 {d.class_id}</span>
                      </div>
                      <span className="font-medium" style={{ color }}>{(d.confidence * 100).toFixed(1)}%</span>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
