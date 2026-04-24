import { useState, useRef, useCallback, useEffect } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  FolderOpen,
  MousePointer,
  Hand,
  Square,
  Trash2,
  Save,
  ZoomIn,
  ZoomOut,
  ChevronLeft,
  ChevronRight,
  Images,
} from "lucide-react";
import { useProject } from "../contexts/ProjectContext";

interface YoloAnnotation {
  class_id: number;
  x_center: number;
  y_center: number;
  width: number;
  height: number;
}

interface PixelBox {
  id: number;
  classId: number;
  x: number; // 像素坐标（基于原始图片尺寸）
  y: number;
  w: number;
  h: number;
}

type Tool = "select" | "pan" | "rect";

const TOOLS: { key: Tool; label: string; icon: React.ReactNode }[] = [
  { key: "select", label: "选择", icon: <MousePointer className="w-3.5 h-3.5" /> },
  { key: "pan", label: "平移", icon: <Hand className="w-3.5 h-3.5" /> },
  { key: "rect", label: "矩形", icon: <Square className="w-3.5 h-3.5" /> },
];

// YOLO 归一化 → 像素坐标
function yoloToPixel(ann: YoloAnnotation, imgW: number, imgH: number): PixelBox {
  return {
    id: Math.random(),
    classId: ann.class_id,
    x: (ann.x_center - ann.width / 2) * imgW,
    y: (ann.y_center - ann.height / 2) * imgH,
    w: ann.width * imgW,
    h: ann.height * imgH,
  };
}

// 像素坐标 → YOLO 归一化
function pixelToYolo(box: PixelBox, imgW: number, imgH: number): YoloAnnotation {
  const x_center = (box.x + box.w / 2) / imgW;
  const y_center = (box.y + box.h / 2) / imgH;
  const width = box.w / imgW;
  const height = box.h / imgH;
  return {
    class_id: box.classId,
    x_center: Math.max(0, Math.min(1, x_center)),
    y_center: Math.max(0, Math.min(1, y_center)),
    width: Math.max(0, Math.min(1, width)),
    height: Math.max(0, Math.min(1, height)),
  };
}

// 将本地文件路径转为可显示的 URL（Tauri v2 使用 convertFileSrc）
function pathToSrc(path: string): string {
  if (!path) return "";
  return convertFileSrc(path);
}

