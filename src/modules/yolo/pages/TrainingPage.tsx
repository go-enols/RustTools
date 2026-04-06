import { useState, useEffect, useRef } from 'react';
import {
  Play,
  Square,
  Settings2,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { useTrainingStore, TrainingConfig, TrainingMetrics } from '../../../core/stores/trainingStore';
import * as echarts from 'echarts';

const defaultConfig: TrainingConfig = {
  base_model: 'yolo11s.pt',
  epochs: 50,
  patience: 50,
  batch_size: 12,
  image_size: 640,
  device_id: 0,
  workers: 8,
  optimizer: 'SGD',
  lr0: 0.01,
  lrf: 0.01,
  momentum: 0.937,
  weight_decay: 0.0005,
  warmup_epochs: 3.0,
  warmup_bias_lr: 0.1,
  warmup_momentum: 0.8,
  hsv_h: 0.25,
  hsv_s: 0.25,
  hsv_v: 0.25,
  translate: 0.1,
  scale: 0.5,
  shear: 0.0,
  perspective: 0.0,
  flipud: 0.0,
  fliplr: 0.5,
  mosaic: 1.0,
  mixup: 0.0,
  copy_paste: 0.0,
  close_mosaic: 10,
  rect: false,
  cos_lr: false,
  single_cls: false,
  amp: true,
  save_period: -1,
  cache: false,
};

// Chart configuration for 10 charts in 2 rows of 5
const chartConfigs = [
  { title: 'train/box_loss', metricKey: 'trainBoxLoss', color: '#52c41a' },
  { title: 'train/cls_loss', metricKey: 'trainClsLoss', color: '#1677ff' },
  { title: 'train/dfl_loss', metricKey: 'trainDflLoss', color: '#faad14' },
  { title: 'metrics/precision(B)', metricKey: 'precision', color: '#722ed1' },
  { title: 'metrics/recall(B)', metricKey: 'recall', color: '#ff4d4f' },
  { title: 'val/box_loss', metricKey: 'valBoxLoss', color: '#52c41a' },
  { title: 'val/cls_loss', metricKey: 'valClsLoss', color: '#1677ff' },
  { title: 'val/dfl_loss', metricKey: 'valDflLoss', color: '#faad14' },
  { title: 'metrics/mAP50(B)', metricKey: 'map50', color: '#722ed1' },
  { title: 'metrics/mAP50-95(B)', metricKey: 'map50_95', color: '#ff4d4f' },
];

export default function TrainingPage() {
  const { isTraining, currentEpoch, totalEpochs, metrics, startTraining, stopTraining, updateMetrics } = useTrainingStore();
  const [config, setConfig] = useState<TrainingConfig>(defaultConfig);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [elapsedTime, setElapsedTime] = useState('00:00:00');
  const [remainingTime, setRemainingTime] = useState('--:--:--');

  // Create 10 separate refs for 10 charts
  const chartRef0 = useRef<HTMLDivElement>(null);
  const chartRef1 = useRef<HTMLDivElement>(null);
  const chartRef2 = useRef<HTMLDivElement>(null);
  const chartRef3 = useRef<HTMLDivElement>(null);
  const chartRef4 = useRef<HTMLDivElement>(null);
  const chartRef5 = useRef<HTMLDivElement>(null);
  const chartRef6 = useRef<HTMLDivElement>(null);
  const chartRef7 = useRef<HTMLDivElement>(null);
  const chartRef8 = useRef<HTMLDivElement>(null);
  const chartRef9 = useRef<HTMLDivElement>(null);

  const chartRefs = [chartRef0, chartRef1, chartRef2, chartRef3, chartRef4, chartRef5, chartRef6, chartRef7, chartRef8, chartRef9];
  const chartInstances = useRef<echarts.ECharts[]>([]);

  // Initialize charts
  useEffect(() => {
    // Create chart instances for each ref
    chartConfigs.forEach((cfg, index) => {
      const dom = chartRefs[index].current;
      if (!dom) return;

      const chart = echarts.init(dom);
      chartInstances.current[index] = chart;

      const option = {
        backgroundColor: 'transparent',
        grid: { top: 30, right: 20, bottom: 30, left: 50 },
        xAxis: {
          type: 'category' as const,
          data: [] as number[],
          axisLine: { lineStyle: { color: '#303030' } },
          axisLabel: { color: '#6d6d6d', fontSize: 10 },
          splitLine: { show: false }
        },
        yAxis: {
          type: 'value' as const,
          axisLine: { lineStyle: { color: '#303030' } },
          axisLabel: { color: '#6d6d6d', fontSize: 10 },
          splitLine: { lineStyle: { color: '#3a3a3a' } }
        },
        series: [{
          name: cfg.title,
          type: 'line' as const,
          data: [] as number[],
          smooth: true,
          lineStyle: { width: 2 },
          itemStyle: { color: cfg.color },
          symbol: 'circle',
          symbolSize: 4,
        }],
        legend: { textStyle: { color: '#b0b0b0', fontSize: 11 }, top: 0 },
        tooltip: { trigger: 'axis' as const, axisPointer: { type: 'cross' as const } },
      };

      chart.setOption(option);
    });

    // Handle resize
    const handleResize = () => {
      chartInstances.current.forEach((chart) => {
        if (chart) chart.resize();
      });
    };
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      chartInstances.current.forEach((chart) => {
        if (chart) chart.dispose();
      });
    };
  }, []);

  // Update charts when metrics change
  useEffect(() => {
    if (metrics.length === 0) return;

    const epochs = metrics.map((m) => m.epoch);

    chartConfigs.forEach((cfg, index) => {
      const chart = chartInstances.current[index];
      if (!chart) return;

      chart.setOption({
        xAxis: { data: epochs },
        series: [{
          data: metrics.map((m) => m[cfg.metricKey as keyof TrainingMetrics] as number),
        }],
      });
    });
  }, [metrics]);

  // Simulate training updates
  useEffect(() => {
    if (!isTraining) return;

    const interval = setInterval(() => {
      const newMetrics: TrainingMetrics = {
        epoch: currentEpoch + 1,
        trainBoxLoss: Math.random() * 0.5 + 0.1,
        trainClsLoss: Math.random() * 0.3 + 0.05,
        trainDflLoss: Math.random() * 0.2 + 0.02,
        valBoxLoss: Math.random() * 0.4 + 0.1,
        valClsLoss: Math.random() * 0.25 + 0.05,
        valDflLoss: Math.random() * 0.15 + 0.02,
        precision: Math.random() * 0.2 + 0.7,
        recall: Math.random() * 0.15 + 0.75,
        map50: Math.random() * 0.1 + 0.75,
        map50_95: Math.random() * 0.08 + 0.55,
      };
      updateMetrics(newMetrics);

      // Update time
      const elapsed = new Date();
      const start = useTrainingStore.getState().startTime;
      if (start) {
        const diff = Math.floor((elapsed.getTime() - start.getTime()) / 1000);
        const hours = Math.floor(diff / 3600);
        const mins = Math.floor((diff % 3600) / 60);
        const secs = diff % 60;
        setElapsedTime(`${hours.toString().padStart(2, '0')}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`);

        if (newMetrics.epoch > 0) {
          const avgTimePerEpoch = diff / newMetrics.epoch;
          const remaining = Math.floor(avgTimePerEpoch * (totalEpochs - newMetrics.epoch));
          const remHours = Math.floor(remaining / 3600);
          const remMins = Math.floor((remaining % 3600) / 60);
          const remSecs = remaining % 60;
          setRemainingTime(`${remHours.toString().padStart(2, '0')}:${remMins.toString().padStart(2, '0')}:${remSecs.toString().padStart(2, '0')}`);
        }
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [isTraining, currentEpoch, totalEpochs, updateMetrics]);

  const handleStartTraining = () => {
    startTraining(config);
  };

  const progress = totalEpochs > 0 ? (currentEpoch / totalEpochs) * 100 : 0;

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      {/* Header */}
      <div className="content-header">
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div>
            <h1 className="text-lg font-semibold">模型训练</h1>
            <p className="text-sm text-tertiary mt-sm">训练YOLO目标检测模型</p>
          </div>
          <div style={{ display: 'flex', gap: 'var(--spacing-md)' }}>
            {!isTraining ? (
              <button className="btn btn-primary" onClick={handleStartTraining}>
                <Play size={16} />
                开始训练
              </button>
            ) : (
              <button className="btn btn-danger" onClick={stopTraining}>
                <Square size={16} />
                停止训练
              </button>
            )}
          </div>
        </div>
      </div>

      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        <div style={{ flex: 1, overflow: 'auto', padding: 'var(--spacing-lg)' }}>
        {/* Training Controls */}
        <div className="training-controls" style={{ marginBottom: 'var(--spacing-lg)' }}>
          <div style={{ flex: 1, display: 'flex', alignItems: 'center', gap: 'var(--spacing-xl)' }}>
            <div>
              <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>基础模型</span>
              <select
                className="select"
                value={config.base_model}
                onChange={(e) => setConfig({ ...config, base_model: e.target.value })}
                style={{ marginLeft: 8 }}
              >
                <option value="yolo11n.pt">YOLO11n</option>
                <option value="yolo11s.pt">YOLO11s</option>
                <option value="yolo11m.pt">YOLO11m</option>
                <option value="yolov8n.pt">YOLOv8n</option>
                <option value="yolov8s.pt">YOLOv8s</option>
              </select>
            </div>
            <div>
              <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>训练轮次</span>
              <input
                type="number"
                className="input"
                value={config.epochs}
                onChange={(e) => setConfig({ ...config, epochs: parseInt(e.target.value) || 50 })}
                style={{ width: 80, marginLeft: 8, textAlign: 'center' }}
              />
            </div>
            <div>
              <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>批处理</span>
              <input
                type="number"
                className="input"
                value={config.batch_size}
                onChange={(e) => setConfig({ ...config, batch_size: parseInt(e.target.value) || 12 })}
                style={{ width: 60, marginLeft: 8, textAlign: 'center' }}
              />
            </div>
          </div>
        </div>

        {/* Progress */}
        {isTraining && (
          <div className="card" style={{ marginBottom: 'var(--spacing-lg)' }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 'var(--spacing-md)' }}>
              <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>
                训练进度: {currentEpoch} / {totalEpochs} Epochs ({progress.toFixed(1)}%)
              </span>
              <span style={{ fontSize: 14, color: 'var(--accent-primary)' }}>{progress.toFixed(1)}%</span>
            </div>
            <div className="progress-bar" style={{ height: 8 }}>
              <div className="progress-fill" style={{ width: `${progress}%` }} />
            </div>
            <div className="training-stats" style={{ marginTop: 'var(--spacing-md)' }}>
              <span>已用时间: <strong>{elapsedTime}</strong></span>
              <span>预计剩余: <strong>{remainingTime}</strong></span>
              {metrics.length > 0 && (
                <>
                  <span>mAP50: <strong>{(metrics[metrics.length - 1].map50 * 100).toFixed(1)}%</strong></span>
                  <span>Precision: <strong>{(metrics[metrics.length - 1].precision * 100).toFixed(1)}%</strong></span>
                </>
              )}
            </div>
          </div>
        )}

        {/* Charts Grid - Row 1 */}
        <div className="chart-grid" style={{ gridTemplateColumns: 'repeat(5, 1fr)', marginBottom: 'var(--spacing-md)' }}>
          {chartConfigs.slice(0, 5).map((cfg, index) => (
            <div key={cfg.title} className="chart-container">
              <div className="chart-title">{cfg.title}</div>
              <div ref={chartRefs[index]} className="chart-wrapper" />
            </div>
          ))}
        </div>

        {/* Charts Grid - Row 2 */}
        <div className="chart-grid" style={{ gridTemplateColumns: 'repeat(5, 1fr)', marginBottom: 'var(--spacing-lg)' }}>
          {chartConfigs.slice(5, 10).map((cfg, index) => (
            <div key={cfg.title} className="chart-container">
              <div className="chart-title">{cfg.title}</div>
              <div ref={chartRefs[index + 5]} className="chart-wrapper" />
            </div>
          ))}
        </div>

        {/* Log Panel */}
        <div className="card">
          <div className="card-header">
            <span className="card-title">训练日志</span>
          </div>
          <div className="log-panel">
            {metrics.length === 0 ? (
              <div className="log-entry">等待开始训练...</div>
            ) : (
              metrics.slice(-10).map((m, i) => (
                <div key={i} className="log-entry">
                  Epoch {m.epoch}: box_loss={m.trainBoxLoss.toFixed(4)}, cls_loss={m.trainClsLoss.toFixed(4)}, mAP50={m.map50.toFixed(4)}, precision={m.precision.toFixed(4)}, recall={m.recall.toFixed(4)}
                </div>
              ))
            )}
          </div>
        </div>
      </div>

      {/* Right Panel */}
      <div className="right-panel" style={{ width: 280, overflow: 'auto', borderLeft: '1px solid var(--border-default)', background: 'var(--bg-surface)' }}>
        <div className="panel-section">
          <div className="panel-section-title">
            <Settings2 size={14} />
            基础参数
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
            <div>
              <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>图像大小</label>
              <input
                type="number"
                className="input"
                value={config.image_size}
                onChange={(e) => setConfig({ ...config, image_size: parseInt(e.target.value) || 640 })}
                style={{ marginTop: 4 }}
              />
            </div>
            <div>
              <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>设备ID</label>
              <select
                className="select"
                value={config.device_id}
                onChange={(e) => setConfig({ ...config, device_id: parseInt(e.target.value) })}
                style={{ width: '100%', marginTop: 4 }}
              >
                <option value={0}>GPU 0</option>
                <option value={1}>GPU 1</option>
              </select>
            </div>
            <div>
              <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>Workers</label>
              <input
                type="number"
                className="input"
                value={config.workers}
                onChange={(e) => setConfig({ ...config, workers: parseInt(e.target.value) || 8 })}
                style={{ marginTop: 4 }}
              />
            </div>
          </div>
        </div>

        <div className="panel-section">
          <div
            className="panel-section-title"
            style={{ cursor: 'pointer' }}
            onClick={() => setShowAdvanced(!showAdvanced)}
          >
            <Settings2 size={14} />
            数据增强
            {showAdvanced ? <ChevronUp size={14} style={{ marginLeft: 'auto' }} /> : <ChevronDown size={14} style={{ marginLeft: 'auto' }} />}
          </div>
          {showAdvanced && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>HSV色调</label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={config.hsv_h}
                  onChange={(e) => setConfig({ ...config, hsv_h: parseFloat(e.target.value) })}
                  className="slider"
                />
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.hsv_h.toFixed(2)}</span>
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>HSV饱和度</label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={config.hsv_s}
                  onChange={(e) => setConfig({ ...config, hsv_s: parseFloat(e.target.value) })}
                  className="slider"
                />
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.hsv_s.toFixed(2)}</span>
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>HSV亮度</label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={config.hsv_v}
                  onChange={(e) => setConfig({ ...config, hsv_v: parseFloat(e.target.value) })}
                  className="slider"
                />
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.hsv_v.toFixed(2)}</span>
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>平移</label>
                <input
                  type="range"
                  min="0"
                  max="0.5"
                  step="0.05"
                  value={config.translate}
                  onChange={(e) => setConfig({ ...config, translate: parseFloat(e.target.value) })}
                  className="slider"
                />
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.translate.toFixed(2)}</span>
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>缩放</label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={config.scale}
                  onChange={(e) => setConfig({ ...config, scale: parseFloat(e.target.value) })}
                  className="slider"
                />
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.scale.toFixed(2)}</span>
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>翻转概率</label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.1"
                  value={config.fliplr}
                  onChange={(e) => setConfig({ ...config, fliplr: parseFloat(e.target.value) })}
                  className="slider"
                />
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.fliplr.toFixed(1)}</span>
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>Mosaic</label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.1"
                  value={config.mosaic}
                  onChange={(e) => setConfig({ ...config, mosaic: parseFloat(e.target.value) })}
                  className="slider"
                />
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.mosaic.toFixed(1)}</span>
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>MixUp</label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.1"
                  value={config.mixup}
                  onChange={(e) => setConfig({ ...config, mixup: parseFloat(e.target.value) })}
                  className="slider"
                />
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{config.mixup.toFixed(1)}</span>
              </div>
            </div>
          )}
        </div>

        {/* Optimizer Parameters */}
        <div className="panel-section">
          <div className="panel-section-title">
            <Settings2 size={14} />
            优化器
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
            <div>
              <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>优化器</label>
              <select
                className="select"
                value={config.optimizer}
                onChange={(e) => setConfig({ ...config, optimizer: e.target.value as 'SGD' | 'Adam' | 'AdamW' })}
                style={{ width: '100%', marginTop: 4 }}
              >
                <option value="SGD">SGD</option>
                <option value="Adam">Adam</option>
                <option value="AdamW">AdamW</option>
              </select>
            </div>
            <div>
              <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>初始学习率</label>
              <input
                type="number"
                className="input"
                value={config.lr0}
                onChange={(e) => setConfig({ ...config, lr0: parseFloat(e.target.value) || 0.01 })}
                step="0.001"
                style={{ marginTop: 4 }}
              />
            </div>
            <div>
              <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>最终学习率因子</label>
              <input
                type="number"
                className="input"
                value={config.lrf}
                onChange={(e) => setConfig({ ...config, lrf: parseFloat(e.target.value) || 0.01 })}
                step="0.001"
                style={{ marginTop: 4 }}
              />
            </div>
            <div>
              <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>动量</label>
              <input
                type="number"
                className="input"
                value={config.momentum}
                onChange={(e) => setConfig({ ...config, momentum: parseFloat(e.target.value) || 0.937 })}
                step="0.001"
                style={{ marginTop: 4 }}
              />
            </div>
            <div>
              <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>权重衰减</label>
              <input
                type="number"
                className="input"
                value={config.weight_decay}
                onChange={(e) => setConfig({ ...config, weight_decay: parseFloat(e.target.value) || 0.0005 })}
                step="0.0001"
                style={{ marginTop: 4 }}
              />
            </div>
          </div>
        </div>

        {/* Advanced Settings */}
        <div className="panel-section">
          <div className="panel-section-title">
            <Settings2 size={14} />
            高级设置
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-md)' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
              <input
                type="checkbox"
                id="amp"
                checked={config.amp}
                onChange={(e) => setConfig({ ...config, amp: e.target.checked })}
              />
              <label htmlFor="amp" style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>混合精度 (AMP)</label>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
              <input
                type="checkbox"
                id="cos_lr"
                checked={config.cos_lr}
                onChange={(e) => setConfig({ ...config, cos_lr: e.target.checked })}
              />
              <label htmlFor="cos_lr" style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>余弦学习率</label>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
              <input
                type="checkbox"
                id="rect"
                checked={config.rect}
                onChange={(e) => setConfig({ ...config, rect: e.target.checked })}
              />
              <label htmlFor="rect" style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>矩形训练</label>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
              <input
                type="checkbox"
                id="cache"
                checked={config.cache}
                onChange={(e) => setConfig({ ...config, cache: e.target.checked })}
              />
              <label htmlFor="cache" style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>缓存图像</label>
            </div>
            <div>
              <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>早停耐心</label>
              <input
                type="number"
                className="input"
                value={config.patience}
                onChange={(e) => setConfig({ ...config, patience: parseInt(e.target.value) || 50 })}
                style={{ marginTop: 4 }}
              />
            </div>
            <div>
              <label style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>Mosaic关闭轮次</label>
              <input
                type="number"
                className="input"
                value={config.close_mosaic}
                onChange={(e) => setConfig({ ...config, close_mosaic: parseInt(e.target.value) || 10 })}
                style={{ marginTop: 4 }}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
    </div>
  );
}
