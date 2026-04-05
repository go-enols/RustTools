import { useState, useEffect } from 'react';
import { useSettingsStore, DeviceInfo } from '../../stores/settingsStore';

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
