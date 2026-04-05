import { useState, useEffect, useCallback } from 'react';
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
} from 'lucide-react';
import { useWorkspaceStore } from '../../../core/stores/workspaceStore';
import { listDirectory, readBinaryFile } from '../../../core/api/file';

type Tool = 'select' | 'draw';

interface AnnotationItem {
  id: string;
  label: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

interface ImageFile {
  name: string;
  path: string;
}

export default function AnnotationPage() {
  const { currentProject } = useWorkspaceStore();
  const [tool, setTool] = useState<Tool>('select');
  const [currentImage, setCurrentImage] = useState(0);
  const [images, setImages] = useState<ImageFile[]>([]);
  const [annotations, setAnnotations] = useState<AnnotationItem[]>([]);
  const [selectedAnnotation, setSelectedAnnotation] = useState<string | null>(null);
  const [showUnlabeled, setShowUnlabeled] = useState(false);
  const [zoom, setZoom] = useState(100);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [currentImageData, setCurrentImageData] = useState<string | null>(null);
  const [imageLoading, setImageLoading] = useState(false);

  // Auto-load images from project config on mount
  useEffect(() => {
    const loadImages = async () => {
      if (!currentProject) return;

      setIsLoading(true);
      setError(null);

      // Use path from project config
      const trainPath = `${currentProject.path}/${currentProject.images.train}`;
      const valPath = `${currentProject.path}/${currentProject.images.val}`;

      // Try train folder first
      const result = await listDirectory(trainPath);

      if (!result.success || !result.data) {
        // Try val folder if train folder doesn't exist
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

  // Load image data when current image changes
  useEffect(() => {
    const loadImageData = async () => {
      if (images.length === 0 || !images[currentImage]) {
        setCurrentImageData(null);
        return;
      }

      setImageLoading(true);
      const img = images[currentImage];
      const result = await readBinaryFile(img.path);

      if (result.success && result.data) {
        // Determine MIME type from file extension
        const ext = img.name.toLowerCase().split('.').pop();
        const mimeType = ext === 'png' ? 'image/png' : ext === 'jpg' || ext === 'jpeg' ? 'image/jpeg' : 'image/gif';
        setCurrentImageData(`data:${mimeType};base64,${result.data}`);
      } else {
        setCurrentImageData(null);
      }
      setImageLoading(false);
    };

    loadImageData();
  }, [currentImage, images]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Skip if user is typing in an input
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
            setCurrentImage((prev) => prev - 1);
          }
          break;
        case 'd':
          if (currentImage < images.length - 1) {
            setCurrentImage((prev) => prev + 1);
          }
          break;
        case 'delete':
        case 'backspace':
          if (selectedAnnotation) {
            setAnnotations((prev) => prev.filter((ann) => ann.id !== selectedAnnotation));
            setSelectedAnnotation(null);
          }
          break;
        case 'escape':
          setSelectedAnnotation(null);
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [currentImage, images.length, selectedAnnotation]);

  // Classes from project config
  const classes = currentProject?.classes.map((name, idx) => ({
    id: idx,
    name,
    color: `var(--accent-primary)`, // TODO: Map to distinct colors
  })) || [];

  const handleDeleteSelected = useCallback(() => {
    if (selectedAnnotation) {
      setAnnotations((prev) => prev.filter((ann) => ann.id !== selectedAnnotation));
      setSelectedAnnotation(null);
    }
  }, [selectedAnnotation]);

  return (
    <div className="annotation-canvas" style={{ height: '100%' }}>
      {/* Left Toolbar */}
      <div className="annotation-toolbar">
        <div
          className={`annotation-tool-btn ${tool === 'select' ? 'active' : ''}`}
          onClick={() => setTool('select')}
          title="选择 (Q)"
        >
          <MousePointer2 size={20} />
        </div>
        <div
          className={`annotation-tool-btn ${tool === 'draw' ? 'active' : ''}`}
          onClick={() => setTool('draw')}
          title="绘制 (W)"
        >
          <Square size={20} />
        </div>
        <div style={{ flex: 1 }} />
        <div className="annotation-tool-btn" title="上一张 (A)">
          <ChevronLeft size={20} />
        </div>
        <div className="annotation-tool-btn" title="下一张 (D)">
          <ChevronRight size={20} />
        </div>
        <div className="annotation-tool-btn" onClick={() => setZoom(Math.min(zoom + 25, 400))} title="放大">
          <ZoomIn size={20} />
        </div>
        <div className="annotation-tool-btn" onClick={() => setZoom(Math.max(zoom - 25, 25))} title="缩小">
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
                进度: {currentImage}/{images.length} ({images.length > 0 ? ((currentImage / images.length) * 100).toFixed(1) : 0}%)
              </span>
              <span style={{ fontSize: 13, color: 'var(--text-tertiary)' }}>
                图片: {annotations.length}
              </span>
            </div>
            {/* Keyboard Shortcuts - Prominent Display */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-md)', padding: 'var(--spacing-xs) var(--spacing-sm)', background: 'var(--bg-elevated)', borderRadius: 'var(--radius-sm)' }}>
              <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>快捷键:</span>
              <div style={{ display: 'flex', gap: 'var(--spacing-md)' }}>
                <span style={{ fontSize: 11, display: 'flex', alignItems: 'center', gap: 4 }}><kbd style={{ padding: '1px 5px', background: 'var(--bg-surface)', border: '1px solid var(--border-default)', borderRadius: 3, fontSize: 10 }}>Q</kbd> 拖动</span>
                <span style={{ fontSize: 11, display: 'flex', alignItems: 'center', gap: 4 }}><kbd style={{ padding: '1px 5px', background: 'var(--bg-surface)', border: '1px solid var(--border-default)', borderRadius: 3, fontSize: 10 }}>W</kbd> 绘制</span>
                <span style={{ fontSize: 11, display: 'flex', alignItems: 'center', gap: 4 }}><kbd style={{ padding: '1px 5px', background: 'var(--bg-surface)', border: '1px solid var(--border-default)', borderRadius: 3, fontSize: 10 }}>A</kbd> 前一张</span>
                <span style={{ fontSize: 11, display: 'flex', alignItems: 'center', gap: 4 }}><kbd style={{ padding: '1px 5px', background: 'var(--bg-surface)', border: '1px solid var(--border-default)', borderRadius: 3, fontSize: 10 }}>D</kbd> 下一张</span>
              </div>
            </div>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-md)' }}>
            <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)', fontSize: 13, color: 'var(--text-secondary)', cursor: 'pointer' }}>
              <input
                type="checkbox"
                checked={showUnlabeled}
                onChange={(e) => setShowUnlabeled(e.target.checked)}
                className="checkbox"
              />
              未标注筛选
            </label>
            {currentProject && (
              <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                数据集: {currentProject.images.train}
              </span>
            )}
          </div>
        </div>

        <div className="annotation-content">
          {/* Left Panel - Label List */}
          <div className="annotation-sidebar-left">
            <div>
              <h4 style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 'var(--spacing-md)' }}>
                标注列表
              </h4>
              {annotations.length === 0 ? (
                <p style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>暂无标注</p>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-sm)' }}>
                  {annotations.map((ann) => (
                    <div
                      key={ann.id}
                      style={{
                        padding: 'var(--spacing-sm)',
                        background: 'var(--bg-elevated)',
                        borderRadius: 'var(--radius-sm)',
                        fontSize: 12,
                      }}
                    >
                      <span style={{ color: 'var(--accent-primary)' }}>{ann.label}</span>
                      <span style={{ color: 'var(--text-tertiary)', marginLeft: 8 }}>
                        ({ann.x}, {ann.y})
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
              <div
                style={{
                  width: '80%',
                  height: '80%',
                  background: 'var(--bg-surface)',
                  border: '2px dashed var(--border-default)',
                  borderRadius: 'var(--radius-lg)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  flexDirection: 'column',
                  gap: 'var(--spacing-md)',
                }}
              >
                <div style={{
                  width: 40,
                  height: 40,
                  border: '3px solid var(--border-default)',
                  borderTopColor: 'var(--accent-primary)',
                  borderRadius: '50%',
                  animation: 'spin 1s linear infinite',
                }} />
                <p style={{ color: 'var(--text-tertiary)', fontSize: 14 }}>加载图片中...</p>
                <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
              </div>
            ) : error ? (
              <div
                style={{
                  width: '80%',
                  height: '80%',
                  background: 'var(--bg-surface)',
                  border: '2px dashed var(--border-default)',
                  borderRadius: 'var(--radius-lg)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  flexDirection: 'column',
                  gap: 'var(--spacing-md)',
                }}
              >
                <FolderOpen size={48} style={{ color: 'var(--status-error)' }} />
                <p style={{ color: 'var(--status-error)', fontSize: 14 }}>{error}</p>
                <p style={{ color: 'var(--text-tertiary)', fontSize: 12 }}>
                  请确保项目包含 images/train 或 images/val 文件夹
                </p>
              </div>
            ) : images.length === 0 ? (
              <div
                style={{
                  width: '80%',
                  height: '80%',
                  background: 'var(--bg-surface)',
                  border: '2px dashed var(--border-default)',
                  borderRadius: 'var(--radius-lg)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  flexDirection: 'column',
                  gap: 'var(--spacing-md)',
                }}
              >
                <ImageIcon size={48} style={{ color: 'var(--text-tertiary)' }} />
                <p style={{ color: 'var(--text-tertiary)', fontSize: 14 }}>
                  未找到图片文件
                </p>
                <p style={{ color: 'var(--text-tertiary)', fontSize: 12 }}>
                  请将图片放入 images/train 文件夹
                </p>
              </div>
            ) : (
              <div
                style={{
                  width: '100%',
                  height: '100%',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  flexDirection: 'column',
                  gap: 'var(--spacing-md)',
                  color: 'var(--text-tertiary)',
                }}
              >
                {imageLoading ? (
                  <>
                    <div style={{
                      width: 40,
                      height: 40,
                      border: '3px solid var(--border-default)',
                      borderTopColor: 'var(--accent-primary)',
                      borderRadius: '50%',
                      animation: 'spin 1s linear infinite',
                    }} />
                    <p style={{ fontSize: 14 }}>加载图片...</p>
                  </>
                ) : currentImageData ? (
                  <img
                    src={currentImageData}
                    alt={images[currentImage]?.name}
                    style={{
                      maxWidth: '100%',
                      maxHeight: '100%',
                      objectFit: 'contain',
                      transform: `scale(${zoom / 100})`,
                    }}
                  />
                ) : (
                  <>
                    <ImageIcon size={64} />
                    <p style={{ fontSize: 14 }}>
                      {images[currentImage]?.name || '无图片'}
                    </p>
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
                <button className="btn btn-ghost" style={{ padding: '2px 8px' }}>
                  <Plus size={14} />
                </button>
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-sm)' }}>
                {classes.map((cls) => (
                  <div
                    key={cls.id}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 'var(--spacing-sm)',
                      padding: 'var(--spacing-sm)',
                      background: 'var(--bg-elevated)',
                      borderRadius: 'var(--radius-sm)',
                    }}
                  >
                    <div
                      style={{
                        width: 12,
                        height: 12,
                        borderRadius: 'var(--radius-sm)',
                        background: cls.color,
                      }}
                    />
                    <span style={{ flex: 1, fontSize: 13 }}>{cls.name}</span>
                    <button className="btn btn-ghost" style={{ padding: 2 }}>
                      <Edit2 size={12} />
                    </button>
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
