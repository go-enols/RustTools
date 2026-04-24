import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { BrainCircuit, RefreshCw, CheckCircle2, AlertCircle, FolderOpen, FileCheck } from "lucide-react";

interface ModelInfo {
  name: string;
  path: string;
  size: number;
}

interface ModelSelectorProps {
  onLoad?: (name: string) => void;
  onSelect?: (name: string, path: string) => void;
  compact?: boolean;
  ext?: string; // 模型扩展名，默认 onnx
  autoLoad?: boolean;
  loadOnSelect?: boolean; // 是否调用后端 load_model，默认 true
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default function ModelSelector({
  onLoad,
  onSelect,
  compact,
  ext = "onnx",
  autoLoad = true,
  loadOnSelect = true,
}: ModelSelectorProps) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [selectedPath, setSelectedPath] = useState("");
  const [selectedName, setSelectedName] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const doLoad = useCallback(
    async (path: string, name: string) => {
      setSelectedPath(path);
      setSelectedName(name);
      setError("");
      if (loadOnSelect) {
        try {
          await invoke("load_model", { modelPath: path });
          // loaded model
          onLoad?.(name);
        } catch (e: any) {
          setError(typeof e === "string" ? e : e.message || "加载模型失败");
          // clear loaded model
        }
      } else {
        // loaded model
        onSelect?.(name, path);
      }
    },
    [onLoad, onSelect, loadOnSelect]
  );

  const refresh = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const list = await invoke<ModelInfo[]>("list_models", { dir: null, ext });
      setModels(list);
      if (list.length > 0 && !selectedPath && autoLoad) {
        await doLoad(list[0].path, list[0].name);
      }
    } catch (e: any) {
      setError(typeof e === "string" ? e : e.message || "扫描模型失败");
    } finally {
      setLoading(false);
    }
  }, [ext, selectedPath, autoLoad, doLoad]);

  const pickModelFile = async () => {
    const file = await open({
      filters: [{ name: "Model", extensions: [ext] }],
    });
    if (file && typeof file === "string") {
      const name = file.split(/[\\/]/).pop() || file;
      await doLoad(file, name);
    }
  };

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (compact) {
    return (
      <div className="flex items-center gap-2 flex-wrap">
        <button
          onClick={pickModelFile}
          className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-gray-100 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-xs text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 transition"
        >
          <FolderOpen className="w-3 h-3" />
          选择模型
        </button>
        {selectedPath && (
          <span className="flex items-center gap-1 text-[10px] text-gray-500 dark:text-gray-400 max-w-[200px] truncate">
            <FileCheck className="w-3 h-3 text-emerald-500 shrink-0" />
            <span className="truncate">{selectedName || selectedPath}</span>
          </span>
        )}
        {error && <span className="text-[10px] text-red-500">{error}</span>}
      </div>
    );
  }

  return (
    <div className="bg-white dark:bg-surface-dark rounded-2xl border border-gray-100 dark:border-gray-800 p-5 shadow-sm">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-sm font-semibold text-gray-900 dark:text-white flex items-center gap-2">
          <BrainCircuit className="w-4 h-4 text-purple-500" />
          模型选择
        </h3>
        <button
          onClick={refresh}
          disabled={loading}
          className="p-1.5 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition"
          title="刷新模型列表"
        >
          <RefreshCw className={`w-3.5 h-3.5 text-gray-400 ${loading ? "animate-spin" : ""}`} />
        </button>
      </div>

      {/* 选择文件按钮 + 当前选择 */}
      <div className="mb-4">
        <button
          onClick={pickModelFile}
          className="w-full flex items-center justify-center gap-2 py-2.5 rounded-xl bg-gray-50 dark:bg-gray-900/50 border border-gray-200 dark:border-gray-700 text-xs text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition"
        >
          <FolderOpen className="w-3.5 h-3.5" />
          选择模型文件
        </button>
        {selectedPath && (
          <div className="mt-2 flex items-center gap-2 px-3 py-2 rounded-lg bg-emerald-50 dark:bg-emerald-950/20 border border-emerald-100 dark:border-emerald-900/30">
            <FileCheck className="w-3.5 h-3.5 text-emerald-500 shrink-0" />
            <span className="text-xs text-emerald-700 dark:text-emerald-400 truncate flex-1">
              {selectedName || selectedPath}
            </span>
          </div>
        )}
      </div>

      {error && (
        <div className="mb-3 flex items-center gap-1.5 text-xs text-red-500 bg-red-50 dark:bg-red-950/20 px-3 py-2 rounded-lg">
          <AlertCircle className="w-3.5 h-3.5" />
          {error}
        </div>
      )}

      {/* 扫描到的模型列表 */}
      <div className="border-t border-gray-100 dark:border-gray-800 pt-3">
        <p className="text-[10px] text-gray-400 dark:text-gray-500 mb-2">模型目录中的文件</p>
        {models.length === 0 ? (
          <p className="text-xs text-gray-400 dark:text-gray-500">未找到 .{ext} 模型文件</p>
        ) : (
          <div className="space-y-1.5 max-h-48 overflow-auto">
            {models.map((m) => {
              const active = selectedPath === m.path;
              return (
                <button
                  key={m.path}
                  onClick={() => doLoad(m.path, m.name)}
                  className={`w-full flex items-center justify-between px-3 py-2 rounded-xl text-left transition-all ${
                    active
                      ? "bg-brand-primary/10 border border-brand-primary/20"
                      : "hover:bg-gray-50 dark:hover:bg-gray-800 border border-transparent"
                  }`}
                >
                  <div className="flex items-center gap-2 min-w-0">
                    {active && <CheckCircle2 className="w-3.5 h-3.5 text-brand-primary shrink-0" />}
                    <span className={`text-xs font-medium truncate ${active ? "text-brand-primary" : "text-gray-700 dark:text-gray-300"}`}>
                      {m.name}
                    </span>
                  </div>
                  <span className="text-[10px] text-gray-400 dark:text-gray-500 shrink-0">{formatSize(m.size)}</span>
                </button>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
