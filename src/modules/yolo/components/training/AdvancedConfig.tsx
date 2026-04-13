import { Settings2, ChevronDown, ChevronUp, RefreshCw } from 'lucide-react';
import { TrainingConfig } from '../../../../core/stores/trainingStore';
import styles from '../../pages/TrainingPage.module.css';

interface AdvancedConfigProps {
  config: TrainingConfig;
  showAdvanced: boolean;
  cudaAvailable: boolean;
  onToggleAdvanced: () => void;
  onConfigChange: (config: TrainingConfig) => void;
  onRefreshCUDA: () => void;
}

export default function AdvancedConfig({
  config,
  showAdvanced,
  cudaAvailable,
  onToggleAdvanced,
  onConfigChange,
  onRefreshCUDA,
}: AdvancedConfigProps) {
  return (
    <>
      {/* Data Augmentation Section */}
      <div className={styles.panelSection}>
        <div className={styles.panelSectionTitle} onClick={onToggleAdvanced} style={{ cursor: 'pointer' }}>
          <Settings2 size={14} />
          数据增强
          {showAdvanced ? <ChevronUp size={14} style={{ marginLeft: 'auto' }} /> : <ChevronDown size={14} style={{ marginLeft: 'auto' }} />}
        </div>
        {showAdvanced && (
          <div className={styles.panelSectionContent}>
            <SliderField label="HSV色调" value={config.hsv_h} min={0} max={1} step={0.05}
              onChange={(v) => onConfigChange({ ...config, hsv_h: v })} />
            <SliderField label="HSV饱和度" value={config.hsv_s} min={0} max={1} step={0.05}
              onChange={(v) => onConfigChange({ ...config, hsv_s: v })} />
            <SliderField label="HSV亮度" value={config.hsv_v} min={0} max={1} step={0.05}
              onChange={(v) => onConfigChange({ ...config, hsv_v: v })} />
            <SliderField label="平移" value={config.translate} min={0} max={0.5} step={0.05}
              onChange={(v) => onConfigChange({ ...config, translate: v })} />
            <SliderField label="缩放" value={config.scale} min={0} max={1} step={0.05}
              onChange={(v) => onConfigChange({ ...config, scale: v })} />
            <SliderField label="翻转概率" value={config.fliplr} min={0} max={1} step={0.1}
              onChange={(v) => onConfigChange({ ...config, fliplr: v })} />
            <SliderField label="Mosaic" value={config.mosaic} min={0} max={1} step={0.1}
              onChange={(v) => onConfigChange({ ...config, mosaic: v })} />
            <SliderField label="MixUp" value={config.mixup} min={0} max={1} step={0.1}
              onChange={(v) => onConfigChange({ ...config, mixup: v })} />
          </div>
        )}
      </div>

      {/* Optimizer Section */}
      <div className={styles.panelSection}>
        <div className={styles.panelSectionTitle}>
          <Settings2 size={14} />
          优化器
        </div>
        <div className={styles.panelSectionContent}>
          <div>
            <label className={styles.inlineLabel}>优化器</label>
            <select className={styles.select} value={config.optimizer}
              onChange={(e) => onConfigChange({ ...config, optimizer: e.target.value as 'SGD' | 'Adam' | 'AdamW' })}
              style={{ width: '100%', marginTop: 4 }}>
              <option value="SGD">SGD</option>
              <option value="Adam">Adam</option>
              <option value="AdamW">AdamW</option>
            </select>
          </div>
          <NumberField label="初始学习率" value={config.lr0} step={0.001}
            onChange={(v) => onConfigChange({ ...config, lr0: v })} />
          <NumberField label="最终学习率因子" value={config.lrf} step={0.001}
            onChange={(v) => onConfigChange({ ...config, lrf: v })} />
          <NumberField label="动量" value={config.momentum} step={0.001}
            onChange={(v) => onConfigChange({ ...config, momentum: v })} />
          <NumberField label="权重衰减" value={config.weight_decay} step={0.0001}
            onChange={(v) => onConfigChange({ ...config, weight_decay: v })} />
        </div>
      </div>

      {/* Advanced Settings Section */}
      <div className={styles.panelSection}>
        <div className={styles.panelSectionTitle}>
          <Settings2 size={14} />
          高级设置
        </div>
        <div className={styles.panelSectionContent}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
            <input type="checkbox" id="amp" checked={config.amp}
              onChange={(e) => onConfigChange({ ...config, amp: e.target.checked })} className={styles.checkbox} />
            <label htmlFor="amp" className={styles.checkboxLabel}>混合精度 (AMP)</label>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
            <input type="checkbox" id="cos_lr" checked={config.cos_lr}
              onChange={(e) => onConfigChange({ ...config, cos_lr: e.target.checked })} className={styles.checkbox} />
            <label htmlFor="cos_lr" className={styles.checkboxLabel}>余弦学习率</label>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
            <input type="checkbox" id="rect" checked={config.rect}
              onChange={(e) => onConfigChange({ ...config, rect: e.target.checked })} className={styles.checkbox} />
            <label htmlFor="rect" className={styles.checkboxLabel}>矩形训练</label>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
            <input type="checkbox" id="cache" checked={config.cache}
              onChange={(e) => onConfigChange({ ...config, cache: e.target.checked })} className={styles.checkbox} />
            <label htmlFor="cache" className={styles.checkboxLabel}>缓存图像</label>
          </div>
          <NumberField label="早停耐心" value={config.patience}
            onChange={(v) => onConfigChange({ ...config, patience: v })} />
          <NumberField label="Mosaic关闭轮次" value={config.close_mosaic}
            onChange={(v) => onConfigChange({ ...config, close_mosaic: v })} />
        </div>
      </div>
    </>
  );
}

// Helper components
interface SliderFieldProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
}

function SliderField({ label, value, min, max, step, onChange }: SliderFieldProps) {
  return (
    <div>
      <label className={styles.inlineLabel}>{label}</label>
      <input type="range" min={min} max={max} step={step} value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))} className={styles.slider} />
      <span className={styles.sliderValue}>{value.toFixed(2)}</span>
    </div>
  );
}

interface NumberFieldProps {
  label: string;
  value: number;
  step?: number;
  onChange: (value: number) => void;
}

function NumberField({ label, value, step, onChange }: NumberFieldProps) {
  return (
    <div>
      <label className={styles.inlineLabel}>{label}</label>
      <input type="number" className={styles.input} value={value === 0 ? '' : value}
        onChange={(e) => {
          if (e.target.value === '') onChange(0);
          else {
            const num = parseFloat(e.target.value);
            if (!isNaN(num)) onChange(num);
          }
        }}
        step={step} style={{ marginTop: 4 }} />
    </div>
  );
}
