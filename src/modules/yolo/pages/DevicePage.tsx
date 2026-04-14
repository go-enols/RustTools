import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useSettingsStore, DeviceInfo } from '../../../core/stores/settingsStore';

interface PythonEnvStatus {
  pythonAvailable: boolean;
  pythonVersion: string | null;
  torchAvailable: boolean;
  torchVersion: string | null;
  ultralyticsAvailable: boolean;
  ultralyticsVersion: string | null;
  cudaAvailable: boolean;
  ready_for_training: boolean;
  installing: boolean;
}

interface InstallProgress {
  stage: string;
  message: string;
  progress: number | null;
}

function PythonEnvCard() {
  const [envStatus, setEnvStatus] = useState<PythonEnvStatus | null>(null);
  const [progress, setProgress] = useState<InstallProgress | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    checkEnv();
    const unlistenProgress = listen<InstallProgress>('python-env-progress', (event) => {
      setProgress(event.payload);
    });
    const unlistenDone = listen<{ success: boolean; message: string }>('python-env-done', () => {
      setProgress(null);
      checkEnv();
    });
    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenDone.then((fn) => fn());
    };
  }, []);

  const checkEnv = async () => {
    setIsLoading(true);
    try {
      const status = await invoke<PythonEnvStatus>('python_env_status');
      setEnvStatus(status);
    } catch (e) {
      console.error('Failed to check Python env:', e);
    } finally {
      setIsLoading(false);
    }
  };

  const installEnv = async () => {
    setIsLoading(true);
    try {
      await invoke('python_env_install');
    } catch (e) {
      console.error('Failed to install env:', e);
    }
  };

  return (
    <div className="card" style={{ marginTop: 'var(--spacing-lg)' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 'var(--spacing-md)' }}>
        <h3 style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)' }}>Python 环境</h3>
        <button
          onClick={checkEnv}
          disabled={isLoading}
          style={{
            padding: '4px 12px',
            fontSize: 12,
            background: 'var(--accent-primary)',
            color: '#fff',
            border: 'none',
            borderRadius: 'var(--radius-sm)',
            cursor: isLoading ? 'not-allowed' : 'pointer',
            opacity: isLoading ? 0.6 : 1,
          }}
        >
          {isLoading ? '检查中...' : '检查'}
        </button>
      </div>

      {/* Badges */}
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, marginBottom: 'var(--spacing-md)' }}>
        <span style={{
          padding: '2px 8px',
          fontSize: 11,
          borderRadius: 4,
          background: envStatus?.pythonAvailable ? 'rgba(34, 197, 94, 0.2)' : 'rgba(239, 68, 68, 0.2)',
          color: envStatus?.pythonAvailable ? '#22c55e' : '#ef4444',
          border: `1px solid ${envStatus?.pythonAvailable ? 'rgba(34, 197, 94, 0.3)' : 'rgba(239, 68, 68, 0.3)'}`,
        }}>
          Python {envStatus?.pythonVersion || 'N/A'}
        </span>
        <span style={{
          padding: '2px 8px',
          fontSize: 11,
          borderRadius: 4,
          background: envStatus?.torchAvailable ? 'rgba(34, 197, 94, 0.2)' : 'rgba(239, 68, 68, 0.2)',
          color: envStatus?.torchAvailable ? '#22c55e' : '#ef4444',
          border: `1px solid ${envStatus?.torchAvailable ? 'rgba(34, 197, 94, 0.3)' : 'rgba(239, 68, 68, 0.3)'}`,
        }}>
          PyTorch {envStatus?.torchVersion || 'N/A'}
        </span>
        <span style={{
          padding: '2px 8px',
          fontSize: 11,
          borderRadius: 4,
          background: envStatus?.ultralyticsAvailable ? 'rgba(34, 197, 94, 0.2)' : 'rgba(239, 68, 68, 0.2)',
          color: envStatus?.ultralyticsAvailable ? '#22c55e' : '#ef4444',
          border: `1px solid ${envStatus?.ultralyticsAvailable ? 'rgba(34, 197, 94, 0.3)' : 'rgba(239, 68, 68, 0.3)'}`,
        }}>
          Ultralytics {envStatus?.ultralyticsVersion || 'N/A'}
        </span>
        <span style={{
          padding: '2px 8px',
          fontSize: 11,
          borderRadius: 4,
          background: envStatus?.cudaAvailable ? 'rgba(34, 197, 94, 0.2)' : 'rgba(239, 68, 68, 0.2)',
          color: envStatus?.cudaAvailable ? '#22c55e' : '#ef4444',
          border: `1px solid ${envStatus?.cudaAvailable ? 'rgba(34, 197, 94, 0.3)' : 'rgba(239, 68, 68, 0.3)'}`,
        }}>
          CUDA {envStatus?.cudaAvailable ? '可用' : '不可用'}
        </span>
      </div>

      {/* Progress bar */}
      {progress && (
        <div style={{ marginTop: 'var(--spacing-md)' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 12, marginBottom: 4 }}>
            <span style={{ color: 'var(--text-secondary)' }}>{progress.message}</span>
            <span style={{ color: 'var(--accent-primary)' }}>
              {progress.progress !== null ? `${Math.round(progress.progress * 100)}%` : ''}
            </span>
          </div>
          <div style={{ height: 4, background: 'var(--bg-elevated)', borderRadius: 2 }}>
            <div style={{
              height: '100%',
              width: progress.progress !== null ? `${progress.progress * 100}%` : '0%',
              background: 'var(--accent-primary)',
              borderRadius: 2,
              transition: 'width 0.3s',
            }} />
          </div>
        </div>
      )}

      {/* Install button */}
      {envStatus && !envStatus.ready_for_training && !envStatus.installing && (
        <button
          onClick={installEnv}
          style={{
            marginTop: 'var(--spacing-md)',
            padding: '8px 16px',
            fontSize: 13,
            background: 'var(--status-success)',
            color: '#fff',
            border: 'none',
            borderRadius: 'var(--radius-sm)',
            cursor: 'pointer',
          }}
        >
          安装环境
        </button>
      )}
    </div>
  );
}

