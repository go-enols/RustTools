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
  Keyboard,
  X,
  RotateCcw,
  Focus,
  Wand2,
  Loader2,
} from "lucide-react";
import { useProject } from "../contexts/ProjectContext";
import ModelSelector from "../components/ModelSelector";

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

const TOOLS: { key: Tool; label: string; icon: React.ReactNode; shortcut: string }[] = [
  { key: "select", label: "选择 (V)", icon: <MousePointer className="w-3.5 h-3.5" />, shortcut: "1" },
  { key: "pan", label: "平移 (H)", icon: <Hand className="w-3.5 h-3.5" />, shortcut: "2" },
  { key: "rect", label: "矩形 (R)", icon: <Square className="w-3.5 h-3.5" />, shortcut: "3" },
];

const SHORTCUTS = [
  { key: "← / A", action: "上一张图片" },
  { key: "→ / D", action: "下一张图片" },
  { key: "1 / V", action: "选择工具" },
  { key: "2 / H", action: "平移工具" },
  { key: "3 / R", action: "矩形标注工具" },
  { key: "空格 (按住)", action: "临时平移" },
  { key: "滚轮", action: "缩放画布" },
  { key: "滚轮 + Shift", action: "快速缩放" },
  { key: "+ / -", action: "放大 / 缩小" },
  { key: "0 / F", action: "适应屏幕" },
  { key: "Delete / ⌫", action: "删除选中标注" },
  { key: "Ctrl + S", action: "保存标注" },
  { key: "Esc", action: "取消绘制" },
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
  const [selectedBoxId, setSelectedBoxId] = useState<number | null>(null);
  const [currentClass, setCurrentClass] = useState(0);
  const [tool, setTool] = useState<Tool>("rect");
  const [prevTool, setPrevTool] = useState<Tool>("rect");
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
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [isPanning, setIsPanning] = useState(false); // 空格临时平移
  const [showAutoAnnotate, setShowAutoAnnotate] = useState(false);
  const [autoAnnotating, setAutoAnnotating] = useState(false);
  const [confThreshold, setConfThreshold] = useState(0.25);
  const [selectedModelPath, setSelectedModelPath] = useState("");
  const [selectedModelName, setSelectedModelName] = useState("");
  const containerRef = useRef<HTMLDivElement>(null);
  const imgRef = useRef<HTMLImageElement>(null);
  const { project } = useProject();

  // 获取类别名称
  const getClassName = useCallback(
    (classId: number) => {
      if (project?.classes && classId >= 0 && classId < project.classes.length) {
        return project.classes[classId];
      }
      return `类别${classId}`;
    },
    [project]
  );

  // 计算适应屏幕的缩放比例
  const computeFitZoom = useCallback(() => {
    const container = containerRef.current;
    if (!container || imgSize.w === 0 || imgSize.h === 0) return 1;
    const rect = container.getBoundingClientRect();
    const padding = 40; // 留一些边距
    const scaleX = (rect.width - padding) / imgSize.w;
    const scaleY = (rect.height - padding) / imgSize.h;
    return Math.min(scaleX, scaleY, 3); // 最大初始放大 3 倍
  }, [imgSize]);

  // 适应屏幕
  const fitToScreen = useCallback(() => {
    const fit = computeFitZoom();
    setZoom(fit);
    setPan({ x: 0, y: 0 });
  }, [computeFitZoom]);

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
      setSelectedBoxId(null);
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
          setSelectedBoxId(null);
          return;
        }
      } catch { /* ignore */ }

      const fallbackDir = project.path + "/images";
      try {
        const list = await invoke<string[]>("list_images", { folder: fallbackDir, recursive: true });
        if (list.length > 0) {
          setImages(list);
          setFolderPath(fallbackDir);
          setSelectedIdx(0);
          setBoxes([]);
          setHasChanges(false);
          setSelectedBoxId(null);
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
      setSelectedBoxId(null);
    } catch (e) {
      console.error(e);
      setBoxes([]);
      setSelectedBoxId(null);
    }
  }, []);

  // 图片加载完成
  const handleImageLoad = useCallback(async () => {
    const img = imgRef.current;
    if (!img) return;
    const w = img.naturalWidth;
    const h = img.naturalHeight;
    setImgSize({ w, h });
    setPan({ x: 0, y: 0 });
    setSelectedBoxId(null);

    // 使用 requestAnimationFrame 确保容器已渲染
    requestAnimationFrame(() => {
      const container = containerRef.current;
      if (!container) return;
      const rect = container.getBoundingClientRect();
      const padding = 40;
      const scaleX = (rect.width - padding) / w;
      const scaleY = (rect.height - padding) / h;
      const fit = Math.min(scaleX, scaleY, 3);
      setZoom(Math.max(fit, 0.1));
    });

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
      setSelectedBoxId(null);
    }
  }, [selectedIdx, images]);

  // 将鼠标坐标转换为图片上的像素坐标（考虑缩放、平移和居中）
  const screenToPixel = useCallback(
    (sx: number, sy: number) => {
      const container = containerRef.current;
      if (!container || imgSize.w === 0 || imgSize.h === 0) return { x: 0, y: 0 };
      const rect = container.getBoundingClientRect();
      const centerX = rect.left + rect.width / 2;
      const centerY = rect.top + rect.height / 2;
      // 变换公式（逆运算）:
      // screen = center + zoom * (pixel - imgCenter) + pan
      // pixel = (screen - center - pan) / zoom + imgCenter
      const pixelX = (sx - centerX - pan.x) / zoom + imgSize.w / 2;
      const pixelY = (sy - centerY - pan.y) / zoom + imgSize.h / 2;
      return { x: pixelX, y: pixelY };
    },
    [zoom, pan, imgSize]
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      // 右键不处理
      if (e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();

      const activeTool = isPanning ? "pan" : tool;

      if (activeTool === "pan") {
        setDrawStart({ x: e.clientX - pan.x, y: e.clientY - pan.y });
        setDrawing(true);
        return;
      }
      if (activeTool !== "rect" || !imgRef.current) {
        // 选择工具：点击空白处取消选中
        if (activeTool === "select") {
          setSelectedBoxId(null);
        }
        return;
      }
      const p = screenToPixel(e.clientX, e.clientY);
      setDrawStart(p);
      setDrawCurrent(p);
      setDrawing(true);
      setSelectedBoxId(null);
    },
    [isPanning, tool, pan, screenToPixel, currentClass]
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      if (!drawing) return;
      e.preventDefault();
      e.stopPropagation();

      const activeTool = isPanning ? "pan" : tool;
      if (activeTool === "pan" && drawStart) {
        setPan({
          x: e.clientX - drawStart.x,
          y: e.clientY - drawStart.y,
        });
        return;
      }
      if (activeTool === "rect") {
        setDrawCurrent(screenToPixel(e.clientX, e.clientY));
      }
    },
    [drawing, isPanning, tool, drawStart, pan, screenToPixel]
  );

  const handleMouseUp = useCallback(() => {
    if (!drawing || !drawStart) return;
    const activeTool = isPanning ? "pan" : tool;

    if (activeTool === "rect" && drawCurrent) {
      const x = Math.min(drawStart.x, drawCurrent.x);
      const y = Math.min(drawStart.y, drawCurrent.y);
      const w = Math.abs(drawCurrent.x - drawStart.x);
      const h = Math.abs(drawCurrent.y - drawStart.y);
      if (w > 5 && h > 5) {
        const newBox: PixelBox = {
          id: Date.now(),
          classId: currentClass,
          x,
          y,
          w,
          h,
        };
        setBoxes((prev) => [...prev, newBox]);
        setSelectedBoxId(newBox.id);
        setHasChanges(true);
      }
    }

    setDrawing(false);
    setDrawStart(null);
    setDrawCurrent(null);
  }, [drawing, drawStart, isPanning, tool, drawCurrent, currentClass]);

  // 滚轮缩放（以鼠标位置为中心）
  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      e.preventDefault();
      const container = containerRef.current;
      if (!container || imgSize.w === 0) return;

      const rect = container.getBoundingClientRect();
      const centerX = rect.left + rect.width / 2;
      const centerY = rect.top + rect.height / 2;

      // 鼠标相对于容器中心的位置
      const mouseRelX = e.clientX - centerX;
      const mouseRelY = e.clientY - centerY;

      // 缩放因子
      const delta = e.deltaY;
      const factor = e.shiftKey ? 0.05 : 0.12;
      const scale = delta > 0 ? 1 - factor : 1 + factor;
      const newZoom = Math.max(0.05, Math.min(10, zoom * scale));

      if (newZoom === zoom) return;

      // 以鼠标为中心缩放：保持鼠标指向的图片像素坐标不变
      // mouseRel = zoom * (pixel - imgCenter) + pan
      // newPan = mouseRel - newZoom * (pixel - imgCenter)
      //        = mouseRel - newZoom * ((mouseRel - pan) / zoom)
      const newPanX = mouseRelX - newZoom * (mouseRelX - pan.x) / zoom;
      const newPanY = mouseRelY - newZoom * (mouseRelY - pan.y) / zoom;

      setZoom(newZoom);
      setPan({ x: newPanX, y: newPanY });
    },
    [zoom, pan, imgSize]
  );

  const deleteBox = (id: number) => {
    setBoxes((prev) => prev.filter((b) => b.id !== id));
    if (selectedBoxId === id) setSelectedBoxId(null);
    setHasChanges(true);
  };

  const deleteSelected = useCallback(() => {
    if (selectedBoxId !== null) {
      deleteBox(selectedBoxId);
    } else if (boxes.length > 0) {
      // 没有选中时删除最后一个
      deleteBox(boxes[boxes.length - 1].id);
    }
  }, [selectedBoxId, boxes]);

  const clearAll = () => {
    setBoxes([]);
    setSelectedBoxId(null);
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

  // 自动标注
  const runAutoAnnotate = async () => {
    const currentImage = images[selectedIdx];
    if (!selectedModelPath || !currentImage || imgSize.w === 0) return;
    setAutoAnnotating(true);
    try {
      const anns = await invoke<YoloAnnotation[]>("auto_annotate_image", {
        modelPath: selectedModelPath,
        imagePath: currentImage,
        confThreshold: confThreshold,
      });
      const newBoxes = anns.map((a) => ({
        ...yoloToPixel(a, imgSize.w, imgSize.h),
        id: Date.now() + Math.random(),
      }));
      setBoxes((prev) => [...prev, ...newBoxes]);
      setHasChanges(true);
      setShowAutoAnnotate(false);
    } catch (e: any) {
      console.error(e);
      alert(typeof e === "string" ? e : e.message || "自动标注失败");
    } finally {
      setAutoAnnotating(false);
    }
  };

  // 切换到下一张/上一张
  const goNext = useCallback(() => {
    if (images.length === 0) return;
    // 自动保存当前标注
    if (hasChanges) {
      saveAnnotations();
    }
    setSelectedIdx((i) => Math.min(images.length - 1, i + 1));
  }, [images.length, hasChanges]);

  const goPrev = useCallback(() => {
    if (images.length === 0) return;
    if (hasChanges) {
      saveAnnotations();
    }
    setSelectedIdx((i) => Math.max(0, i - 1));
  }, [images.length, hasChanges]);

  // 键盘快捷键
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // 忽略输入框中的快捷键
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable) {
        // 但允许 Ctrl+S 和 Escape
        if (!(e.key === "Escape" || (e.key === "s" && (e.ctrlKey || e.metaKey)))) {
          return;
        }
      }

      if (e.key === "Escape") {
        if (drawing) {
          setDrawing(false);
          setDrawStart(null);
          setDrawCurrent(null);
        } else if (showShortcuts) {
          setShowShortcuts(false);
        } else {
          setSelectedBoxId(null);
        }
        return;
      }

      // 空格：临时平移
      if (e.key === " " && !e.repeat) {
        e.preventDefault();
        if (!isPanning) {
          setIsPanning(true);
          setPrevTool(tool);
        }
        return;
      }

      if (e.key === "ArrowLeft" || e.key === "a" || e.key === "A") {
        e.preventDefault();
        goPrev();
        return;
      }
      if (e.key === "ArrowRight" || e.key === "d" || e.key === "D") {
        e.preventDefault();
        goNext();
        return;
      }

      if (e.key === "1" || e.key === "v" || e.key === "V") {
        setTool("select");
        return;
      }
      if (e.key === "2" || e.key === "h" || e.key === "H") {
        setTool("pan");
        return;
      }
      if (e.key === "3" || e.key === "r" || e.key === "R") {
        setTool("rect");
        return;
      }

      if (e.key === "0" || e.key === "f" || e.key === "F") {
        e.preventDefault();
        fitToScreen();
        return;
      }

      if (e.key === "+" || e.key === "=") {
        e.preventDefault();
        setZoom((z) => Math.min(10, z * 1.2));
        return;
      }
      if (e.key === "-" || e.key === "_") {
        e.preventDefault();
        setZoom((z) => Math.max(0.05, z / 1.2));
        return;
      }

      if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        deleteSelected();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault();
        saveAnnotations();
        return;
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      if (e.key === " " && isPanning) {
        setIsPanning(false);
        setTool(prevTool);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
    };
  }, [drawing, showShortcuts, isPanning, tool, prevTool, goPrev, goNext, fitToScreen, deleteSelected, hasChanges, images, selectedIdx, boxes, imgSize]);

  const currentImage = images[selectedIdx];

  // 绘制中的临时框
  const tempBox =
    drawing && tool === "rect" && !isPanning && drawStart && drawCurrent
      ? {
          x: Math.min(drawStart.x, drawCurrent.x),
          y: Math.min(drawStart.y, drawCurrent.y),
          w: Math.abs(drawCurrent.x - drawStart.x),
          h: Math.abs(drawCurrent.y - drawCurrent.y),
        }
      : null;

  const activeTool = isPanning ? "pan" : tool;

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
              onClick={() => {
                if (hasChanges) saveAnnotations();
                setSelectedIdx(i);
              }}
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
            <button
              onClick={goPrev}
              className="p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded"
              title="上一张 (←)"
            >
              <ChevronLeft className="w-3 h-3" />
            </button>
            <span>
              {selectedIdx + 1} / {images.length}
            </span>
            <button
              onClick={goNext}
              className="p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded"
              title="下一张 (→)"
            >
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
              className={`p-2 rounded-xl transition-all relative ${
                activeTool === t.key
                  ? "bg-brand-primary/10 text-brand-primary"
                  : "text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
              }`}
            >
              {t.icon}
              <span className="absolute -bottom-0.5 -right-0.5 text-[8px] font-medium text-gray-400 dark:text-gray-500 bg-white dark:bg-gray-800 rounded px-0.5 border border-gray-100 dark:border-gray-700">
                {t.shortcut}
              </span>
            </button>
          ))}

          <div className="w-px h-5 bg-gray-200 dark:bg-gray-700 mx-2" />

          <button
            onClick={() => setZoom((z) => Math.max(0.05, z / 1.2))}
            className="p-2 rounded-xl text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition"
            title="缩小 (-)"
          >
            <ZoomOut className="w-3.5 h-3.5" />
          </button>
          <span className="text-[10px] text-gray-400 dark:text-gray-500 w-12 text-center">
            {Math.round(zoom * 100)}%
          </span>
          <button
            onClick={() => setZoom((z) => Math.min(10, z * 1.2))}
            className="p-2 rounded-xl text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition"
            title="放大 (+)"
          >
            <ZoomIn className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={fitToScreen}
            className="p-2 rounded-xl text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition"
            title="适应屏幕 (0/F)"
          >
            <Focus className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={() => { setZoom(1); setPan({ x: 0, y: 0 }); }}
            className="p-2 rounded-xl text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition"
            title="重置视图"
          >
            <RotateCcw className="w-3.5 h-3.5" />
          </button>

          <div className="flex-1" />

          {hasChanges && (
            <span className="text-[10px] text-amber-500 mr-2">有未保存的更改</span>
          )}

          <button
            onClick={() => setShowShortcuts(true)}
            className="p-2 rounded-xl text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition"
            title="快捷键说明"
          >
            <Keyboard className="w-3.5 h-3.5" />
          </button>

          <button
            onClick={() => setShowAutoAnnotate(true)}
            disabled={!images.length}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs text-purple-600 dark:text-purple-400 hover:bg-purple-50 dark:hover:bg-purple-900/20 transition disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <Wand2 className="w-3 h-3" />
            自动标注
          </button>
          <button
            onClick={clearAll}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs text-red-500 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition"
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
          className="flex-1 relative overflow-hidden"
          style={{ cursor: activeTool === "pan" || isPanning ? "grab" : activeTool === "rect" ? "crosshair" : "default" }}
          onMouseDown={handleMouseDown}
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
          onMouseLeave={handleMouseUp}
          onWheel={handleWheel}
        >
          {currentImage ? (
            <div
              className="absolute top-1/2 left-1/2"
              style={{
                width: imgSize.w > 0 ? imgSize.w : undefined,
                height: imgSize.h > 0 ? imgSize.h : undefined,
                transformOrigin: "center center",
                transform: `translate(-50%, -50%) translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
              }}
            >
              <img
                ref={imgRef}
                src={pathToSrc(currentImage)}
                alt="annotation"
                className="block select-none"
                draggable={false}
                onDragStart={(e) => e.preventDefault()}
                onLoad={handleImageLoad}
                onError={(e) => {
                  console.error("Image load failed:", currentImage, e);
                }}
                style={{
                  width: imgSize.w > 0 ? imgSize.w : undefined,
                  height: imgSize.h > 0 ? imgSize.h : undefined,
                }}
              />

              {/* 标注框 overlay - 与图片完全重合 */}
              {imgSize.w > 0 && (
                <div
                  className="absolute top-0 left-0 pointer-events-none"
                  style={{
                    width: imgSize.w,
                    height: imgSize.h,
                  }}
                >
                  {boxes.map((box) => (
                    <div
                      key={box.id}
                      className={`absolute pointer-events-auto group transition-colors ${
                        selectedBoxId === box.id
                          ? "border-2 border-blue-500"
                          : "border-2 border-pink-500/80"
                      }`}
                      style={{
                        left: `${box.x}px`,
                        top: `${box.y}px`,
                        width: `${box.w}px`,
                        height: `${box.h}px`,
                      }}
                      onClick={(e) => {
                        e.stopPropagation();
                        setSelectedBoxId(box.id);
                      }}
                    >
                      <span
                        className={`absolute -top-4 left-0 text-white text-[9px] px-1 py-0.5 rounded font-medium whitespace-nowrap ${
                          selectedBoxId === box.id ? "bg-blue-500" : "bg-pink-500"
                        }`}
                      >
                        {getClassName(box.classId)}
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
                onClick={() => setSelectedBoxId(box.id)}
                className={`flex items-center justify-between px-3 py-2 rounded-xl text-xs group cursor-pointer transition-colors ${
                  selectedBoxId === box.id
                    ? "bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800"
                    : "bg-gray-50 dark:bg-gray-900/50 hover:bg-gray-100 dark:hover:bg-gray-800"
                }`}
              >
                <span className="text-gray-600 dark:text-gray-300">
                  #{i + 1} {getClassName(box.classId)}
                </span>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    deleteBox(box.id);
                  }}
                  className="text-gray-300 dark:text-gray-600 hover:text-red-400 dark:hover:text-red-400 transition opacity-0 group-hover:opacity-100"
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
            <span className="text-xs text-brand-primary font-medium truncate max-w-[120px]">
              {getClassName(currentClass)}
            </span>
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

      {/* 快捷键说明弹窗 */}
      {showShortcuts && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
          <div className="bg-white dark:bg-gray-900 rounded-2xl shadow-2xl w-80 max-w-[90vw] overflow-hidden">
            <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-800">
              <h3 className="text-sm font-semibold text-gray-900 dark:text-white flex items-center gap-2">
                <Keyboard className="w-4 h-4" />
                快捷键说明
              </h3>
              <button
                onClick={() => setShowShortcuts(false)}
                className="p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-400 transition"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="p-4 max-h-[60vh] overflow-auto">
              <div className="space-y-2">
                {SHORTCUTS.map((s) => (
                  <div key={s.key} className="flex items-center justify-between text-xs">
                    <kbd className="px-2 py-1 rounded-lg bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 font-mono border border-gray-200 dark:border-gray-700">
                      {s.key}
                    </kbd>
                    <span className="text-gray-500 dark:text-gray-400">{s.action}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* 自动标注弹窗 */}
      {showAutoAnnotate && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
          <div className="bg-white dark:bg-gray-900 rounded-2xl shadow-2xl w-96 max-w-[90vw] overflow-hidden">
            <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-800">
              <h3 className="text-sm font-semibold text-gray-900 dark:text-white flex items-center gap-2">
                <Wand2 className="w-4 h-4 text-purple-500" />
                自动标注
              </h3>
              <button
                onClick={() => setShowAutoAnnotate(false)}
                className="p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-400 transition"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="p-4 space-y-4">
              <div>
                <label className="text-xs text-gray-500 dark:text-gray-400 mb-1.5 block">选择模型</label>
                <ModelSelector
                  compact
                  ext="onnx,pt"
                  loadOnSelect={false}
                  autoLoad={false}
                  onSelect={(name: string, path: string) => {
                    setSelectedModelName(name);
                    setSelectedModelPath(path);
                  }}
                />
              </div>
              <div>
                <label className="text-xs text-gray-500 dark:text-gray-400 mb-1.5 block">
                  置信度阈值: {confThreshold.toFixed(2)}
                </label>
                <input
                  type="range"
                  min={0.05}
                  max={0.95}
                  step={0.05}
                  value={confThreshold}
                  onChange={(e) => setConfThreshold(Number(e.target.value))}
                  className="w-full h-1.5 bg-gray-200 dark:bg-gray-700 rounded-lg appearance-none cursor-pointer accent-purple-500"
                />
                <div className="flex justify-between text-[10px] text-gray-400 dark:text-gray-500 mt-1">
                  <span>低 (更多框)</span>
                  <span>高 (更准)</span>
                </div>
              </div>
              {selectedModelPath && (
                <div className="text-[10px] text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-gray-800/50 px-2 py-1.5 rounded-lg truncate">
                  已选: {selectedModelName || selectedModelPath}
                </div>
              )}
              <button
                onClick={runAutoAnnotate}
                disabled={!selectedModelPath || autoAnnotating || !images.length}
                className="w-full py-2 rounded-xl bg-gradient-to-r from-purple-500 to-violet-500 text-white text-xs font-medium hover:shadow-lg hover:shadow-purple-500/20 disabled:opacity-40 disabled:cursor-not-allowed transition-all flex items-center justify-center gap-2"
              >
                {autoAnnotating ? (
                  <>
                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                    推理中...
                  </>
                ) : (
                  <>
                    <Wand2 className="w-3.5 h-3.5" />
                    运行自动标注
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
