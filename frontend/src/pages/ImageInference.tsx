import { useState, useRef, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Image, ScanEye, Upload, BrainCircuit, X, Eye, EyeOff } from "lucide-react";
import ModelSelector from "../components/ModelSelector";

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

export default function ImageInference() {
  const [imagePath, setImagePath] = useState<string | null>(null);
  const [detections, setDetections] = useState<Detection[]>([]);
  const [inferring, setInferring] = useState(false);
  const [selectedModel, setSelectedModel] = useState("");
  const [confThreshold, setConfThreshold] = useState(0.25);
  const [showBoxes, setShowBoxes] = useState(true);
  const [showLabels, setShowLabels] = useState(true);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  const openImage = useCallback(async () => {
    const file = await open({
      filters: [{ name: "Image", extensions: ["jpg", "jpeg", "png", "bmp", "webp"] }],
    });
    if (file) {
      setImagePath(file);
      setDetections([]);
    }
  }, []);

  const runInference = async () => {
    if (!imagePath || !selectedModel) {
      if (!selectedModel) alert("请先选择一个 ONNX 模型");
      return;
    }
    setInferring(true);
    try {
      await invoke("load_model", { modelPath: selectedModel });
      const result = await invoke<Detection[]>("run_inference_image", {
        imagePath,
        confThreshold: confThreshold,
      });
      setDetections(result);
    } catch (e) {
      console.error(e);
    } finally {
      setInferring(false);
    }
  };

  // Draw image + detections on canvas
  useEffect(() => {
    if (!imagePath || !canvasRef.current) return;
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
          const x = d.x1 * canvas.width;
          const y = d.y1 * canvas.height;
          const w = (d.x2 - d.x1) * canvas.width;
          const h = (d.y2 - d.y1) * canvas.height;
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
    img.src = `file://${imagePath}`;
  }, [imagePath, detections, showBoxes, showLabels]);

  if (!imagePath) {
    return (
      <div className="min-h-full flex flex-col items-center justify-center p-8">
        <div className="w-20 h-20 rounded-3xl bg-gradient-to-br from-blue-100 to-cyan-100 dark:from-blue-900/30 dark:to-cyan-900/30 flex items-center justify-center mb-6">
          <Image className="w-10 h-10 text-blue-500" />
        </div>
        <h1 className="text-xl font-bold text-gray-900 dark:text-white mb-2">图片推理</h1>
        <p className="text-sm text-gray-500 dark:text-gray-400 mb-8">选择图片文件，进行 YOLO 目标检测</p>
        <button
          onClick={openImage}
          className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-gradient-to-r from-blue-500 to-cyan-500 text-white text-sm font-medium hover:shadow-lg hover:shadow-blue-500/20 transition-all"
        >
          <Upload className="w-4 h-4" />
          选择图片
        </button>
      </div>
    );
  }

  return (
    <div className="min-h-full p-8">
      <h1 className="text-2xl font-bold text-gray-900 dark:text-white mb-1">图片推理</h1>
      <p className="text-sm text-gray-500 dark:text-gray-400 mb-8 truncate">{imagePath}</p>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-5">
        {/* 图片预览区 */}
        <div className="lg:col-span-8">
          <div className="bg-black rounded-2xl overflow-hidden relative shadow-lg flex items-center justify-center">
            <canvas
              ref={canvasRef}
              className="max-w-full max-h-[70vh] object-contain"
            />
          </div>
        </div>

        {/* 侧边栏 */}
        <div className="lg:col-span-4 space-y-5">
          <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
              <BrainCircuit className="w-4 h-4 text-blue-500" />
              模型与参数
            </h2>
            <div className="mb-4">
              <ModelSelector
                onSelect={(_name, path) => setSelectedModel(path)}
                compact
                loadOnSelect={false}
                ext="onnx"
              />
            </div>
            <div className="flex items-center justify-between mb-4">
              <span className="text-xs text-gray-500 dark:text-gray-400">置信度阈值</span>
              <div className="flex items-center gap-2">
                <span className="text-[10px] text-gray-400 w-8 text-right">{confThreshold.toFixed(2)}</span>
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
            <div className="flex items-center justify-between mb-4">
              <span className="text-xs text-gray-500 dark:text-gray-400">显示检测框</span>
              <button
                onClick={() => setShowBoxes(!showBoxes)}
                className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 transition"
              >
                {showBoxes ? <Eye className="w-4 h-4" /> : <EyeOff className="w-4 h-4" />}
              </button>
            </div>
            <div className="flex items-center justify-between mb-4">
              <span className="text-xs text-gray-500 dark:text-gray-400">显示标签</span>
              <button
                onClick={() => setShowLabels(!showLabels)}
                className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 transition"
              >
                {showLabels ? <Eye className="w-4 h-4" /> : <EyeOff className="w-4 h-4" />}
              </button>
            </div>
            <button
              onClick={runInference}
              disabled={inferring || !selectedModel}
              className="w-full py-2.5 rounded-xl bg-gradient-to-r from-blue-500 to-cyan-500 text-white text-xs font-medium hover:shadow-lg hover:shadow-blue-500/20 disabled:opacity-50 disabled:cursor-not-allowed transition-all flex items-center justify-center gap-2"
            >
              <ScanEye className="w-3.5 h-3.5" />
              {inferring ? "推理中..." : "开始推理"}
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

          <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
            <button
              onClick={() => {
                setImagePath(null);
                setDetections([]);
              }}
              className="w-full py-2.5 rounded-xl bg-gray-100 dark:bg-gray-900/50 text-gray-700 dark:text-gray-300 text-xs font-medium hover:bg-gray-200 dark:hover:bg-gray-800 transition-colors flex items-center justify-center gap-2"
            >
              <X className="w-3.5 h-3.5" />
              关闭图片
            </button>
            <button
              onClick={openImage}
              className="w-full mt-2 py-2.5 rounded-xl bg-gray-100 dark:bg-gray-900/50 text-gray-700 dark:text-gray-300 text-xs font-medium hover:bg-gray-200 dark:hover:bg-gray-800 transition-colors flex items-center justify-center gap-2"
            >
              <Upload className="w-3.5 h-3.5" />
              选择其他图片
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
