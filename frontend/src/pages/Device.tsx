import { useEffect, useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Cpu, MemoryStick, Monitor, CircuitBoard, RefreshCw, Clock } from "lucide-react";

interface DeviceInfo {
  cpu: { model: string; cores: number; threads: number };
  memory: { total_mb: number; used_mb: number };
  gpus: Array<{ name: string; memory_mb: number; cuda_available: boolean }>;
  os: string;
  arch: string;
}

export default function Device() {
  const [info, setInfo] = useState<DeviceInfo | null>(null);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [loading, setLoading] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchInfo = useCallback(async () => {
    setLoading(true);
    try {
      const data = await invoke<DeviceInfo>("get_device_info");
      setInfo(data);
      setLastUpdate(new Date());
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchInfo();
  }, [fetchInfo]);

  useEffect(() => {
    if (autoRefresh) {
      intervalRef.current = setInterval(fetchInfo, 2000);
    }
    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [autoRefresh, fetchInfo]);

  if (!info) {
    return (
      <div className="min-h-full flex items-center justify-center">
        <div className="w-6 h-6 border-2 border-brand-primary border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  return (
    <div className="min-h-full p-8">
      <div className="flex items-center justify-between mb-1">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">设备信息</h1>
        <div className="flex items-center gap-3">
          {lastUpdate && (
            <span className="flex items-center gap-1 text-[10px] text-gray-400 dark:text-gray-500">
              <Clock className="w-3 h-3" />
              更新于 {lastUpdate.toLocaleTimeString()}
            </span>
          )}
          <label className="flex items-center gap-1.5 text-[10px] text-gray-500 dark:text-gray-400 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
              className="rounded"
            />
            自动刷新
          </label>
          <button
            onClick={fetchInfo}
            disabled={loading}
            className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 text-xs hover:bg-gray-200 dark:hover:bg-gray-700 transition disabled:opacity-50"
          >
            <RefreshCw className={`w-3 h-3 ${loading ? "animate-spin" : ""}`} />
            刷新
          </button>
        </div>
      </div>
      <p className="text-sm text-gray-500 dark:text-gray-400 mb-8">硬件与系统信息概览（每 2 秒自动刷新）</p>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
        <InfoCard icon={<CircuitBoard className="w-5 h-5" />} title="系统" color="text-blue-500">
          <InfoRow label="操作系统" value={info.os} />
          <InfoRow label="架构" value={info.arch} />
        </InfoCard>

        <InfoCard icon={<Cpu className="w-5 h-5" />} title="处理器" color="text-purple-500">
          <InfoRow label="型号" value={info.cpu.model} />
          <InfoRow label="物理核心" value={`${info.cpu.cores}`} />
          <InfoRow label="逻辑线程" value={`${info.cpu.threads}`} />
        </InfoCard>

        <InfoCard icon={<MemoryStick className="w-5 h-5" />} title="内存" color="text-emerald-500">
          <InfoRow label="总容量" value={`${(info.memory.total_mb / 1024).toFixed(1)} GB`} />
          <InfoRow label="已使用" value={`${(info.memory.used_mb / 1024).toFixed(1)} GB`} />
          <InfoRow
            label="使用率"
            value={`${info.memory.total_mb > 0 ? ((info.memory.used_mb / info.memory.total_mb) * 100).toFixed(1) : "0.0"}%`}
          />
          {/* 内存使用进度条 */}
          <div className="w-full bg-gray-100 dark:bg-gray-900 rounded-full h-2 mt-1">
            <div
              className="bg-emerald-500 h-2 rounded-full transition-all duration-500"
              style={{
                width: `${info.memory.total_mb > 0 ? (info.memory.used_mb / info.memory.total_mb) * 100 : 0}%`,
              }}
            />
          </div>
        </InfoCard>

        {info.gpus.length > 0 ? (
          info.gpus.map((gpu, i) => (
            <InfoCard key={i} icon={<Monitor className="w-5 h-5" />} title={`GPU ${i}`} color="text-amber-500">
              <InfoRow label="型号" value={gpu.name} />
              <InfoRow label="显存" value={`${(gpu.memory_mb / 1024).toFixed(1)} GB`} />
              <InfoRow label="CUDA" value={gpu.cuda_available ? "可用" : "未启用"} />
            </InfoCard>
          ))
        ) : (
          <InfoCard icon={<Monitor className="w-5 h-5" />} title="GPU" color="text-gray-400">
            <p className="text-xs text-gray-500 dark:text-gray-400">未检测到 NVIDIA GPU</p>
          </InfoCard>
        )}
      </div>
    </div>
  );
}

function InfoCard({ icon, title, color, children }: { icon: React.ReactNode; title: string; color: string; children: React.ReactNode }) {
  return (
    <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
      <div className="flex items-center gap-2 mb-4">
        <span className={color}>{icon}</span>
        <h2 className="text-sm font-semibold text-gray-900 dark:text-white">{title}</h2>
      </div>
      <div className="space-y-2.5">{children}</div>
    </div>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between text-xs">
      <span className="text-gray-400 dark:text-gray-500">{label}</span>
      <span className="text-gray-700 dark:text-gray-300 font-medium">{value}</span>
    </div>
  );
}
