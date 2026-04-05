import { useState, useRef } from 'react';
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
} from 'lucide-react';
import { useTrainingStore } from '../../../core/stores/trainingStore';

export default function VideoPage() {
  const videoRef = useRef<HTMLVideoElement>(null);
  const trainedModels = useTrainingStore((state) => state.trainedModels);
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [isMuted, setIsMuted] = useState(false);
  const [isInferenceEnabled, setIsInferenceEnabled] = useState(true);
  const [confidence, setConfidence] = useState(0.65);
  const [currentTime] = useState(0);
  const [duration] = useState(0);
  const [showScreenshot, setShowScreenshot] = useState(false);
  const [screenshotCount, setScreenshotCount] = useState(10);
  const [screenshotMode, setScreenshotMode] = useState<'interval' | 'frames'>('interval');
  const [intervalMs, setIntervalMs] = useState(50);
  const [handDetection, setHandDetection] = useState(false);
  const [gpuAccel, setGpuAccel] = useState(true);
  const [screenshots] = useState<string[]>([]);

  const selectedModel = trainedModels.find((m) => m.id === selectedModelId);
  const modelPath = selectedModel?.modelPath || '';

  const handleOpenVideo = async () => {
    // TODO: Implement with Tauri dialog API
  };

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

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div className="content-header">
        <h1 className="text-lg font-semibold">视频推理</h1>
        <p className="text-sm text-tertiary mt-sm">对视频进行目标检测推理</p>
      </div>

      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        {/* Left Panel - Screenshot List */}
        <div style={{ width: 200, background: 'var(--bg-surface)', borderRight: '1px solid var(--border-default)', display: 'flex', flexDirection: 'column' }}>
          <div style={{ padding: 'var(--spacing-md)', borderBottom: '1px solid var(--border-default)' }}>
            <span style={{ fontSize: 12, color: 'var(--text-tertiary)', textTransform: 'uppercase', letterSpacing: 1 }}>
              推理结果 ({screenshots.length})
            </span>
          </div>
          <div style={{ flex: 1, overflow: 'auto', padding: 'var(--spacing-sm)' }}>
            {screenshots.length === 0 ? (
              <div className="empty-state" style={{ padding: 'var(--spacing-lg)' }}>
                <Camera size={20} style={{ color: 'var(--text-tertiary)', marginBottom: 'var(--spacing-sm)' }} />
                <p style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>暂无截图</p>
              </div>
            ) : (
              screenshots.map((s, i) => (
                <div key={i} style={{ marginBottom: 'var(--spacing-sm)', borderRadius: 'var(--radius-sm)', overflow: 'hidden' }}>
                  <img src={s} alt={`Screenshot ${i + 1}`} style={{ width: '100%' }} />
                </div>
              ))
            )}
          </div>
        </div>

        {/* Main Video Area */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
          {/* Model Path Bar */}
          <div style={{ padding: 'var(--spacing-sm) var(--spacing-lg)', background: 'var(--bg-elevated)', display: 'flex', alignItems: 'center', gap: 'var(--spacing-md)' }}>
            {trainedModels.length > 0 ? (
              <select
                className="select"
                value={selectedModelId || ''}
                onChange={(e) => setSelectedModelId(e.target.value || null)}
                style={{ flex: 1 }}
              >
                <option value="">-- 选择训练好的模型 --</option>
                {trainedModels.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.projectName} ({m.yoloVersion}/{m.modelSize}) - Epoch {m.bestEpoch}
                  </option>
                ))}
              </select>
            ) : (
              <input
                type="text"
                className="input"
                value={modelPath}
                placeholder="暂无训练模型，请先训练模型..."
                readOnly
                style={{ flex: 1 }}
              />
            )}
            <button className="btn btn-secondary" style={{ padding: '4px 12px' }}>
              <FolderOpen size={14} />
              自定义模型
            </button>
            {selectedModel && (
              <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                mAP50: {(selectedModel.map50 * 100).toFixed(1)}%
              </span>
            )}
          </div>

          {/* Video Canvas */}
          <div style={{ flex: 1, background: 'var(--bg-primary)', display: 'flex', alignItems: 'center', justifyContent: 'center', position: 'relative' }}>
            <div style={{ textAlign: 'center' }}>
              <div
                style={{
                  width: 80,
                  height: 80,
                  borderRadius: '50%',
                  background: 'var(--status-error-bg)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  margin: '0 auto var(--spacing-lg)',
                }}
              >
                <Video size={36} style={{ color: 'var(--status-error)' }} />
              </div>
              <p style={{ color: 'var(--text-tertiary)', marginBottom: 'var(--spacing-md)' }}>未加载视频</p>
              <button className="btn btn-primary" onClick={handleOpenVideo}>
                <FolderOpen size={16} />
                选择视频
              </button>
            </div>
          </div>

          {/* Video Controls */}
          <div
            style={{
              height: 48,
              background: 'var(--bg-elevated)',
              borderTop: '1px solid var(--border-default)',
              display: 'flex',
              alignItems: 'center',
              padding: '0 var(--spacing-lg)',
              gap: 'var(--spacing-md)',
            }}
          >
            {/* Progress */}
            <div style={{ flex: 1, display: 'flex', alignItems: 'center', gap: 'var(--spacing-md)' }}>
              <span style={{ fontSize: 12, color: 'var(--text-tertiary)', width: 48 }}>
                {formatTime(currentTime)} / {formatTime(duration)}
              </span>
              <div style={{ flex: 1, height: 4, background: 'var(--border-default)', borderRadius: 'var(--radius-full)' }}>
                <div
                  style={{
                    height: '100%',
                    width: duration > 0 ? `${(currentTime / duration) * 100}%` : '0%',
                    background: 'var(--accent-primary)',
                    borderRadius: 'var(--radius-full)',
                  }}
                />
              </div>
            </div>

            {/* Controls */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
              <button
                style={{
                  width: 32,
                  height: 32,
                  borderRadius: 'var(--radius-md)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: 'var(--text-secondary)',
                  background: 'transparent',
                  border: 'none',
                  cursor: 'pointer',
                }}
              >
                <SkipBack size={16} />
              </button>
              <button
                onClick={togglePlay}
                style={{
                  width: 40,
                  height: 40,
                  borderRadius: '50%',
                  background: 'var(--accent-primary)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: 'white',
                  border: 'none',
                  cursor: 'pointer',
                }}
              >
                {isPlaying ? <Pause size={18} /> : <Play size={18} />}
              </button>
              <button
                style={{
                  width: 32,
                  height: 32,
                  borderRadius: 'var(--radius-md)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: 'var(--text-secondary)',
                  background: 'transparent',
                  border: 'none',
                  cursor: 'pointer',
                }}
              >
                <SkipForward size={16} />
              </button>
              <button
                onClick={() => setIsMuted(!isMuted)}
                style={{
                  width: 32,
                  height: 32,
                  borderRadius: 'var(--radius-md)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: 'var(--text-secondary)',
                  background: 'transparent',
                  border: 'none',
                  cursor: 'pointer',
                }}
              >
                {isMuted ? <VolumeX size={16} /> : <Volume2 size={16} />}
              </button>
            </div>
          </div>
        </div>

        {/* Right Panel - Inference Config */}
        <div className="right-panel">
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
                <span style={{ fontSize: 13, color: 'var(--text-primary)' }}>开启截图</span>
              </label>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}>截图数</label>
                <input
                  type="number"
                  className="input"
                  value={screenshotCount}
                  onChange={(e) => setScreenshotCount(parseInt(e.target.value) || 10)}
                  style={{ width: '100%' }}
                />
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}>模式</label>
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
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}>时间间隔 (ms)</label>
                <input
                  type="number"
                  className="input"
                  value={intervalMs}
                  onChange={(e) => setIntervalMs(parseInt(e.target.value) || 50)}
                  style={{ width: '100%' }}
                />
              </div>
            </div>
          </div>

          <div className="panel-section">
            <div className="panel-section-title">
              <Settings2 size={14} />
              推理设置
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
              <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)', cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={isInferenceEnabled}
                  onChange={(e) => setIsInferenceEnabled(e.target.checked)}
                  className="checkbox"
                />
                <span style={{ fontSize: 13, color: 'var(--text-primary)' }}>开启推理</span>
              </label>
              <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)', cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={gpuAccel}
                  onChange={(e) => setGpuAccel(e.target.checked)}
                  className="checkbox"
                />
                <span style={{ fontSize: 13, color: 'var(--text-primary)' }}>GPU加速</span>
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
          </div>

          <div className="panel-section">
            <div className="panel-section-title">
              <Gauge size={14} />
              高级
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
              <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)', cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={handDetection}
                  onChange={(e) => setHandDetection(e.target.checked)}
                  className="checkbox"
                />
                <span style={{ fontSize: 13, color: 'var(--text-primary)' }}>手关节检测</span>
              </label>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
