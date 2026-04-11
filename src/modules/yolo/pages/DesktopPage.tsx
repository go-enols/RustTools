import { useState, useEffect, useRef } from 'react';
import {
  Monitor,
  Play,
  Square,
  Settings2,
  Gauge,
  Loader2,
  XCircle,
  Eye,
  MonitorUp,
  FolderOpen,
} from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import { useTrainingStore } from '../../../core/stores/trainingStore';
import {
  startDesktopCapture,
  stopDesktopCapture,
} from '../../../core/api/desktop';
import type { AnnotationBox } from '../../../core/api/types';

/// Desktop capture frame from backend
interface DesktopCaptureFrame {
  session_id: string;
  image: string; // Base64 encoded JPEG
  boxes: AnnotationBox[];
  width: number;
  height: number;
  fps: number;
  timestamp: number;
}

export default function DesktopPage() {
  const trainedModels = useTrainingStore((state) => state.trainedModels);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  // Model selection
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [customModelPath, setCustomModelPath] = useState<string | null>(null);

  // Inference settings
  const [confidence, setConfidence] = useState(0.65);
  const [gpuAccel, setGpuAccel] = useState(true);
  const [fpsLimit, setFpsLimit] = useState(30);
  const [monitor, setMonitor] = useState(1);

  // Inference state
  const [isLoading, setIsLoading] = useState(false);
  const [isCapturing, setIsCapturing] = useState(false);
  const [inferenceError, setInferenceError] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);

  // Detection state
  const [currentBoxes, setCurrentBoxes] = useState<AnnotationBox[]>([]);
  const [currentImage, setCurrentImage] = useState<string | null>(null);
  const [displaySize, setDisplaySize] = useState({ width: 1920, height: 1080 });
  const [lastFps, setLastFps] = useState(0);

  const selectedModel = trainedModels.find((m) => m.id === selectedModelId);
  const modelPath = selectedModel?.modelPath || '';
  
  // 支持自定义路径或训练列表路径
  const effectiveModelPath = customModelPath || modelPath;
  const isUsingCustomModel = !!customModelPath;
  
  // 浏览模型文件
  const handleBrowseModel = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'YOLO Models',
          extensions: ['onnx', 'pt', 'pth', 'safetensors']
        }]
      });
      
      if (selected) {
        setCustomModelPath(selected as string);
        setSelectedModelId(null); // 清除训练列表选择
      }
    } catch (err) {
      console.error('Failed to open file dialog:', err);
      setInferenceError('无法打开文件选择对话框');
    }
  };

  // Color palette for different classes
  const colors = [
    '#FF6B6B',
    '#4ECDC4',
    '#45B7D1',
    '#96CEB4',
    '#FFEAA7',
    '#DDA0DD',
    '#98D8C8',
    '#F7DC6F',
  ];

  // Listen for detection events
  useEffect(() => {
    let lastFrameTime = Date.now();

    const unlistenDetection = listen<DesktopCaptureFrame>('desktop-capture-frame', (event) => {
      const payload = event.payload;
      
      // Store current image and detections
      setCurrentImage(payload.image);
      setCurrentBoxes(payload.boxes);
      setDisplaySize({ width: payload.width, height: payload.height });
      setLastFps(payload.fps);
      
      // Calculate FPS locally
      const now = Date.now();
      if (now - lastFrameTime >= 1000) {
        lastFrameTime = now;
      }
    });

    return () => {
      unlistenDetection.then((fn) => fn());
    };
  }, []);

  // Redraw canvas when image or boxes change
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Set canvas size
    canvas.width = displaySize.width;
    canvas.height = displaySize.height;

    // Clear canvas
    ctx.clearRect(0, 0, displaySize.width, displaySize.height);

    if (currentImage) {
      // Draw image if available
      const img = new Image();
      img.onload = () => {
        // Draw image to fill canvas
        ctx.drawImage(img, 0, 0, displaySize.width, displaySize.height);
        
        // Draw detection boxes on top
        currentBoxes.forEach((box) => {
          const x = box.x;
          const y = box.y;
          const w = box.width;
          const h = box.height;
          const color = colors[box.class_id % colors.length];

          // Draw rectangle
          ctx.strokeStyle = color;
          ctx.lineWidth = 3;
          ctx.strokeRect(x, y, w, h);

          // Draw label background
          const label = `${box.class_name} ${(box.confidence || 0.9).toFixed(2)}`;
          ctx.font = 'bold 16px Arial';
          const textMetrics = ctx.measureText(label);
          const textHeight = 22;

          ctx.fillStyle = color;
          ctx.fillRect(x, y - textHeight - 4, textMetrics.width + 12, textHeight + 4);

          // Draw label text
          ctx.fillStyle = '#000000';
          ctx.fillText(label, x + 6, y - 8);

          // Draw class ID circle
          ctx.fillStyle = color;
          ctx.beginPath();
          ctx.arc(x + 15, y + 15, 16, 0, Math.PI * 2);
          ctx.fill();

          ctx.fillStyle = '#FFFFFF';
          ctx.font = 'bold 14px Arial';
          ctx.fillText(String(box.class_id), x + 9, y + 20);
        });

        // Draw info overlay
        ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
        ctx.fillRect(10, 10, 220, 70);
        ctx.fillStyle = '#FFFFFF';
        ctx.font = '14px Arial';
        ctx.fillText(`检测数量: ${currentBoxes.length}`, 20, 32);
        ctx.fillText(`FPS: ${lastFps}`, 20, 52);
      };
      img.src = `data:image/jpeg;base64,${currentImage}`;
    } else {
      // Draw placeholder if no image
      ctx.fillStyle = '#1a1a1a';
      ctx.fillRect(0, 0, displaySize.width, displaySize.height);
    }
  }, [currentBoxes, currentImage, displaySize, lastFps, colors]);

  // Start desktop capture
  const handleStartCapture = async () => {
    if (!effectiveModelPath) {
      setInferenceError('请先选择模型文件（从列表或浏览）');
      return;
    }

    // 检查模型格式
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const compatibilityResult = await invoke<{ success: boolean; data: { is_compatible: boolean; message: string } }>(
        'check_model_compatibility',
        { path: effectiveModelPath }
      );

      if (compatibilityResult.success && compatibilityResult.data) {
        const { is_compatible, message } = compatibilityResult.data;
        
        if (!is_compatible) {
          // 显示详细错误信息
          setInferenceError(message);
          return;
        }
      }
    } catch (err) {
      console.error('Failed to check model compatibility:', err);
      // 如果检查失败，继续尝试（可能是网络问题或其他原因）
    }

    setIsCapturing(true);
    setInferenceError(null);
    setCurrentBoxes([]);
    setCurrentImage(null);

    const response = await startDesktopCapture({
      model_path: effectiveModelPath,
      confidence,
      device: gpuAccel ? '0' : 'cpu',
      monitor,
      fps_limit: fpsLimit,
    });

    if (response.success && response.data) {
      const sessionId = response.data as unknown as string;
      setSessionId(sessionId);
    } else {
      setInferenceError(response.error || '启动桌面捕获失败');
      setIsCapturing(false);
    }
  };

  // Stop desktop capture
  const handleStopCapture = async () => {
    if (sessionId) {
      await stopDesktopCapture(sessionId);
      setIsCapturing(false);
      setSessionId(null);
      setCurrentImage(null);
    }
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div className="content-header">
        <h1 className="text-lg font-semibold">桌面推理</h1>
        <p className="text-sm text-tertiary mt-sm">
          实时捕获桌面画面进行目标检测
        </p>
      </div>

      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        {/* Left Panel - Preview */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', padding: 'var(--spacing-md)' }}>
          {/* Canvas area */}
          <div
            style={{
              flex: 1,
              background: 'var(--bg-elevated)',
              borderRadius: 'var(--radius-lg)',
              overflow: 'hidden',
              position: 'relative',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              minHeight: 400,
            }}
          >
            <canvas
              ref={canvasRef}
              style={{
                maxWidth: '100%',
                maxHeight: '100%',
                objectFit: 'contain',
              }}
            />

            {/* Empty state */}
            {!isCapturing && (
              <div
                style={{
                  position: 'absolute',
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                  gap: 'var(--spacing-md)',
                  color: 'var(--text-tertiary)',
                }}
              >
                <MonitorUp size={48} />
                <p>点击"开始捕获"进行实时桌面检测</p>
              </div>
            )}

            {/* Loading overlay */}
            {isLoading && (
              <div
                style={{
                  position: 'absolute',
                  inset: 0,
                  background: 'rgba(0,0,0,0.5)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                }}
              >
                <Loader2 size={32} className="spin" style={{ color: 'white' }} />
              </div>
            )}

            {/* Capturing indicator */}
            {isCapturing && (
              <div
                style={{
                  position: 'absolute',
                  top: 8,
                  right: 8,
                  background: 'rgba(0,0,0,0.7)',
                  borderRadius: 6,
                  padding: '4px 8px',
                  fontSize: 12,
                  color: '#4ECDC4',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                }}
              >
                <div
                  style={{
                    width: 8,
                    height: 8,
                    borderRadius: '50%',
                    background: '#FF6B6B',
                    animation: 'pulse 1s infinite',
                  }}
                />
                实时捕获中 | FPS: {lastFps}
              </div>
            )}
          </div>

          {/* Detection results summary */}
          <div
            style={{
              padding: 'var(--spacing-md)',
              background: 'var(--bg-surface)',
              borderRadius: 'var(--radius-md)',
              marginTop: 'var(--spacing-sm)',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
              <Eye size={14} />
              <span style={{ fontSize: 13, fontWeight: 500 }}>检测结果</span>
            </div>
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(4, 1fr)',
                gap: 'var(--spacing-sm)',
              }}
            >
              <div
                style={{
                  padding: '8px 12px',
                  background: 'var(--bg-elevated)',
                  borderRadius: 6,
                  textAlign: 'center',
                }}
              >
                <div style={{ fontSize: 20, fontWeight: 600, color: 'var(--accent-primary)' }}>
                  {currentBoxes.length}
                </div>
                <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>当前检测</div>
              </div>
              <div
                style={{
                  padding: '8px 12px',
                  background: 'var(--bg-elevated)',
                  borderRadius: 6,
                  textAlign: 'center',
                }}
              >
                <div style={{ fontSize: 20, fontWeight: 600, color: 'var(--status-success)' }}>
                  {lastFps}
                </div>
                <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>实时帧率</div>
              </div>
              <div
                style={{
                  padding: '8px 12px',
                  background: 'var(--bg-elevated)',
                  borderRadius: 6,
                  textAlign: 'center',
                }}
              >
                <div style={{ fontSize: 20, fontWeight: 600, color: 'var(--accent-secondary)' }}>
                  {displaySize.width}×{displaySize.height}
                </div>
                <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>分辨率</div>
              </div>
              <div
                style={{
                  padding: '8px 12px',
                  background: 'var(--bg-elevated)',
                  borderRadius: 6,
                  textAlign: 'center',
                }}
              >
                <div style={{ fontSize: 20, fontWeight: 600, color: 'var(--status-warning)' }}>
                  {monitor}
                </div>
                <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>显示器</div>
              </div>
            </div>

            {/* Detection list */}
            {currentBoxes.length > 0 && (
              <div
                style={{
                  marginTop: 'var(--spacing-sm)',
                  maxHeight: 120,
                  overflow: 'auto',
                  fontSize: 12,
                }}
              >
                <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                  <thead>
                    <tr style={{ borderBottom: '1px solid var(--border-default)' }}>
                      <th style={{ textAlign: 'left', padding: '4px 8px', color: 'var(--text-tertiary)' }}>
                        类别
                      </th>
                      <th style={{ textAlign: 'right', padding: '4px 8px', color: 'var(--text-tertiary)' }}>
                        置信度
                      </th>
                      <th style={{ textAlign: 'right', padding: '4px 8px', color: 'var(--text-tertiary)' }}>
                        位置
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {currentBoxes.map((box, idx) => (
                      <tr
                        key={idx}
                        style={{ borderBottom: '1px solid var(--border-subtle)' }}
                      >
                        <td style={{ padding: '4px 8px' }}>
                          <span
                            style={{
                              display: 'inline-block',
                              padding: '2px 6px',
                              borderRadius: 4,
                              background: 'var(--accent-primary)',
                              color: 'white',
                              fontSize: 11,
                            }}
                          >
                            {box.class_name}
                          </span>
                        </td>
                        <td style={{ textAlign: 'right', padding: '4px 8px' }}>
                          {(box.confidence || 0.9).toFixed(2)}
                        </td>
                        <td style={{ textAlign: 'right', padding: '4px 8px', color: 'var(--text-tertiary)' }}>
                          [{Math.round(box.x)}, {Math.round(box.y)}, {Math.round(box.width)},{' '}
                          {Math.round(box.height)}]
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>

          {/* Error display */}
          {inferenceError && (
            <div
              style={{
                padding: 'var(--spacing-md)',
                background: 'var(--status-error-bg)',
                borderRadius: 'var(--radius-md)',
                marginTop: 'var(--spacing-sm)',
                display: 'flex',
                alignItems: 'center',
                gap: 8,
              }}
            >
              <XCircle size={14} style={{ color: 'var(--status-error)' }} />
              <span style={{ fontSize: 13, color: 'var(--status-error)' }}>{inferenceError}</span>
            </div>
          )}
        </div>

        {/* Right Panel - Config */}
        <div
          style={{
            width: 280,
            background: 'var(--bg-surface)',
            borderLeft: '1px solid var(--border-default)',
            padding: 'var(--spacing-md)',
            display: 'flex',
            flexDirection: 'column',
            gap: 'var(--spacing-md)',
            overflow: 'auto',
          }}
        >
          {/* Model selection */}
          <div className="panel-section">
            <div className="panel-section-title">
              <Gauge size={14} />
              选择模型
            </div>
            
            {/* 下拉选择训练列表 */}
            <div style={{ marginBottom: 'var(--spacing-sm)' }}>
              <select
                className="select"
                value={selectedModelId || ''}
                onChange={(e) => {
                  setSelectedModelId(e.target.value || null);
                  setCustomModelPath(null); // 清除自定义路径
                }}
                style={{ width: '100%' }}
                disabled={isCapturing}
              >
                <option value="">选择训练好的模型...</option>
                {trainedModels.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.projectName} - {m.name}
                  </option>
                ))}
              </select>
            </div>
            
            {/* 或手动选择文件 */}
            <button
              className="btn-secondary"
              onClick={handleBrowseModel}
              disabled={isCapturing}
              style={{ width: '100%', marginBottom: 'var(--spacing-sm)' }}
            >
              <FolderOpen size={14} />
              浏览模型文件...
            </button>
            
            {/* 显示当前选择 */}
            {effectiveModelPath && (
              <div style={{ 
                fontSize: 11, 
                color: isUsingCustomModel ? 'var(--accent-primary)' : 'var(--text-tertiary)', 
                marginTop: 4,
                wordBreak: 'break-all'
              }}>
                <div style={{ fontWeight: 500, marginBottom: 2 }}>
                  {isUsingCustomModel ? '📁 自定义模型' : '📋 训练模型'}
                </div>
                <div style={{ opacity: 0.8 }}>
                  {effectiveModelPath.split(/[/\\]/).pop()}
                </div>
              </div>
            )}
            
            {!effectiveModelPath && (
              <div style={{ fontSize: 11, color: 'var(--status-warning)', marginTop: 4 }}>
                ⚠️ 请选择模型文件
              </div>
            )}
          </div>

          {/* Capture settings */}
          <div className="panel-section">
            <div className="panel-section-title">
              <Settings2 size={14} />
              捕获设置
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
              {/* Monitor selection */}
              <div>
                <label
                  style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}
                >
                  显示器
                </label>
                <select
                  className="select"
                  value={monitor}
                  onChange={(e) => setMonitor(parseInt(e.target.value))}
                  style={{ width: '100%' }}
                  disabled={isCapturing}
                >
                  <option value={1}>显示器 1 (主)</option>
                  <option value={2}>显示器 2</option>
                  <option value={3}>显示器 3</option>
                  <option value={4}>显示器 4</option>
                </select>
              </div>

              {/* FPS limit */}
              <div>
                <label
                  style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}
                >
                  帧率限制: {fpsLimit} FPS
                </label>
                <input
                  type="range"
                  min="5"
                  max="60"
                  step="5"
                  value={fpsLimit}
                  onChange={(e) => setFpsLimit(parseInt(e.target.value))}
                  className="slider"
                  disabled={isCapturing}
                />
              </div>

              {/* Confidence threshold */}
              <div>
                <label
                  style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}
                >
                  置信度阈值: {confidence.toFixed(2)}
                </label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={confidence}
                  onChange={(e) => setConfidence(parseFloat(e.target.value))}
                  className="slider"
                />
              </div>

              {/* GPU acceleration */}
              <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)', cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={gpuAccel}
                  onChange={(e) => setGpuAccel(e.target.checked)}
                  className="checkbox"
                  disabled={isCapturing}
                />
                <span style={{ fontSize: 13 }}>GPU加速</span>
              </label>
            </div>
          </div>

          {/* Capture control */}
          <div className="panel-section">
            <div className="panel-section-title">
              <Monitor size={14} />
              捕获控制
            </div>
            <div style={{ display: 'flex', gap: 8 }}>
              {!isCapturing ? (
                <button
                  className="btn-primary"
                  style={{ flex: 1 }}
                  onClick={handleStartCapture}
                  disabled={!effectiveModelPath || isLoading}
                >
                  <Play size={14} />
                  开始捕获
                </button>
              ) : (
                <button
                  className="btn-danger"
                  style={{ flex: 1 }}
                  onClick={handleStopCapture}
                >
                  <Square size={14} />
                  停止捕获
                </button>
              )}
            </div>
            <div
              style={{
                fontSize: 11,
                color: 'var(--text-tertiary)',
                marginTop: 8,
                lineHeight: 1.4,
              }}
            >
              💡 提示: 使用原生Rust实现高效桌面捕获，目前支持实时显示，YOLO推理即将上线。
            </div>
          </div>

          {/* Info */}
          <div className="panel-section">
            <div className="panel-section-title">功能说明</div>
            <div
              style={{
                fontSize: 12,
                color: 'var(--text-secondary)',
                lineHeight: 1.5,
              }}
            >
              <p style={{ marginBottom: 8 }}>
                本功能使用 <strong>Rust原生实现</strong>进行实时桌面捕获。
              </p>
              <p style={{ marginBottom: 8 }}>
                捕获采用底层API实现，对系统资源占用低，支持 30-60 FPS 的实时显示。
              </p>
              <p>⚠️ 注意：</p>
              <ul style={{ marginLeft: 16, marginTop: 4 }}>
                <li>YOLO推理功能开发中</li>
                <li>当前显示实际桌面画面</li>
                <li>检测框将在推理上线后显示</li>
              </ul>
            </div>
          </div>
        </div>
      </div>

      {/* CSS for pulse animation */}
      <style>{`
        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.5; }
        }
      `}</style>
    </div>
  );
}
