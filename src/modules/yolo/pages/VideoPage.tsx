import { useState, useRef, useEffect } from 'react';
import {
  Video,
  FolderOpen,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Volume2,
  VolumeX,
  Camera,
  Settings2,
  Gauge,
  Loader2,
  CheckCircle,
  XCircle,
} from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { useTrainingStore } from '../../../core/stores/trainingStore';
import {
  loadVideo,
  startVideoInference,
  startRustVideoInference,
  stopVideoInference,
  stopRustVideoInference,
  extractFrames,
} from '../../../core/api/video';
import type { VideoInferenceConfig, AnnotationBox } from '../../../core/api/types';

interface VideoMetadata {
  duration: number;
  fps: number;
  frames: number;
  width: number;
  height: number;
}

interface InferenceFrame {
  frameIndex: number;
  timestampMs: number;
  boxes: AnnotationBox[];
  screenshotPath?: string;
}

export default function VideoPage() {
  const videoRef = useRef<HTMLVideoElement>(null);
  const trainedModels = useTrainingStore((state) => state.trainedModels);

  // Video state
  const [videoPath, setVideoPath] = useState<string | null>(null);
  const [videoMeta, setVideoMeta] = useState<VideoMetadata | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [isMuted, setIsMuted] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);

  // Model selection
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [customModelPath, setCustomModelPath] = useState<string | null>(null);
  
  // 从训练列表获取模型路径
  const selectedModel = trainedModels.find((m) => m.id === selectedModelId);
  const modelPath = selectedModel?.modelPath || '';
  
  // 支持自定义路径或训练列表路径
  const effectiveModelPath = customModelPath || modelPath;
  const isUsingCustomModel = !!customModelPath;

  // 浏览模型文件
  const handleBrowseModel = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'YOLO Models',
          extensions: ['onnx', 'pt', 'pth', 'safetensors']
        }]
      });
      
      if (selected) {
        const path = typeof selected === 'string' ? selected : selected[0];
        setCustomModelPath(path);
        setSelectedModelId(null); // 清除训练列表选择
      }
    } catch (err) {
      console.error('Failed to open file dialog:', err);
      setInferenceError('无法打开文件选择对话框');
    }
  };

  // Inference settings
  const [confidence, setConfidence] = useState(0.65);
  const [gpuAccel, setGpuAccel] = useState(true);
  const [isInferenceEnabled, setIsInferenceEnabled] = useState(true);
  const [useRustInference, setUseRustInference] = useState(true); // 默认使用 Rust 推理

  // Screenshot settings
  const [showScreenshot, setShowScreenshot] = useState(false);
  const [screenshotCount, setScreenshotCount] = useState(10);
  const [screenshotMode, setScreenshotMode] = useState<'interval' | 'frames'>('interval');
  const [intervalMs, setIntervalMs] = useState(1000);

  // Inference state
  const [isLoading, setIsLoading] = useState(false);
  const [isInferring, setIsInferring] = useState(false);
  const [inferenceError, setInferenceError] = useState<string | null>(null);
  const [inferenceResults, setInferenceResults] = useState<InferenceFrame[]>([]);
  const [currentInferenceFrame, setCurrentInferenceFrame] = useState<number>(0);
  const [sessionId, setSessionId] = useState<string | null>(null);

  // Processed frames display
  const [processedFrames, setProcessedFrames] = useState<Set<number>>(new Set());

  // Listen for inference events
  useEffect(() => {
    // Python 推理事件
    const unlistenFrame = listen<{
      session_id: string;
      frame: number;
      boxes: AnnotationBox[];
    }>('video-inference-frame', (event) => {
      if (sessionId && event.payload.session_id === sessionId) {
        setCurrentInferenceFrame(event.payload.frame);
        setProcessedFrames((prev) => new Set([...prev, event.payload.frame]));
        setInferenceResults((prev) => [
          ...prev,
          {
            frameIndex: event.payload.frame,
            timestampMs: 0,
            boxes: event.payload.boxes,
          },
        ]);
      }
    });

    const unlistenComplete = listen<{
      session_id: string;
      success: boolean;
      error?: string;
    }>('video-inference-complete', (event) => {
      if (sessionId && event.payload.session_id === sessionId) {
        setIsInferring(false);
        if (!event.payload.success) {
          setInferenceError(event.payload.error || 'Inference failed');
        }
      }
    });

    // Rust 推理事件
    const unlistenRustFrame = listen<{
      session_id: string;
      frame: number;
      boxes: AnnotationBox[];
    }>('rust-video-inference-frame', (event) => {
      if (sessionId && event.payload.session_id === sessionId) {
        setCurrentInferenceFrame(event.payload.frame);
        setProcessedFrames((prev) => new Set([...prev, event.payload.frame]));
        setInferenceResults((prev) => [
          ...prev,
          {
            frameIndex: event.payload.frame,
            timestampMs: 0,
            boxes: event.payload.boxes,
          },
        ]);
      }
    });

    const unlistenRustComplete = listen<{
      session_id: string;
      success: boolean;
      frames?: number;
      results_path?: string;
      error?: string;
    }>('rust-video-inference-complete', (event) => {
      if (sessionId && event.payload.session_id === sessionId) {
        setIsInferring(false);
        if (!event.payload.success) {
          setInferenceError(event.payload.error || 'Inference failed');
        } else {
          console.log(`[Rust Inference] Complete! Processed ${event.payload.frames} frames`);
          console.log(`[Rust Inference] Results saved to: ${event.payload.results_path}`);
        }
      }
    });

    return () => {
      unlistenFrame.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenRustFrame.then((fn) => fn());
      unlistenRustComplete.then((fn) => fn());
    };
  }, [sessionId]);

  // Handle video time update
  const handleTimeUpdate = () => {
    if (videoRef.current) {
      setCurrentTime(videoRef.current.currentTime);
    }
  };

  // Handle video loaded metadata
  const handleLoadedMetadata = () => {
    if (videoRef.current) {
      setDuration(videoRef.current.duration);
    }
  };

  // Open video file
  const handleOpenVideo = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Video', extensions: ['mp4', 'avi', 'mov', 'mkv', 'webm'] }],
      });

      if (!selected) return;

      const path = typeof selected === 'string' ? selected : selected;
      setVideoPath(path);
      setIsLoading(true);
      setInferenceError(null);
      setInferenceResults([]);
      setProcessedFrames(new Set());

      // Load video metadata via backend
      const response = await loadVideo(path);
      if (response.success && response.data) {
        setVideoMeta(response.data);
      }

      // Load video in video element
      if (videoRef.current) {
        videoRef.current.src = `file://${path}`;
        videoRef.current.load();
      }
    } catch (err) {
      setInferenceError(`Failed to open video: ${err}`);
    } finally {
      setIsLoading(false);
    }
  };

  // Toggle play/pause
  const togglePlay = () => {
    if (videoRef.current) {
      if (isPlaying) {
        videoRef.current.pause();
      } else {
        videoRef.current.play();
      }
      setIsPlaying(!isPlaying);
    }
  };

  // Seek video
  const handleSeek = (time: number) => {
    if (videoRef.current) {
      videoRef.current.currentTime = time;
      setCurrentTime(time);
    }
  };

  // Start inference
  const handleStartInference = async () => {
    if (!videoPath || !effectiveModelPath) {
      setInferenceError('请先选择视频和模型文件（从列表或浏览）');
      return;
    }

    setIsInferring(true);
    setInferenceError(null);
    setInferenceResults([]);
    setProcessedFrames(new Set());

    const config: VideoInferenceConfig = {
      video_path: videoPath,
      model_path: effectiveModelPath,
      confidence,
      iou_threshold: 0.5,
      device: gpuAccel ? '0' : 'cpu',
      output_dir: `/tmp/inference_${Date.now()}`,
      frame_interval: 1,
    };

    // 根据选择使用不同的推理引擎
    const response = useRustInference
      ? await startRustVideoInference(config)
      : await startVideoInference(config);
      
    if (response.success && response.data) {
      setSessionId(response.data.inference_id);
    } else {
      setInferenceError(response.error || '启动推理失败');
      setIsInferring(false);
    }
  };

  // Stop inference
  const handleStopInference = async () => {
    if (sessionId) {
      // 根据使用的推理引擎停止
      if (useRustInference) {
        await stopRustVideoInference();
      } else {
        await stopVideoInference();
      }
      setIsInferring(false);
    }
  };

  // Capture screenshots
  const handleCaptureScreenshots = async () => {
    if (!videoPath || !videoMeta) return;

    setIsLoading(true);
    try {
      if (screenshotMode === 'interval') {
        // Extract frames at interval
        const interval = Math.max(intervalMs, 100);
        const response = await extractFrames(videoPath, interval);
        if (response.success && response.data) {
          console.log(`Extracted ${response.data.length} frames`);
        }
      } else {
        // Capture specific number of frames evenly distributed
        const interval = Math.floor((videoMeta.duration * 1000) / screenshotCount);
        const response = await extractFrames(videoPath, Math.max(interval, 100));
        if (response.success && response.data) {
          console.log(`Extracted ${response.data.length} frames`);
        }
      }
    } catch (err) {
      setInferenceError(`Screenshot failed: ${err}`);
    } finally {
      setIsLoading(false);
    }
  };

  // Format time display
  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  };

  // Get current frame index

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div className="content-header">
        <h1 className="text-lg font-semibold">视频推理</h1>
        <p className="text-sm text-tertiary mt-sm">对视频进行目标检测推理</p>
      </div>

      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        {/* Left Panel - Video Player */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', padding: 'var(--spacing-md)' }}>
          {/* Video area */}
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
            }}
          >
            {videoPath ? (
              <>
                <video
                  ref={videoRef}
                  style={{ width: '100%', height: '100%', objectFit: 'contain' }}
                  onTimeUpdate={handleTimeUpdate}
                  onLoadedMetadata={handleLoadedMetadata}
                  onPlay={() => setIsPlaying(true)}
                  onPause={() => setIsPlaying(false)}
                  muted={isMuted}
                />
                {/* Inference overlay */}
                {isInferenceEnabled && inferenceResults.length > 0 && (
                  <div style={{ position: 'absolute', inset: 0, pointerEvents: 'none' }}>
                    {/* Draw boxes for current frame */}
                  </div>
                )}
              </>
            ) : (
              <div
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                  gap: 'var(--spacing-md)',
                  color: 'var(--text-tertiary)',
                }}
              >
                <Video size={48} />
                <p>点击下方按钮加载视频</p>
              </div>
            )}

            {/* Loading overlay */}
            {isLoading && (
              <div style={{ position: 'absolute', inset: 0, background: 'var(--overlay-light)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <Loader2 size={32} className="spin" style={{ color: 'white' }} />
              </div>
            )}

            {/* Inference progress overlay */}
            {isInferring && (
              <div style={{ position: 'absolute', top: 8, right: 8, background: 'var(--overlay-medium)', borderRadius: 6, padding: '4px 8px', fontSize: 12, color: 'white' }}>
                🔍 推理中: Frame {currentInferenceFrame}
              </div>
            )}
          </div>

          {/* Video controls */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-md)', padding: 'var(--spacing-md) 0' }}>
            <button className="btn-icon" onClick={() => handleSeek(Math.max(0, currentTime - 10))}>
              <SkipBack size={16} />
            </button>
            <button className="btn-icon" onClick={togglePlay}>
              {isPlaying ? <Pause size={16} /> : <Play size={16} />}
            </button>
            <button className="btn-icon" onClick={() => handleSeek(Math.min(duration, currentTime + 10))}>
              <SkipForward size={16} />
            </button>

            {/* Progress bar */}
            <div style={{ flex: 1, display: 'flex', alignItems: 'center', gap: 8 }}>
              <span style={{ fontSize: 12, color: 'var(--text-tertiary)', minWidth: 40 }}>{formatTime(currentTime)}</span>
              <input
                type="range"
                min={0}
                max={duration || 100}
                step={0.1}
                value={currentTime}
                onChange={(e) => handleSeek(parseFloat(e.target.value))}
                className="slider"
                style={{ flex: 1 }}
              />
              <span style={{ fontSize: 12, color: 'var(--text-tertiary)', minWidth: 40 }}>{formatTime(duration)}</span>
            </div>

            <button className="btn-icon" onClick={() => setIsMuted(!isMuted)}>
              {isMuted ? <VolumeX size={16} /> : <Volume2 size={16} />}
            </button>

            <button className="btn-primary" onClick={handleOpenVideo}>
              <FolderOpen size={14} />
              打开视频
            </button>
          </div>

          {/* Inference status */}
          {(isInferring || inferenceError || inferenceResults.length > 0) && (
            <div style={{ padding: 'var(--spacing-md)', background: 'var(--bg-surface)', borderRadius: 'var(--radius-md)', marginTop: 'var(--spacing-sm)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                {isInferring && <Loader2 size={14} className="spin" />}
                {inferenceError && <XCircle size={14} style={{ color: 'var(--status-error)' }} />}
                {inferenceResults.length > 0 && !isInferring && <CheckCircle size={14} style={{ color: 'var(--status-success)' }} />}
                <span style={{ fontSize: 13 }}>
                  {isInferring && `推理中... 已处理 ${processedFrames.size} 帧`}
                  {inferenceError && `错误: ${inferenceError}`}
                  {inferenceResults.length > 0 && !isInferring && `推理完成 (${inferenceResults.length} 帧有检测结果)`}
                </span>
              </div>
            </div>
          )}
        </div>

        {/* Right Panel - Config */}
        <div style={{ width: 280, background: 'var(--bg-surface)', borderLeft: '1px solid var(--border-default)', padding: 'var(--spacing-md)', display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)', overflow: 'auto' }}>
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
                disabled={isInferring}
              >
                <option value="">选择训练好的模型...</option>
                {trainedModels.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.projectName} - {m.modelPath.split('/').pop()}
                  </option>
                ))}
              </select>
            </div>
            
            {/* 或手动选择文件 */}
            <button
              className="btn-secondary"
              onClick={handleBrowseModel}
              disabled={isInferring}
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

          {/* Inference settings */}
          <div className="panel-section">
            <div className="panel-section-title">
              <Settings2 size={14} />
              推理设置
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
              {/* 推理引擎选择 */}
              <div>
                <label
                  style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}
                >
                  推理引擎
                </label>
                <select
                  className="select"
                  value={useRustInference ? 'rust' : 'python'}
                  onChange={(e) => setUseRustInference(e.target.value === 'rust')}
                  style={{ width: '100%' }}
                  disabled={isInferring}
                >
                  <option value="rust">🤖 Rust (推荐 - 纯 Rust 高性能)</option>
                  <option value="python">🐍 Python (需要环境配置)</option>
                </select>
                <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 4 }}>
                  {useRustInference 
                    ? '✓ 纯 Rust 实现，无需 Python 环境' 
                    : '⚠️ 需要安装 Python 和 ultralytics'}
                </div>
              </div>
              
              <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)', cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={isInferenceEnabled}
                  onChange={(e) => setIsInferenceEnabled(e.target.checked)}
                  className="checkbox"
                />
                <span style={{ fontSize: 13 }}>开启推理</span>
              </label>
              <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)', cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={gpuAccel}
                  onChange={(e) => setGpuAccel(e.target.checked)}
                  className="checkbox"
                  disabled={useRustInference} // Rust 推理暂不支持 GPU
                />
                <span style={{ fontSize: 13, color: useRustInference ? 'var(--text-tertiary)' : undefined }}>
                  GPU加速 {!useRustInference ? '' : '(开发中)'}
                </span>
              </label>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}>
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
            </div>

            {/* Inference buttons */}
            <div style={{ display: 'flex', gap: 8, marginTop: 'var(--spacing-md)' }}>
              {!isInferring ? (
                <button
                  className="btn-primary"
                  style={{ flex: 1 }}
                  onClick={handleStartInference}
                  disabled={!videoPath || !effectiveModelPath}
                >
                  {useRustInference ? '🤖 Rust 推理' : '🐍 Python 推理'}
                </button>
              ) : (
                <button
                  className="btn-danger"
                  style={{ flex: 1 }}
                  onClick={handleStopInference}
                >
                  停止推理
                </button>
              )}
            </div>
          </div>

          {/* Screenshot settings */}
          <div className="panel-section">
            <div className="panel-section-title">
              <Camera size={14} />
              截图设置
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
              <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)', cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={showScreenshot}
                  onChange={(e) => setShowScreenshot(e.target.checked)}
                  className="checkbox"
                />
                <span style={{ fontSize: 13 }}>开启截图</span>
              </label>
              {showScreenshot && (
                <>
                  <div>
                    <label style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}>
                      截图数量
                    </label>
                    <input
                      type="number"
                      className="input"
                      value={screenshotCount}
                      onChange={(e) => setScreenshotCount(parseInt(e.target.value) || 10)}
                      min={1}
                      max={100}
                      style={{ width: '100%' }}
                    />
                  </div>
                  <div>
                    <label style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}>
                      模式
                    </label>
                    <select
                      className="select"
                      value={screenshotMode}
                      onChange={(e) => setScreenshotMode(e.target.value as 'interval' | 'frames')}
                      style={{ width: '100%' }}
                    >
                      <option value="interval">时间间隔</option>
                      <option value="frames">分帧模式</option>
                    </select>
                  </div>
                  <div>
                    <label style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}>
                      {screenshotMode === 'interval' ? '时间间隔 (ms)' : '帧间隔'}
                    </label>
                    <input
                      type="number"
                      className="input"
                      value={intervalMs}
                      onChange={(e) => setIntervalMs(parseInt(e.target.value) || 1000)}
                      min={100}
                      step={100}
                      style={{ width: '100%' }}
                    />
                  </div>
                  <button
                    className="btn-secondary"
                    style={{ width: '100%' }}
                    onClick={handleCaptureScreenshots}
                    disabled={!videoPath}
                  >
                    <Camera size={14} />
                    提取截图
                  </button>
                </>
              )}
            </div>
          </div>

          {/* Video info */}
          {videoMeta && (
            <div className="panel-section">
              <div className="panel-section-title">视频信息</div>
              <div style={{ fontSize: 12, color: 'var(--text-secondary)', display: 'flex', flexDirection: 'column', gap: 4 }}>
                <div>分辨率: {videoMeta.width} × {videoMeta.height}</div>
                <div>帧率: {videoMeta.fps.toFixed(1)} fps</div>
                <div>时长: {formatTime(videoMeta.duration)}</div>
                <div>总帧数: {videoMeta.frames}</div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