export default function Annotation() {
  const [images, setImages] = useState<string[]>([]);
  const [selectedIdx, setSelectedIdx] = useState<number>(0);
  const [boxes, setBoxes] = useState<PixelBox[]>([]);
  const [currentClass, setCurrentClass] = useState(0);
  const [tool, setTool] = useState<Tool>("rect");
  const [drawing, setDrawing] = useState(false);
  const [drawStart, setDrawStart] = useState<{ x: number; y: number } | null>(null);
  const [drawCurrent, setDrawCurrent] = useState<{ x: number; y: number } | null>(null);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [imgSize, setImgSize] = useState({ w: 0, h: 0 });
  const [folderPath, setFolderPath] = useState<string>("");
  const [, setLoading] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [split, setSplit] = useState<"train" | "val">("train");
  const containerRef = useRef<HTMLDivElement>(null);
  const imgRef = useRef<HTMLImageElement>(null);
  const { project } = useProject();

  // 加载文件夹
  const loadFolder = useCallback(async (dir?: string) => {
    const target = dir ?? await open({ directory: true });
    if (!target) return;
    setLoading(true);
    try {
      const list = await invoke<string[]>("list_images", { folder: target, recursive: true });
      setImages(list);
      setFolderPath(target);
      setSelectedIdx(0);
      setBoxes([]);
      setHasChanges(false);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }, []);

  // 自动加载当前项目的图片（根据当前 split）
  useEffect(() => {
    if (!project || folderPath) return;
    const tryLoad = async () => {
      // 使用项目配置中的图片路径，支持相对和绝对路径
      const imgConfig = project.images;
      const splitPath = split === "train" ? imgConfig?.train : imgConfig?.val;
      const imagesDir = splitPath
        ? (splitPath.startsWith("/") || /^[A-Za-z]:/.test(splitPath)
            ? splitPath
            : project.path + "/" + splitPath)
        : project.path + "/images/" + split;

      try {
        const list = await invoke<string[]>("list_images", { folder: imagesDir, recursive: true });
        if (list.length > 0) {
          setImages(list);
          setFolderPath(imagesDir);
          setSelectedIdx(0);
          setBoxes([]);
          setHasChanges(false);
          return;
        }
      } catch { /* ignore */ }

      // Fallback: 尝试旧式 images/ 目录
      const fallbackDir = project.path + "/images";
      try {
        const list = await invoke<string[]>("list_images", { folder: fallbackDir, recursive: true });
        if (list.length > 0) {
          setImages(list);
          setFolderPath(fallbackDir);
          setSelectedIdx(0);
          setBoxes([]);
          setHasChanges(false);
        }
      } catch { /* ignore */ }
    };
    tryLoad();
  }, [project, folderPath, split]);

  // 加载当前图片的标注
  const loadAnnotations = useCallback(async (imagePath: string, w: number, h: number) => {
    try {
      const anns = await invoke<YoloAnnotation[]>("read_yolo_labels", { imagePath });
      setBoxes(anns.map((a) => yoloToPixel(a, w, h)));
      setHasChanges(false);
    } catch (e) {
      console.error(e);
      setBoxes([]);
    }
  }, []);

  // 图片加载完成
  const handleImageLoad = useCallback(async () => {
    const img = imgRef.current;
    if (!img) return;
    const w = img.naturalWidth;
    const h = img.naturalHeight;
    setImgSize({ w, h });
    setZoom(1);
    setPan({ x: 0, y: 0 });

    const currentImage = images[selectedIdx];
    if (currentImage) {
      await loadAnnotations(currentImage, w, h);
    }
  }, [images, selectedIdx, loadAnnotations]);

  // 切换图片时重置
  useEffect(() => {
    if (images.length > 0 && selectedIdx < images.length) {
      setBoxes([]);
      setHasChanges(false);
      // 标注会在图片加载完成后通过 handleImageLoad 读取
    }
  }, [selectedIdx, images]);

  // 将鼠标坐标转换为图片上的像素坐标（考虑缩放和平移）
  const screenToPixel = useCallback(
    (sx: number, sy: number) => {
      const container = containerRef.current;
      if (!container) return { x: 0, y: 0 };
      const rect = container.getBoundingClientRect();
      const relX = (sx - rect.left - pan.x) / zoom;
      const relY = (sy - rect.top - pan.y) / zoom;
      return { x: relX, y: relY };
    },
    [zoom, pan]
  );

  const handleMouseDown = (e: React.MouseEvent) => {
    if (tool === "pan") {
      setDrawStart({ x: e.clientX - pan.x, y: e.clientY - pan.y });
      setDrawing(true);
      return;
    }
    if (tool !== "rect" || !imgRef.current) return;
    const p = screenToPixel(e.clientX, e.clientY);
    setDrawStart(p);
    setDrawCurrent(p);
    setDrawing(true);
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!drawing) return;
    if (tool === "pan" && drawStart) {
      setPan({
        x: e.clientX - drawStart.x,
        y: e.clientY - drawStart.y,
      });
      return;
    }
    if (tool === "rect") {
      setDrawCurrent(screenToPixel(e.clientX, e.clientY));
    }
  };

  const handleMouseUp = () => {
    if (!drawing || !drawStart) return;

    if (tool === "rect" && drawCurrent) {
      const x = Math.min(drawStart.x, drawCurrent.x);
      const y = Math.min(drawStart.y, drawCurrent.y);
      const w = Math.abs(drawCurrent.x - drawStart.x);
      const h = Math.abs(drawCurrent.y - drawStart.y);
      if (w > 5 && h > 5) {
        setBoxes((prev) => [
          ...prev,
          { id: Date.now(), classId: currentClass, x, y, w, h },
        ]);
        setHasChanges(true);
      }
    }

    setDrawing(false);
    setDrawStart(null);
    setDrawCurrent(null);
  };

  const deleteBox = (id: number) => {
    setBoxes((prev) => prev.filter((b) => b.id !== id));
    setHasChanges(true);
  };

  const clearAll = () => {
    setBoxes([]);
    setHasChanges(true);
  };

  const saveAnnotations = async () => {
    const currentImage = images[selectedIdx];
    if (!currentImage || imgSize.w === 0) return;
    const yoloBoxes = boxes.map((b) => pixelToYolo(b, imgSize.w, imgSize.h));
    try {
      await invoke("save_yolo_labels", {
        imagePath: currentImage,
        annotations: yoloBoxes,
      });
      setHasChanges(false);
    } catch (e) {
      console.error(e);
    }
  };

  const currentImage = images[selectedIdx];

  // 绘制中的临时框
  const tempBox =
    drawing && tool === "rect" && drawStart && drawCurrent
      ? {
          x: Math.min(drawStart.x, drawCurrent.x),
          y: Math.min(drawStart.y, drawCurrent.y),
          w: Math.abs(drawCurrent.x - drawStart.x),
          h: Math.abs(drawCurrent.y - drawCurrent.y),
        }
      : null;

  return (
    <div className="h-full flex">
      {/* 左：图片列表 */}
      <div className="w-56 shrink-0 bg-white dark:bg-surface-dark border-r border-gray-200 dark:border-gray-800 flex flex-col">
        <div className="p-3 border-b border-gray-200 dark:border-gray-800 space-y-2">
          {project && (
            <div className="flex rounded-xl bg-gray-100 dark:bg-gray-900/50 p-0.5">
              <button
                onClick={() => setSplit("train")}
                className={`flex-1 flex items-center justify-center gap-1 px-2 py-1.5 rounded-lg text-[10px] font-medium transition-all ${
                  split === "train"
                    ? "bg-white dark:bg-gray-800 text-brand-primary shadow-sm"
                    : "text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
                }`}
              >
                <Images className="w-3 h-3" />
                训练集
              </button>
              <button
                onClick={() => setSplit("val")}
                className={`flex-1 flex items-center justify-center gap-1 px-2 py-1.5 rounded-lg text-[10px] font-medium transition-all ${
                  split === "val"
                    ? "bg-white dark:bg-gray-800 text-brand-primary shadow-sm"
                    : "text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
                }`}
              >
                <Images className="w-3 h-3" />
                验证集
              </button>
            </div>
          )}
          <button
            onClick={() => loadFolder()}
            className="w-full flex items-center gap-2 px-3 py-2 rounded-xl bg-gray-100 dark:bg-gray-900/50 text-xs text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-800 transition-colors"
          >
            <FolderOpen className="w-3.5 h-3.5" />
            打开文件夹
          </button>
        </div>
        <div className="flex-1 overflow-auto p-2 space-y-1">
          {images.map((img, i) => (
            <button
              key={img}
              onClick={() => setSelectedIdx(i)}
              className={`w-full text-left px-3 py-2 rounded-xl text-xs transition-all truncate ${
                selectedIdx === i
                  ? "bg-brand-primary/10 text-brand-primary font-medium"
                  : "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
              }`}
            >
              {img.split(/[\\/]/).pop()}
            </button>
          ))}
          {images.length === 0 && (
            <p className="text-xs text-gray-400 dark:text-gray-500 text-center py-8">未加载图片</p>
          )}
        </div>
        {images.length > 0 && (
          <div className="p-2 border-t border-gray-200 dark:border-gray-800 flex items-center justify-between text-[10px] text-gray-400">
            <button onClick={() => setSelectedIdx((i) => Math.max(0, i - 1))} className="p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded">
              <ChevronLeft className="w-3 h-3" />
            </button>
            <span>
              {selectedIdx + 1} / {images.length}
            </span>
            <button onClick={() => setSelectedIdx((i) => Math.min(images.length - 1, i + 1))} className="p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded">
              <ChevronRight className="w-3 h-3" />
            </button>
          </div>
        )}
      </div>

      {/* 中：画布 */}
      <div className="flex-1 flex flex-col min-w-0 bg-gray-100 dark:bg-gray-950">
        {/* 工具栏 */}
        <div className="h-11 flex items-center gap-1 px-3 border-b border-gray-200 dark:border-gray-800 bg-white dark:bg-surface-dark">
          {TOOLS.map((t) => (
            <button
              key={t.key}
              onClick={() => setTool(t.key)}
              title={t.label}
              className={`p-2 rounded-xl transition-all ${
                tool === t.key
                  ? "bg-brand-primary/10 text-brand-primary"
                  : "text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
              }`}
            >
              {t.icon}
            </button>
          ))}

          <div className="w-px h-5 bg-gray-200 dark:bg-gray-700 mx-2" />

          <button
            onClick={() => setZoom((z) => Math.max(0.3, z - 0.15))}
            className="p-2 rounded-xl text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition"
            title="缩小"
          >
            <ZoomOut className="w-3.5 h-3.5" />
          </button>
          <span className="text-[10px] text-gray-400 dark:text-gray-500 w-10 text-center">{Math.round(zoom * 100)}%</span>
          <button
            onClick={() => setZoom((z) => Math.min(5, z + 0.15))}
            className="p-2 rounded-xl text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition"
            title="放大"
          >
            <ZoomIn className="w-3.5 h-3.5" />
          </button>

          <div className="flex-1" />

          {hasChanges && (
            <span className="text-[10px] text-amber-500 mr-2">有未保存的更改</span>
          )}

          <button
            onClick={clearAll}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition"
          >
            <Trash2 className="w-3 h-3" />
            清空
          </button>
          <button
            onClick={saveAnnotations}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs text-brand-primary hover:bg-blue-50 dark:hover:bg-blue-900/20 transition"
          >
            <Save className="w-3 h-3" />
            保存
          </button>
        </div>

        {/* 图片区域 */}
        <div
          ref={containerRef}
          className="flex-1 relative overflow-hidden cursor-crosshair"
          onMouseDown={handleMouseDown}
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
          onMouseLeave={handleMouseUp}
        >
          {currentImage ? (
            <div
              className="absolute top-1/2 left-1/2 origin-center"
              style={{
                transform: `translate(-50%, -50%) translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
              }}
            >
              <img
                ref={imgRef}
                src={pathToSrc(currentImage)}
                alt="annotation"
                className="block select-none"
                draggable={false}
                onLoad={handleImageLoad}
                onError={(e) => {
                  console.error("Image load failed:", currentImage, e);
                }}
              />

              {/* 标注框 overlay */}
              {imgSize.w > 0 && (
                <div className="absolute inset-0 pointer-events-none">
                  {boxes.map((box) => (
                    <div
                      key={box.id}
                      className="absolute border-2 border-pink-500/80 pointer-events-auto group"
                      style={{
                        left: `${box.x}px`,
                        top: `${box.y}px`,
                        width: `${box.w}px`,
                        height: `${box.h}px`,
                      }}
                    >
                      <span className="absolute -top-4 left-0 bg-pink-500 text-white text-[9px] px-1 py-0.5 rounded font-medium whitespace-nowrap">
                        类别{box.classId}
                      </span>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          deleteBox(box.id);
                        }}
                        className="absolute -top-2 -right-2 w-4 h-4 rounded-full bg-red-500 text-white flex items-center justify-center opacity-0 group-hover:opacity-100 transition text-[8px]"
                      >
                        ×
                      </button>
                    </div>
                  ))}

                  {/* 绘制中的临时框 */}
                  {tempBox && (
                    <div
                      className="absolute border-2 border-dashed border-pink-400/60"
                      style={{
                        left: `${tempBox.x}px`,
                        top: `${tempBox.y}px`,
                        width: `${tempBox.w}px`,
                        height: `${tempBox.h}px`,
                      }}
                    />
                  )}
                </div>
              )}
            </div>
          ) : (
            <div className="absolute inset-0 flex items-center justify-center">
              <div className="text-center">
                <FolderOpen className="w-10 h-10 text-gray-300 dark:text-gray-700 mx-auto mb-3" />
                <p className="text-sm text-gray-400 dark:text-gray-500">打开图片文件夹开始标注</p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* 右：标注列表 */}
      <div className="w-60 shrink-0 bg-white dark:bg-surface-dark border-l border-gray-200 dark:border-gray-800 flex flex-col">
        <div className="p-4 border-b border-gray-200 dark:border-gray-800">
          <h3 className="text-xs font-semibold text-gray-900 dark:text-white mb-3">标注列表 ({boxes.length})</h3>
          <div className="space-y-1.5 max-h-64 overflow-auto">
            {boxes.map((box, i) => (
              <div
                key={box.id}
                className="flex items-center justify-between px-3 py-2 rounded-xl bg-gray-50 dark:bg-gray-900/50 text-xs group"
              >
                <span className="text-gray-600 dark:text-gray-300">
                  #{i + 1} 类别{box.classId}
                </span>
                <button
                  onClick={() => deleteBox(box.id)}
                  className="text-gray-300 hover:text-red-400 transition opacity-0 group-hover:opacity-100"
                >
                  <Trash2 className="w-3 h-3" />
                </button>
              </div>
            ))}
            {boxes.length === 0 && (
              <p className="text-xs text-gray-400 dark:text-gray-500 text-center py-4">暂无标注</p>
            )}
          </div>
        </div>

        <div className="p-4">
          <h3 className="text-xs font-semibold text-gray-900 dark:text-white mb-3">当前类别</h3>
          <div className="flex items-center gap-2">
            <input
              type="number"
              min={0}
              value={currentClass}
              onChange={(e) => setCurrentClass(Math.max(0, Number(e.target.value)))}
              className="w-16 text-xs px-2 py-1.5 rounded-xl border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50 text-gray-900 dark:text-white text-center focus:outline-none focus:border-brand-primary transition-colors"
            />
            <span className="text-xs text-gray-400 dark:text-gray-500">按类别 ID 标注</span>
          </div>
        </div>

        <div className="flex-1" />

        <div className="p-4 border-t border-gray-200 dark:border-gray-800">
          <button
            onClick={saveAnnotations}
            disabled={!hasChanges}
            className="w-full py-2 rounded-xl bg-gradient-to-r from-pink-500 to-rose-500 text-white text-xs font-medium hover:shadow-lg hover:shadow-pink-500/20 disabled:opacity-40 disabled:cursor-not-allowed transition-all flex items-center justify-center gap-2"
          >
            <Save className="w-3.5 h-3.5" />
            保存当前标注
          </button>
        </div>
      </div>
    </div>
  );
}
