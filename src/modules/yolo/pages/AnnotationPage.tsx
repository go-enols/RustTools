import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import {
  MousePointer2,
  Square,
  ChevronLeft,
  ChevronRight,
  ZoomIn,
  ZoomOut,
  RotateCw,
  Trash2,
  Plus,
  Edit2,
  FolderOpen,
  Image as ImageIcon,
  Save,
  X,
  Check,
} from 'lucide-react';
import { useWorkspaceStore } from '../../../core/stores/workspaceStore';
import { listDirectory, readBinaryFile } from '../../../core/api/file';
import { updateClasses, loadAnnotation, saveAnnotation, YoloAnnotation } from '../../../core/api/annotation';

type Tool = 'select' | 'draw';

interface AnnotationItem {
  id: string;
  label: string;
  classId: number;
  x: number;
  y: number;
  width: number;
  height: number;
}

interface ImageFile {
  name: string;
  path: string;
}

interface DrawState {
  isDrawing: boolean;
  startX: number;
  startY: number;
  currentX: number;
  currentY: number;
}

export default function AnnotationPage() {
  const { currentProject, openProject } = useWorkspaceStore();
  const [tool, setTool] = useState<Tool>('select');
  const [currentImage, setCurrentImage] = useState(0);
  const [images, setImages] = useState<ImageFile[]>([]);
  const [annotations, setAnnotations] = useState<AnnotationItem[]>([]);
  const [selectedAnnotation, setSelectedAnnotation] = useState<string | null>(null);
  const [selectedClassId, setSelectedClassId] = useState<number>(0);
  const [zoom, setZoom] = useState(100);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [currentImageData, setCurrentImageData] = useState<string | null>(null);
  const [imageLoading, setImageLoading] = useState(false);
  const [imageDimensions, setImageDimensions] = useState<{ width: number; height: number } | null>(null);
  const [containerSize, setContainerSize] = useState<{ width: number; height: number } | null>(null);

  // Image drag state
  const [imagePosition, setImagePosition] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const dragStartRef = useRef({ x: 0, y: 0, imgX: 0, imgY: 0 });

  // Draw state
  const [drawState, setDrawState] = useState<DrawState>({
    isDrawing: false,
    startX: 0,
    startY: 0,
    currentX: 0,
    currentY: 0,
  });

  const containerRef = useRef<HTMLDivElement>(null);
  const imageRef = useRef<HTMLImageElement>(null);

  // Classes from project config
  const classes = useMemo(() => currentProject?.classes.map((name, idx) => ({
    id: idx,
    name,
    color: getClassColor(idx),
  })) || [], [currentProject?.classes]);

  // Calculate display transform based on container, image size and zoom
  const displayTransform = useMemo(() => {
    if (!imageDimensions || !containerSize) {
      return { scale: 1, offsetX: 0, offsetY: 0 };
    }
    const fillScale = Math.min(
      containerSize.width / imageDimensions.width,
      containerSize.height / imageDimensions.height
    );
    const scale = fillScale * (zoom / 100);
    const scaledWidth = imageDimensions.width * scale;
    const scaledHeight = imageDimensions.height * scale;
    const offsetX = (containerSize.width - scaledWidth) / 2 + imagePosition.x;
    const offsetY = (containerSize.height - scaledHeight) / 2 + imagePosition.y;
    return { scale, offsetX, offsetY };
  }, [imageDimensions, containerSize, zoom, imagePosition]);

  // Generate consistent color for class
  function getClassColor(classId: number): string {
    const colors = [
      '#ff6b6b', '#4ecdc4', '#45b7d1', '#96ceb4', '#ffeaa7',
      '#dfe6e9', '#fd79a8', '#a29bfe', '#00b894', '#e17055',
    ];
    return colors[classId % colors.length];
  }

  // Load label file path for current image
  const getLabelPath = useCallback((imagePath: string): string => {
    if (!currentProject) return '';
    // Convert images/train/image.jpg to labels/train/image.txt
    const relativePath = imagePath.replace(currentProject.path, '').replace(/\\/g, '/');
    const labelPath = relativePath.replace('/images/', '/labels/').replace(/\.(jpg|jpeg|png)$/i, '.txt');
    return `${currentProject.path}${labelPath}`;
  }, [currentProject]);

  // Auto-load images from project config on mount
  useEffect(() => {
    const loadImages = async () => {
      if (!currentProject) return;

      setIsLoading(true);
      setError(null);

      const trainPath = `${currentProject.path}/${currentProject.images.train}`;
      const valPath = `${currentProject.path}/${currentProject.images.val}`;

      const result = await listDirectory(trainPath);

      if (!result.success || !result.data) {
        const valResult = await listDirectory(valPath);
        if (!valResult.success || !valResult.data) {
          setError(`加载失败: ${result.error || valResult.error || '目录不存在'}`);
          setIsLoading(false);
          return;
        }
        const valImages = valResult.data.filter((f) => !f.is_dir && /\.(jpg|jpeg|png)$/i.test(f.name));
        setImages(valImages.map((f) => ({ name: f.name, path: f.path })));
        setIsLoading(false);
        return;
      }

      const imageFiles = result.data.filter((f) => !f.is_dir && /\.(jpg|jpeg|png)$/i.test(f.name));
      setImages(imageFiles.map((f) => ({ name: f.name, path: f.path })));
      setIsLoading(false);
    };

    loadImages();
  }, [currentProject]);

  // Load image data and annotations when current image changes
  useEffect(() => {
    const loadImageData = async () => {
      if (images.length === 0 || !images[currentImage]) {
        setCurrentImageData(null);
        setAnnotations([]);
        return;
      }

      setImageLoading(true);
      const img = images[currentImage];
      const result = await readBinaryFile(img.path);

      if (result.success && result.data) {
        const ext = img.name.toLowerCase().split('.').pop();
        const mimeType = ext === 'png' ? 'image/png' : ext === 'jpg' || ext === 'jpeg' ? 'image/jpeg' : 'image/gif';
        const dataUrl = `data:${mimeType};base64,${result.data}`;
        setCurrentImageData(dataUrl);

        const imgEl = new window.Image();
        imgEl.onload = () => {
          setImageDimensions({ width: imgEl.naturalWidth, height: imgEl.naturalHeight });
        };
        imgEl.src = dataUrl;

        // Load existing annotations
        const labelPath = getLabelPath(img.path);
        const labelResult = await loadAnnotation(labelPath);
        if (labelResult.success && labelResult.data) {
          const loadedAnnotations: AnnotationItem[] = labelResult.data.map((ann, idx) => ({
            id: `ann-${Date.now()}-${idx}`,
            label: classes[ann.class_id]?.name || `Class ${ann.class_id}`,
            classId: ann.class_id,
            x: (ann.x_center - ann.width / 2) * imgEl.naturalWidth,
            y: (ann.y_center - ann.height / 2) * imgEl.naturalHeight,
            width: ann.width * imgEl.naturalWidth,
            height: ann.height * imgEl.naturalHeight,
          }));
          setAnnotations(loadedAnnotations);
        }
      } else {
        setCurrentImageData(null);
        setImageDimensions(null);
        setAnnotations([]);
      }
      setImageLoading(false);
      setImagePosition({ x: 0, y: 0 });
    };

    loadImageData();
  }, [currentImage, images, currentProject, classes, getLabelPath]);

  // Reset currentImage when images changes and currentImage is out of bounds
  useEffect(() => {
    if (currentImage >= images.length && images.length > 0) {
      setCurrentImage(images.length - 1);
    } else if (images.length === 0) {
      setCurrentImage(0);
    }
  }, [images, currentImage]);

  // Save annotations when switching images - receives annotations as parameter to avoid stale closure
  const saveCurrentAnnotations = useCallback(async (annotationsToSave: AnnotationItem[]) => {
    if (!currentProject || !imageDimensions || images.length === 0 || !images[currentImage]) return;

    const labelPath = getLabelPath(images[currentImage].path);
    const yoloAnnotations: YoloAnnotation[] = annotationsToSave.map(ann => ({
      class_id: ann.classId,
      x_center: (ann.x + ann.width / 2) / imageDimensions.width,
      y_center: (ann.y + ann.height / 2) / imageDimensions.height,
      width: ann.width / imageDimensions.width,
      height: ann.height / imageDimensions.height,
    }));

    await saveAnnotation(labelPath, yoloAnnotations);
  }, [currentProject, imageDimensions, images, currentImage, getLabelPath]);

  // Save when switching images - save BEFORE clearing annotations
  useEffect(() => {
    return () => {
      // Save with current annotations from the state at this moment
      saveCurrentAnnotations(annotations);
    };
  }, [currentImage, annotations, saveCurrentAnnotations]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }

      switch (e.key.toLowerCase()) {
        case 'q':
          setTool('select');
          break;
        case 'w':
          setTool('draw');
          break;
        case 'a':
          if (currentImage > 0) {
            saveCurrentAnnotations(annotations);
            setCurrentImage(prev => prev - 1);
          }
          break;
        case 'd':
          if (currentImage < images.length - 1) {
            saveCurrentAnnotations(annotations);
            setCurrentImage(prev => prev + 1);
          }
          break;
        case 'delete':
        case 'backspace':
          if (selectedAnnotation) {
            setAnnotations(prev => prev.filter(ann => ann.id !== selectedAnnotation));
            setSelectedAnnotation(null);
          }
          break;
        case 'escape':
          setSelectedAnnotation(null);
          setDrawState({ isDrawing: false, startX: 0, startY: 0, currentX: 0, currentY: 0 });
          break;
        case 's':
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            saveCurrentAnnotations(annotations);
          }
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [currentImage, images.length, selectedAnnotation, saveCurrentAnnotations]);

  // Mouse wheel zoom
  useEffect(() => {
    if (!currentImageData) return;

    const container = containerRef.current;
    if (!container) return;

    const handleWheel = (e: WheelEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }
      e.preventDefault();
      const delta = e.deltaY > 0 ? -10 : 10;
      setZoom(prev => Math.min(Math.max(prev + delta, 25), 400));
    };

    container.addEventListener('wheel', handleWheel, { passive: false });
    return () => container.removeEventListener('wheel', handleWheel);
  }, [currentImageData]);

  // Track container size
  useEffect(() => {
    if (!currentImageData) return;

    const container = containerRef.current;
    if (!container) return;

    let rafId: number;
    const updateSize = () => {
      rafId = requestAnimationFrame(() => {
        const width = container.clientWidth;
        const height = container.clientHeight;
        if (width > 0 && height > 0) {
          setContainerSize({ width, height });
        }
      });
    };

    updateSize();
    const observer = new ResizeObserver(updateSize);
    observer.observe(container);
    return () => {
      cancelAnimationFrame(rafId);
      observer.disconnect();
    };
  }, [currentImageData]);

  // Handle mouse events for drag and draw
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (tool === 'select') {
      // Start dragging
      setIsDragging(true);
      dragStartRef.current = {
        x: e.clientX,
        y: e.clientY,
        imgX: imagePosition.x,
        imgY: imagePosition.y,
      };
    } else if (tool === 'draw' && imageDimensions && containerSize) {
      const rect = containerRef.current?.getBoundingClientRect();
      if (!rect) return;

      const mouseX = (e.clientX - rect.left - displayTransform.offsetX) / displayTransform.scale;
      const mouseY = (e.clientY - rect.top - displayTransform.offsetY) / displayTransform.scale;

      setDrawState({
        isDrawing: true,
        startX: mouseX,
        startY: mouseY,
        currentX: mouseX,
        currentY: mouseY,
      });
    }
  }, [tool, imagePosition, imageDimensions, containerSize, displayTransform]);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (isDragging && tool === 'select') {
      const deltaX = e.clientX - dragStartRef.current.x;
      const deltaY = e.clientY - dragStartRef.current.y;
      setImagePosition({
        x: dragStartRef.current.imgX + deltaX,
        y: dragStartRef.current.imgY + deltaY,
      });
    } else if (drawState.isDrawing && tool === 'draw' && imageDimensions && containerSize) {
      const rect = containerRef.current?.getBoundingClientRect();
      if (!rect) return;

      const mouseX = (e.clientX - rect.left - displayTransform.offsetX) / displayTransform.scale;
      const mouseY = (e.clientY - rect.top - displayTransform.offsetY) / displayTransform.scale;

      setDrawState(prev => ({
        ...prev,
        currentX: mouseX,
        currentY: mouseY,
      }));
    }
  }, [isDragging, drawState.isDrawing, tool, imageDimensions, containerSize, displayTransform]);

  const handleMouseUp = useCallback(() => {
    if (isDragging) {
      setIsDragging(false);
    }
    if (drawState.isDrawing && tool === 'draw') {
      const width = Math.abs(drawState.currentX - drawState.startX);
      const height = Math.abs(drawState.currentY - drawState.startY);

      if (width > 10 && height > 10 && classes.length > 0) {
        const x = Math.min(drawState.startX, drawState.currentX);
        const y = Math.min(drawState.startY, drawState.currentY);

        const newAnnotation: AnnotationItem = {
          id: `ann-${Date.now()}`,
          label: classes[selectedClassId]?.name || 'Unknown',
          classId: selectedClassId,
          x,
          y,
          width,
          height,
        };

        setAnnotations(prev => [...prev, newAnnotation]);
      }

      setDrawState({
        isDrawing: false,
        startX: 0,
        startY: 0,
        currentX: 0,
        currentY: 0,
      });
    }
  }, [isDragging, drawState, tool, classes, selectedClassId]);

  const handleDeleteSelected = useCallback(() => {
    if (selectedAnnotation) {
      setAnnotations(prev => prev.filter(ann => ann.id !== selectedAnnotation));
      setSelectedAnnotation(null);
    }
  }, [selectedAnnotation]);

  // Get current drawing rect for display
  const getDrawRect = () => {
    if (!drawState.isDrawing) return null;
    const x = Math.min(drawState.startX, drawState.currentX);
    const y = Math.min(drawState.startY, drawState.currentY);
    const width = Math.abs(drawState.currentX - drawState.startX);
    const height = Math.abs(drawState.currentY - drawState.startY);
    return { x, y, width, height };
  };

  // Class management
  const [editingClassId, setEditingClassId] = useState<number | null>(null);
  const [editingClassName, setEditingClassName] = useState('');
  const [newClassName, setNewClassName] = useState('');

  const handleAddClass = async () => {
    if (!currentProject || !newClassName.trim()) return;

    const newClasses = [...currentProject.classes, newClassName.trim()];
    const result = await updateClasses(currentProject.path, newClasses);

    if (result.success) {
      openProject({
        ...currentProject!,
        classes: newClasses,
      });
      setNewClassName('');
    }
  };

  const handleUpdateClass = async (classId: number, newName: string) => {
    if (!currentProject || !newName.trim()) return;

    const newClasses = [...currentProject.classes];
    newClasses[classId] = newName.trim();
    const result = await updateClasses(currentProject.path, newClasses);

    if (result.success) {
      openProject({
        ...currentProject!,
        classes: newClasses,
      });
      setEditingClassId(null);
      setEditingClassName('');
    }
  };

  const handleDeleteClass = async (classId: number) => {
    if (!currentProject || currentProject.classes.length <= 1) return;

    const newClasses = currentProject.classes.filter((_, idx) => idx !== classId);
    const result = await updateClasses(currentProject.path, newClasses);

    if (result.success) {
      openProject({
        ...currentProject!,
        classes: newClasses,
      });
      if (selectedClassId >= classId && selectedClassId > 0) {
        setSelectedClassId(selectedClassId - 1);
      }
    }
  };

  return (
    <div className="annotation-canvas" style={{ height: '100%' }}>
      {/* Left Toolbar */}
      <div className="annotation-toolbar">
        <div
          className={`annotation-tool-btn ${tool === 'select' ? 'active' : ''}`}
          onClick={() => setTool('select')}
          title="选择/拖动 (Q)"
        >
          <MousePointer2 size={20} />
        </div>
        <div
          className={`annotation-tool-btn ${tool === 'draw' ? 'active' : ''}`}
          onClick={() => setTool('draw')}
          title="绘制标注 (W)"
        >
          <Square size={20} />
        </div>
        <div style={{ flex: 1 }} />
        <div className="annotation-tool-btn" onClick={() => { saveCurrentAnnotations(annotations); }} title="保存 (Ctrl+S)">
          <Save size={20} />
        </div>
        <div className="annotation-tool-btn" onClick={() => currentImage > 0 && setCurrentImage(prev => prev - 1)} title="上一张 (A)">
          <ChevronLeft size={20} />
        </div>
        <div className="annotation-tool-btn" onClick={() => currentImage < images.length - 1 && setCurrentImage(prev => prev + 1)} title="下一张 (D)">
          <ChevronRight size={20} />
        </div>
        <div className="annotation-tool-btn" onClick={() => setZoom(prev => Math.min(prev + 25, 400))} title="放大">
          <ZoomIn size={20} />
        </div>
        <div className="annotation-tool-btn" onClick={() => setZoom(prev => Math.max(prev - 25, 25))} title="缩小">
          <ZoomOut size={20} />
        </div>
        <div className="annotation-tool-btn" title="旋转">
          <RotateCw size={20} />
        </div>
        <div className="annotation-tool-btn" title="删除选中" onClick={handleDeleteSelected}>
          <Trash2 size={20} />
        </div>
      </div>

      <div className="annotation-workspace">
        {/* Header */}
        <div className="annotation-header">
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 'var(--spacing-sm)' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-lg)' }}>
              <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>
                进度: {currentImage + 1}/{images.length} ({images.length > 0 ? (((currentImage + 1) / images.length) * 100).toFixed(1) : 0}%)
              </span>
              <span style={{ fontSize: 13, color: 'var(--text-tertiary)' }}>
                标注: {annotations.length}
              </span>
              <span style={{ fontSize: 13, color: tool === 'draw' ? 'var(--accent-primary)' : 'var(--text-tertiary)' }}>
                模式: {tool === 'select' ? '拖动' : '绘制'}
              </span>
            </div>
            {/* Keyboard Shortcuts */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-md)', padding: 'var(--spacing-xs) var(--spacing-sm)', background: 'var(--bg-elevated)', borderRadius: 'var(--radius-sm)' }}>
              <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>快捷键:</span>
              <div style={{ display: 'flex', gap: 'var(--spacing-md)' }}>
                <span style={{ fontSize: 11, display: 'flex', alignItems: 'center', gap: 4 }}><kbd style={{ padding: '1px 5px', background: 'var(--bg-surface)', border: '1px solid var(--border-default)', borderRadius: 3, fontSize: 10 }}>Q</kbd> 拖动</span>
                <span style={{ fontSize: 11, display: 'flex', alignItems: 'center', gap: 4 }}><kbd style={{ padding: '1px 5px', background: 'var(--bg-surface)', border: '1px solid var(--border-default)', borderRadius: 3, fontSize: 10 }}>W</kbd> 绘制</span>
                <span style={{ fontSize: 11, display: 'flex', alignItems: 'center', gap: 4 }}><kbd style={{ padding: '1px 5px', background: 'var(--bg-surface)', border: '1px solid var(--border-default)', borderRadius: 3, fontSize: 10 }}>A</kbd> 前一张</span>
                <span style={{ fontSize: 11, display: 'flex', alignItems: 'center', gap: 4 }}><kbd style={{ padding: '1px 5px', background: 'var(--bg-surface)', border: '1px solid var(--border-default)', borderRadius: 3, fontSize: 10 }}>D</kbd> 下一张</span>
                <span style={{ fontSize: 11, display: 'flex', alignItems: 'center', gap: 4 }}><kbd style={{ padding: '1px 5px', background: 'var(--bg-surface)', border: '1px solid var(--border-default)', borderRadius: 3, fontSize: 10 }}>Ctrl+S</kbd> 保存</span>
              </div>
            </div>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-md)' }}>
            {currentProject && (
              <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                数据集: {currentProject.images.train}
              </span>
            )}
          </div>
        </div>

        <div className="annotation-content">
          {/* Left Panel - Annotation List */}
          <div className="annotation-sidebar-left">
            <div>
              <h4 style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 'var(--spacing-md)' }}>
                标注列表
              </h4>
              {annotations.length === 0 ? (
                <p style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>暂无标注 (按W绘制)</p>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-sm)' }}>
                  {annotations.map((ann) => (
                    <div
                      key={ann.id}
                      onClick={() => setSelectedAnnotation(ann.id)}
                      style={{
                        padding: 'var(--spacing-sm)',
                        background: selectedAnnotation === ann.id ? 'var(--bg-active)' : 'var(--bg-elevated)',
                        borderRadius: 'var(--radius-sm)',
                        fontSize: 12,
                        cursor: 'pointer',
                        borderLeft: `3px solid ${getClassColor(ann.classId)}`,
                      }}
                    >
                      <span style={{ color: getClassColor(ann.classId), fontWeight: 500 }}>{ann.label}</span>
                      <span style={{ color: 'var(--text-tertiary)', marginLeft: 8, fontSize: 11 }}>
                        {Math.round(ann.x)}, {Math.round(ann.y)} {Math.round(ann.width)}x{Math.round(ann.height)}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* Main Canvas */}
          <div className="annotation-main">
            {isLoading ? (
              <div style={{ width: '80%', height: '80%', background: 'var(--bg-surface)', border: '2px dashed var(--border-default)', borderRadius: 'var(--radius-lg)', display: 'flex', alignItems: 'center', justifyContent: 'center', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
                <div style={{ width: 40, height: 40, border: '3px solid var(--border-default)', borderTopColor: 'var(--accent-primary)', borderRadius: '50%', animation: 'spin 1s linear infinite' }} />
                <p style={{ color: 'var(--text-tertiary)', fontSize: 14 }}>加载图片中...</p>
                <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
              </div>
            ) : error ? (
              <div style={{ width: '80%', height: '80%', background: 'var(--bg-surface)', border: '2px dashed var(--border-default)', borderRadius: 'var(--radius-lg)', display: 'flex', alignItems: 'center', justifyContent: 'center', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
                <FolderOpen size={48} style={{ color: 'var(--status-error)' }} />
                <p style={{ color: 'var(--status-error)', fontSize: 14 }}>{error}</p>
                <p style={{ color: 'var(--text-tertiary)', fontSize: 12 }}>请确保项目包含 images/train 或 images/val 文件夹</p>
              </div>
            ) : images.length === 0 ? (
              <div style={{ width: '80%', height: '80%', background: 'var(--bg-surface)', border: '2px dashed var(--border-default)', borderRadius: 'var(--radius-lg)', display: 'flex', alignItems: 'center', justifyContent: 'center', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
                <ImageIcon size={48} style={{ color: 'var(--text-tertiary)' }} />
                <p style={{ color: 'var(--text-tertiary)', fontSize: 14 }}>未找到图片文件</p>
                <p style={{ color: 'var(--text-tertiary)', fontSize: 12 }}>请将图片放入 images/train 文件夹</p>
              </div>
            ) : (
              <div
                ref={containerRef}
                style={{
                  width: '100%',
                  height: '100%',
                  overflow: 'hidden',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  cursor: tool === 'select' ? (isDragging ? 'grabbing' : 'grab') : 'crosshair',
                }}
                onMouseDown={handleMouseDown}
                onMouseMove={handleMouseMove}
                onMouseUp={handleMouseUp}
                onMouseLeave={handleMouseUp}
              >
                {imageLoading ? (
                  <>
                    <div style={{ width: 40, height: 40, border: '3px solid var(--border-default)', borderTopColor: 'var(--accent-primary)', borderRadius: '50%', animation: 'spin 1s linear infinite' }} />
                    <p style={{ fontSize: 14 }}>加载图片...</p>
                  </>
                ) : currentImageData ? (
                  <div style={{ position: 'relative', width: '100%', height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                    <img
                      ref={imageRef}
                      src={currentImageData}
                      alt={images[currentImage]?.name}
                      style={{
                        position: 'absolute',
                        left: displayTransform.offsetX,
                        top: displayTransform.offsetY,
                        width: imageDimensions?.width,
                        height: imageDimensions?.height,
                        transform: `scale(${displayTransform.scale})`,
                        transformOrigin: 'top left',
                        userSelect: 'none',
                      }}
                      draggable={false}
                    />
                    {/* Annotation boxes overlay */}
                    <div style={{ position: 'absolute', left: displayTransform.offsetX, top: displayTransform.offsetY, transform: `scale(${displayTransform.scale})`, transformOrigin: 'top left', pointerEvents: 'none', width: imageDimensions?.width, height: imageDimensions?.height }}>
                      {annotations.map((ann) => (
                        <div
                          key={ann.id}
                          style={{
                            position: 'absolute',
                            left: ann.x,
                            top: ann.y,
                            width: ann.width,
                            height: ann.height,
                            border: `2px solid ${getClassColor(ann.classId)}`,
                            backgroundColor: `${getClassColor(ann.classId)}33`,
                            boxSizing: 'border-box',
                            pointerEvents: 'auto',
                            cursor: 'pointer',
                          }}
                          onClick={(e) => {
                            e.stopPropagation();
                            setSelectedAnnotation(ann.id);
                          }}
                        >
                          <span style={{
                            position: 'absolute',
                            top: -20,
                            left: 0,
                            background: getClassColor(ann.classId),
                            color: 'white',
                            padding: '1px 6px',
                            borderRadius: 3,
                            fontSize: 11,
                            whiteSpace: 'nowrap',
                          }}>
                            {ann.label}
                          </span>
                        </div>
                      ))}
                      {/* Current drawing box */}
                      {drawState.isDrawing && getDrawRect() && (
                        <div
                          style={{
                            position: 'absolute',
                            left: getDrawRect()!.x,
                            top: getDrawRect()!.y,
                            width: getDrawRect()!.width,
                            height: getDrawRect()!.height,
                            border: `2px dashed ${getClassColor(selectedClassId)}`,
                            backgroundColor: `${getClassColor(selectedClassId)}22`,
                            boxSizing: 'border-box',
                            pointerEvents: 'none',
                          }}
                        />
                      )}
                    </div>
                  </div>
                ) : (
                  <>
                    <ImageIcon size={64} />
                    <p style={{ fontSize: 14 }}>{images[currentImage]?.name || '无图片'}</p>
                  </>
                )}
              </div>
            )}
          </div>

          {/* Right Panel - Class Management */}
          <div className="annotation-sidebar-right">
            <div style={{ marginBottom: 'var(--spacing-lg)' }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 'var(--spacing-md)' }}>
                <h4 style={{ fontSize: 13, color: 'var(--text-secondary)' }}>类别管理</h4>
              </div>

              {/* Add new class */}
              <div style={{ display: 'flex', gap: 'var(--spacing-xs)', marginBottom: 'var(--spacing-sm)' }}>
                <input
                  type="text"
                  value={newClassName}
                  onChange={(e) => setNewClassName(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && handleAddClass()}
                  placeholder="新类别名称"
                  className="input"
                  style={{ flex: 1, fontSize: 12, padding: '4px 8px' }}
                />
                <button className="btn btn-primary" onClick={handleAddClass} style={{ padding: '4px 8px' }}>
                  <Plus size={14} />
                </button>
              </div>

              {/* Class list */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-xs)' }}>
                {classes.map((cls) => (
                  <div
                    key={cls.id}
                    onClick={() => setSelectedClassId(cls.id)}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 'var(--spacing-sm)',
                      padding: 'var(--spacing-sm)',
                      background: selectedClassId === cls.id ? 'var(--bg-active)' : 'var(--bg-elevated)',
                      borderRadius: 'var(--radius-sm)',
                      cursor: 'pointer',
                      borderLeft: `3px solid ${cls.color}`,
                    }}
                  >
                    {editingClassId === cls.id ? (
                      <>
                        <input
                          type="text"
                          value={editingClassName}
                          onChange={(e) => setEditingClassName(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') handleUpdateClass(cls.id, editingClassName);
                            if (e.key === 'Escape') setEditingClassId(null);
                          }}
                          autoFocus
                          className="input"
                          style={{ flex: 1, fontSize: 12, padding: '2px 6px' }}
                          onClick={(e) => e.stopPropagation()}
                        />
                        <button className="btn btn-ghost" onClick={(e) => { e.stopPropagation(); handleUpdateClass(cls.id, editingClassName); }} style={{ padding: 2 }}>
                          <Check size={12} />
                        </button>
                        <button className="btn btn-ghost" onClick={(e) => { e.stopPropagation(); setEditingClassId(null); }} style={{ padding: 2 }}>
                          <X size={12} />
                        </button>
                      </>
                    ) : (
                      <>
                        <div style={{ width: 12, height: 12, borderRadius: 2, background: cls.color, flexShrink: 0 }} />
                        <span style={{ flex: 1, fontSize: 13 }}>{cls.name}</span>
                        <button className="btn btn-ghost" onClick={(e) => { e.stopPropagation(); setEditingClassId(cls.id); setEditingClassName(cls.name); }} style={{ padding: 2 }}>
                          <Edit2 size={12} />
                        </button>
                        {classes.length > 1 && (
                          <button className="btn btn-ghost" onClick={(e) => { e.stopPropagation(); handleDeleteClass(cls.id); }} style={{ padding: 2 }}>
                            <Trash2 size={12} />
                          </button>
                        )}
                      </>
                    )}
                  </div>
                ))}
              </div>
            </div>

            <div>
              <h4 style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 'var(--spacing-md)' }}>
                统计
              </h4>
              <div style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                <p>总标注数: {annotations.length}</p>
                <p>类别数: {classes.length}</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
