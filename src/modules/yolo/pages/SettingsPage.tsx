import { useEffect } from 'react';
import {
  Settings as SettingsIcon,
  Monitor,
  FolderOpen,
  Moon,
  Sun,
  Save,
  RotateCcw,
} from 'lucide-react';
import { useSettingsStore } from '../../../core/stores/settingsStore';

export default function SettingsPage() {
  const {
    settings,
    isLoading,
    isSaving,
    error,
    loadSettings,
    saveSettings,
    updateSetting,
    resetToDefaults,
  } = useSettingsStore();

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  const handleSave = async () => {
    await saveSettings(settings);
  };

  if (isLoading) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <span style={{ color: 'var(--text-tertiary)' }}>加载中...</span>
      </div>
    );
  }

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div className="content-header">
        <h1 className="text-lg font-semibold">设置</h1>
        <p className="text-sm text-tertiary mt-sm">配置应用程序首选项</p>
      </div>

      <div className="content-body" style={{ maxWidth: 640 }}>
        {error && (
          <div
            style={{
              padding: 'var(--spacing-md)',
              background: 'var(--status-error-bg)',
              border: '1px solid var(--status-error)',
              borderRadius: 'var(--radius-md)',
              color: 'var(--status-error)',
              fontSize: 13,
              marginBottom: 'var(--spacing-lg)',
            }}
          >
            {error}
          </div>
        )}

        {/* App Settings */}
        <div className="card" style={{ marginBottom: 'var(--spacing-lg)' }}>
          <div className="card-title" style={{ marginBottom: 'var(--spacing-lg)' }}>
            <SettingsIcon size={16} />
            应用设置
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-lg)' }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>主题</span>
              <div style={{ display: 'flex', gap: 'var(--spacing-sm)' }}>
                <button
                  className={`btn ${settings.theme === 'dark' ? 'btn-primary' : 'btn-secondary'}`}
                  onClick={() => updateSetting('theme', 'dark')}
                  style={{ padding: '4px 12px' }}
                >
                  <Moon size={14} />
                </button>
                <button
                  className={`btn ${settings.theme === 'light' ? 'btn-primary' : 'btn-secondary'}`}
                  onClick={() => updateSetting('theme', 'light')}
                  style={{ padding: '4px 12px' }}
                >
                  <Sun size={14} />
                </button>
              </div>
            </div>

            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>语言</span>
              <select
                value={settings.language}
                onChange={(e) => updateSetting('language', e.target.value as 'zh' | 'en')}
                className="select"
                style={{ width: 160 }}
              >
                <option value="zh">简体中文</option>
                <option value="en">English</option>
              </select>
            </div>

            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>自动保存</span>
              <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-md)' }}>
                <input
                  type="range"
                  min={1}
                  max={10}
                  value={settings.autoSaveMinutes}
                  onChange={(e) => updateSetting('autoSaveMinutes', Number(e.target.value))}
                  className="slider"
                  style={{ width: 100 }}
                />
                <span style={{ fontSize: 13, color: 'var(--text-tertiary)', width: 80 }}>
                  {settings.autoSaveMinutes} 分钟
                </span>
              </div>
            </div>

            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
              <input
                type="checkbox"
                checked={settings.openRecentOnStartup}
                onChange={(e) => updateSetting('openRecentOnStartup', e.target.checked)}
                className="checkbox"
              />
              <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>启动时打开最近项目</span>
            </div>

            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
              <input
                type="checkbox"
                checked={settings.animationsEnabled}
                onChange={(e) => updateSetting('animationsEnabled', e.target.checked)}
                className="checkbox"
              />
              <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>启用动画效果</span>
            </div>
          </div>
        </div>

        {/* Training Settings */}
        <div className="card" style={{ marginBottom: 'var(--spacing-lg)' }}>
          <div className="card-title" style={{ marginBottom: 'var(--spacing-lg)' }}>
            <Monitor size={16} />
            训练设置
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-lg)' }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>默认设备</span>
              <select
                value={settings.defaultDevice}
                onChange={(e) => updateSetting('defaultDevice', e.target.value)}
                className="select"
                style={{ width: 160 }}
              >
                <option value="GPU 0">GPU 0</option>
                <option value="GPU 1">GPU 1</option>
                <option value="CPU">CPU</option>
              </select>
            </div>

            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>备用设备</span>
              <select
                value={settings.fallbackDevice}
                onChange={(e) => updateSetting('fallbackDevice', e.target.value)}
                className="select"
                style={{ width: 160 }}
              >
                <option value="CPU">CPU</option>
                <option value="GPU 0">GPU 0</option>
              </select>
            </div>

            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>显存限制</span>
              <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
                <input
                  type="number"
                  className="input"
                  value={settings.vramLimitGb}
                  onChange={(e) => updateSetting('vramLimitGb', Number(e.target.value))}
                  style={{ width: 80, textAlign: 'center' }}
                />
                <span style={{ fontSize: 13, color: 'var(--text-tertiary)' }}>GB</span>
              </div>
            </div>

            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>Workers</span>
              <input
                type="number"
                className="input"
                value={settings.workers}
                onChange={(e) => updateSetting('workers', Number(e.target.value))}
                style={{ width: 80, textAlign: 'center' }}
              />
            </div>
          </div>
        </div>

        {/* Paths */}
        <div className="card" style={{ marginBottom: 'var(--spacing-lg)' }}>
          <div className="card-title" style={{ marginBottom: 'var(--spacing-lg)' }}>
            <FolderOpen size={16} />
            数据路径
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-lg)' }}>
            <div>
              <label style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}>
                数据集目录
              </label>
              <input
                type="text"
                className="input"
                value={settings.datasetPath}
                onChange={(e) => updateSetting('datasetPath', e.target.value)}
              />
            </div>
            <div>
              <label style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}>
                模型存储
              </label>
              <input
                type="text"
                className="input"
                value={settings.modelPath}
                onChange={(e) => updateSetting('modelPath', e.target.value)}
              />
            </div>
            <div>
              <label style={{ fontSize: 12, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4 }}>
                缓存目录
              </label>
              <input
                type="text"
                className="input"
                value={settings.cachePath}
                onChange={(e) => updateSetting('cachePath', e.target.value)}
              />
            </div>
          </div>
        </div>

        {/* Actions */}
        <div style={{ display: 'flex', gap: 'var(--spacing-md)' }}>
          <button
            className="btn btn-primary"
            onClick={handleSave}
            disabled={isSaving}
          >
            <Save size={16} />
            {isSaving ? '保存中...' : '保存设置'}
          </button>
          <button
            className="btn btn-secondary"
            onClick={resetToDefaults}
          >
            <RotateCcw size={16} />
            恢复默认
          </button>
        </div>
      </div>
    </div>
  );
}