export default function DevicePage() {
  const { devices, loadDevices } = useSettingsStore();
  const [selectedDeviceId, setSelectedDeviceId] = useState<number>(0);

  useEffect(() => {
    loadDevices();
  }, [loadDevices]);

  const selected = devices.find((d) => d.id === selectedDeviceId);

  const formatBytes = (bytes: number) => {
    const gb = bytes / (1024 * 1024 * 1024);
    return `${gb.toFixed(1)} GB`;
  };

  const getMemoryUtilization = (device: DeviceInfo) => {
    return Math.round((device.memory_used / device.memory_total) * 100);
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div className="content-header">
        <h1 className="text-lg font-semibold">设备管理</h1>
        <p className="text-sm text-tertiary mt-sm">查看和管理计算设备</p>
      </div>

      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        {/* Left - Device List */}
        <div style={{ width: 280, background: 'var(--bg-surface)', borderRight: '1px solid var(--border-default)', padding: 'var(--spacing-lg)' }}>
          <h3 style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 'var(--spacing-md)' }}>
            可用设备
          </h3>
          <div className="device-list">
            {devices.map((device) => (
              <div
                key={device.id}
                className={`device-card ${selectedDeviceId === device.id ? 'selected' : ''}`}
                onClick={() => setSelectedDeviceId(device.id)}
              >
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 4 }}>
                  <span className="device-name">{device.name}</span>
                  <span className={`badge ${device.type === 'GPU' ? 'badge-blue' : 'badge-green'}`}>
                    {device.type}
                  </span>
                </div>
                <div className="device-info">
                  {formatBytes(device.memory_total)} · 利用率 {getMemoryUtilization(device)}%
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Right - Device Details */}
        <div style={{ flex: 1, overflow: 'auto', padding: 'var(--spacing-xl)' }}>
          {/* Python Environment — always visible, independent of device selection */}
          <PythonEnvCard />

          {selected && (
            <div style={{ maxWidth: 600 }}>
              {selected.type === 'GPU' ? (
                <GPUDetail device={selected} formatBytes={formatBytes} getMemoryUtilization={getMemoryUtilization} />
              ) : (
                <CPUDetail device={selected} formatBytes={formatBytes} getMemoryUtilization={getMemoryUtilization} />
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

interface GPUDetailProps {
  device: DeviceInfo;
  formatBytes: (bytes: number) => string;
  getMemoryUtilization: (device: DeviceInfo) => number;
}

function GPUDetail({ device, formatBytes, getMemoryUtilization }: GPUDetailProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-lg)' }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div>
          <h2 style={{ fontSize: 18, fontWeight: 600, color: 'var(--text-primary)' }}>{device.name}</h2>
          {device.driver_version && (
            <p style={{ fontSize: 13, color: 'var(--text-tertiary)', marginTop: 4 }}>
              CUDA {device.cuda_version} · 驱动 {device.driver_version}
            </p>
          )}
        </div>
        <span className="badge badge-blue" style={{ padding: '4px 12px', fontSize: 13 }}>{device.type}</span>
      </div>

      {/* Utilization */}
      <div className="card">
        <h3 style={{ fontSize: 13, color: 'var(--text-tertiary)', marginBottom: 'var(--spacing-md)' }}>GPU 利用率</h3>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-md)' }}>
          <div style={{ flex: 1 }}>
            <div className="progress-bar" style={{ height: 6 }}>
              <div className="progress-fill" style={{ width: `${getMemoryUtilization(device)}%` }} />
            </div>
          </div>
          <span style={{ fontWeight: 500, width: 48, textAlign: 'right' }}>{getMemoryUtilization(device)}%</span>
        </div>
      </div>

      {/* Memory */}
      <div className="card">
        <h3 style={{ fontSize: 13, color: 'var(--text-tertiary)', marginBottom: 'var(--spacing-md)' }}>显存</h3>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-md)' }}>
          <div style={{ flex: 1 }}>
            <div className="progress-bar" style={{ height: 6 }}>
              <div className="progress-fill" style={{ width: `${getMemoryUtilization(device)}%`, background: 'var(--status-success)' }} />
            </div>
          </div>
          <span style={{ fontWeight: 500, width: 80, textAlign: 'right' }}>
            {formatBytes(device.memory_used)} / {formatBytes(device.memory_total)}
          </span>
        </div>
      </div>

      {/* Stats Grid */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 'var(--spacing-md)' }}>
        <div className="card" style={{ textAlign: 'center' }}>
          <div style={{ fontSize: 24, fontWeight: 700, color: 'var(--accent-primary)' }}>{device.compute_capability || '-'}</div>
          <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 4 }}>计算能力</div>
        </div>
        <div className="card" style={{ textAlign: 'center' }}>
          <div style={{ fontSize: 24, fontWeight: 700, color: 'var(--status-success)' }}>{device.cuda_version || '-'}</div>
          <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 4 }}>CUDA 版本</div>
        </div>
        <div className="card" style={{ textAlign: 'center' }}>
          <div style={{ fontSize: 24, fontWeight: 700, color: 'var(--status-warning)' }}>{formatBytes(device.memory_free)}</div>
          <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 4 }}>可用显存</div>
        </div>
      </div>
    </div>
  );
}

