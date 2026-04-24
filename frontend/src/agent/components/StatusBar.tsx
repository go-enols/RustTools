import { Cpu, Bot, Wifi, WifiOff, Zap } from "lucide-react";
import type { StatusBarProps } from "../types";

export default function StatusBar({
  modelName,
  agentName,
  connected,
  tokenCount,
  taskProgress,
}: StatusBarProps) {
  return (
    <div className="h-7 bg-surface dark:bg-surface-dark border-t border-gray-200/50 dark:border-gray-800/50 flex items-center px-3 gap-4 text-[11px] select-none shrink-0">
      {/* 模型 */}
      <div className="flex items-center gap-1.5 text-gray-500 dark:text-gray-400">
        <Cpu className="w-3 h-3" />
        <span className="font-medium">{modelName || "Auto"}</span>
      </div>

      {/* 分隔线 */}
      <div className="w-px h-3.5 bg-gray-200 dark:bg-gray-700" />

      {/* Agent */}
      <div className="flex items-center gap-1.5 text-gray-500 dark:text-gray-400">
        <Bot className="w-3 h-3" />
        <span>{agentName || "未选择"}</span>
      </div>

      {/* 分隔线 */}
      <div className="w-px h-3.5 bg-gray-200 dark:bg-gray-700" />

      {/* 连接状态 */}
      <div className="flex items-center gap-1.5">
        {connected ? (
          <Wifi className="w-3 h-3 text-brand-success" />
        ) : (
          <WifiOff className="w-3 h-3 text-brand-danger" />
        )}
        <span
          className={
            connected
              ? "text-brand-success"
              : "text-brand-danger"
          }
        >
          {connected ? "已连接" : "未连接"}
        </span>
      </div>

      {/* Token使用量 */}
      {tokenCount !== undefined && (
        <>
          <div className="w-px h-3.5 bg-gray-200 dark:bg-gray-700" />
          <div className="flex items-center gap-1.5 text-gray-500 dark:text-gray-400">
            <Zap className="w-3 h-3" />
            <span>{tokenCount.toLocaleString()} tokens</span>
          </div>
        </>
      )}

      {/* 任务进度 */}
      {taskProgress && (
        <>
          <div className="w-px h-3.5 bg-gray-200 dark:bg-gray-700" />
          <div className="flex items-center gap-2 text-gray-500 dark:text-gray-400">
            <span>
              进度: {taskProgress.current}/{taskProgress.total}
            </span>
            <div className="w-20 h-1.5 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
              <div
                className="h-full bg-brand-primary rounded-full transition-all"
                style={{
                  width: `${(taskProgress.current / taskProgress.total) * 100}%`,
                }}
              />
            </div>
          </div>
        </>
      )}

      <div className="flex-1" />

      <span className="text-gray-400 dark:text-gray-500">
        AI Agent IDE v0.1.0
      </span>
    </div>
  );
}
