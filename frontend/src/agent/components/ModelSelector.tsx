import { useState, useRef, useEffect } from "react";
import {
  Brain,
  CheckCircle2,
  XCircle,
  Settings,
  Cpu,
  Cloud,
  ChevronDown,
  Sparkles,
  Server,
} from "lucide-react";
import type { ModelConfig, ModelSelectorProps } from "../types";

export default function ModelSelector({
  models,
  activeModel,
  onChange,
  onManage,
}: ModelSelectorProps) {
  const [open, setOpen] = useState(false);
  const [autoMode, setAutoMode] = useState(activeModel === "auto");
  const dropdownRef = useRef<HTMLDivElement>(null);

  // 点击外部关闭下拉菜单
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node)
      ) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const handleSelect = (modelId: string) => {
    onChange(modelId);
    setAutoMode(false);
    setOpen(false);
  };

  const handleToggleAuto = () => {
    const newAuto = !autoMode;
    setAutoMode(newAuto);
    onChange(newAuto ? "auto" : models[0]?.id || "");
    if (newAuto) setOpen(false);
  };

  // 分组模型
  const cloudModels = models.filter(
    (m) => m.provider === "openai" || m.provider === "anthropic" || m.provider === "gemini"
  );
  const localModels = models.filter((m) => m.provider === "ollama");
  const customModels = models.filter(
    (m) => m.provider === "openai_compatible"
  );

  const activeModelData = models.find((m) => m.id === activeModel);

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        onClick={() => setOpen(!open)}
        className="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-surface dark:bg-surface-dark border border-gray-200/50 dark:border-gray-800/50 hover:bg-surface-hover dark:hover:bg-surface-hover-dark transition-colors text-sm"
        title="选择模型"
      >
        <Brain className="w-4 h-4 text-brand-primary" />
        <span className="text-gray-700 dark:text-gray-300 font-medium max-w-[120px] truncate">
          {autoMode ? "Auto" : activeModelData?.name || "选择模型"}
        </span>
        <ChevronDown
          className={`w-3.5 h-3.5 text-gray-400 transition-transform ${
            open ? "rotate-180" : ""
          }`}
        />
      </button>

      {open && (
        <div className="absolute top-full left-0 mt-1.5 w-72 bg-surface dark:bg-surface-dark rounded-2xl shadow-xl border border-gray-200/50 dark:border-gray-800/50 z-50 overflow-hidden">
          {/* Auto模式 */}
          <div className="px-3 py-2.5 border-b border-gray-100 dark:border-gray-800">
            <button
              onClick={handleToggleAuto}
              className={`w-full flex items-center gap-2.5 px-2.5 py-2 rounded-xl transition-colors ${
                autoMode
                  ? "bg-brand-primary/10 text-brand-primary"
                  : "hover:bg-gray-50 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300"
              }`}
            >
              <Sparkles className="w-4 h-4" />
              <div className="flex-1 text-left">
                <div className="text-sm font-medium">自动路由</div>
                <div className="text-[10px] text-gray-400">
                  根据任务类型自动选择最佳模型
                </div>
              </div>
              {autoMode && <CheckCircle2 className="w-4 h-4" />}
            </button>
          </div>

          {/* 模型列表 */}
          <div className="max-h-64 overflow-auto py-1">
            {cloudModels.length > 0 && (
              <ModelGroup
                icon={<Cloud className="w-3.5 h-3.5" />}
                title="云模型"
                models={cloudModels}
                activeModel={autoMode ? "" : activeModel}
                onSelect={handleSelect}
              />
            )}
            {localModels.length > 0 && (
              <ModelGroup
                icon={<Server className="w-3.5 h-3.5" />}
                title="本地模型"
                models={localModels}
                activeModel={autoMode ? "" : activeModel}
                onSelect={handleSelect}
              />
            )}
            {customModels.length > 0 && (
              <ModelGroup
                icon={<Cpu className="w-3.5 h-3.5" />}
                title="自定义"
                models={customModels}
                activeModel={autoMode ? "" : activeModel}
                onSelect={handleSelect}
              />
            )}
            {models.length === 0 && (
              <div className="px-4 py-6 text-center text-xs text-gray-400">
                暂无模型配置
              </div>
            )}
          </div>

          {/* 底部管理按钮 */}
          <div className="border-t border-gray-100 dark:border-gray-800 px-3 py-2">
            <button
              onClick={() => {
                setOpen(false);
                onManage();
              }}
              className="w-full flex items-center gap-2 px-2.5 py-2 rounded-xl text-sm text-gray-500 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
            >
              <Settings className="w-3.5 h-3.5" />
              <span>管理模型</span>
            </button>
          </div>
        </div>
      )}

      {/* Auto模式提示 */}
      {autoMode && (
        <div className="absolute top-full left-0 mt-1.5 w-64 p-2.5 bg-blue-50 dark:bg-blue-950/30 rounded-xl border border-blue-100 dark:border-blue-900/50 text-[11px] text-blue-700 dark:text-blue-300 z-40">
          <div className="font-medium mb-1">自动路由规则</div>
          <ul className="space-y-0.5 text-blue-600/80 dark:text-blue-400/70">
            <li>代码任务 &rarr; Claude / GPT-4</li>
            <li>简单对话 &rarr; 轻量模型</li>
            <li>长文本 &rarr; 大上下文模型</li>
            <li>分析任务 &rarr; 最强模型</li>
          </ul>
        </div>
      )}
    </div>
  );
}

function ModelGroup({
  icon,
  title,
  models,
  activeModel,
  onSelect,
}: {
  icon: React.ReactNode;
  title: string;
  models: ModelConfig[];
  activeModel: string;
  onSelect: (id: string) => void;
}) {
  return (
    <div className="px-2 py-1">
      <div className="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] font-medium text-gray-400 uppercase tracking-wider">
        {icon}
        {title}
      </div>
      {models.map((model) => (
        <button
          key={model.id}
          onClick={() => onSelect(model.id)}
          className={`w-full flex items-center gap-2.5 px-2.5 py-2 rounded-xl transition-colors text-left ${
            model.id === activeModel
              ? "bg-brand-primary/10 text-brand-primary"
              : "hover:bg-gray-50 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300"
          }`}
        >
          <ProviderIcon provider={model.provider} />
          <div className="flex-1 min-w-0">
            <div className="text-sm font-medium truncate">{model.name}</div>
            <div className="text-[10px] text-gray-400 capitalize">
              {model.provider}
            </div>
          </div>
          {model.id === activeModel ? (
            <CheckCircle2 className="w-3.5 h-3.5 shrink-0" />
          ) : (
            <XCircle className="w-3.5 h-3.5 shrink-0 opacity-0" />
          )}
        </button>
      ))}
    </div>
  );
}

function ProviderIcon({ provider }: { provider: ModelConfig["provider"] }) {
  const colorClass =
    provider === "openai"
      ? "text-emerald-500"
      : provider === "anthropic"
      ? "text-orange-500"
      : provider === "gemini"
      ? "text-blue-500"
      : provider === "ollama"
      ? "text-purple-500"
      : "text-gray-500";

  return <div className={`w-2 h-2 rounded-full ${colorClass} bg-current`} />;
}