interface CPUDetailProps {
  device: DeviceInfo;
  formatBytes: (bytes: number) => string;
  getMemoryUtilization: (device: DeviceInfo) => number;
}

function CPUDetail({ device, formatBytes, getMemoryUtilization }: CPUDetailProps) {
  const cores = [12, 15, 8, 22, 31, 18, 25, 12];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-lg)' }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div>
          <h2 style={{ fontSize: 18, fontWeight: 600, color: 'var(--text-primary)' }}>{device.name}</h2>
        </div>
        <span className="badge badge-green" style={{ padding: '4px 12px', fontSize: 13 }}>{device.type}</span>
      </div>

      {/* Memory Usage */}
      <div className="card">
        <h3 style={{ fontSize: 13, color: 'var(--text-tertiary)', marginBottom: 'var(--spacing-md)' }}>内存使用</h3>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-md)' }}>
          <div style={{ flex: 1 }}>
            <div className="progress-bar" style={{ height: 6 }}>
              <div className="progress-fill" style={{ width: `${getMemoryUtilization(device)}%`, background: 'var(--accent-primary)' }} />
            </div>
          </div>
          <span style={{ fontWeight: 500, width: 80, textAlign: 'right' }}>
            {formatBytes(device.memory_used)} / {formatBytes(device.memory_total)}
          </span>
        </div>
      </div>

      {/* Cores */}
      <div className="card">
        <h3 style={{ fontSize: 13, color: 'var(--text-tertiary)', marginBottom: 'var(--spacing-md)' }}>核心利用率</h3>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(8, 1fr)', gap: 'var(--spacing-sm)' }}>
          {cores.map((usage, i) => (
            <div key={i} style={{ textAlign: 'center' }}>
              <div
                style={{
                  height: 64,
                  background: 'var(--bg-elevated)',
                  borderRadius: 'var(--radius-sm)',
                  display: 'flex',
                  alignItems: 'flex-end',
                  justifyContent: 'center',
                  paddingBottom: 4,
                }}
              >
                <div
                  style={{
                    width: '100%',
                    background: 'var(--accent-primary)',
                    borderRadius: 'var(--radius-sm)',
                    height: `${usage}%`,
                  }}
                />
              </div>
              <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 4 }}>{i}</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
