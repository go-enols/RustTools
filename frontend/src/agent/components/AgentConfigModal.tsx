import { useState, useEffect } from "react";
import {
  X,
  Save,
  Sparkles,
  Plus,
  Trash2,
  FolderOpen,
} from "lucide-react";
import type { AgentConfigModalProps, AgentDefinition } from "../types";
import { BUILTIN_TOOLS } from "../hooks/useTools";

const TOOL_OPTIONS = BUILTIN_TOOLS.map((t) => ({
  value: t.id,
  label: t.name,
}));

const DEFAULT_AGENT: AgentDefinition = {
  id: "",
  name: "",
  description: "",
  systemPrompt: "",
  modelId: "auto",
  tools: [],
  mcpServers: [],
  autoMode: false,
  maxIterations: 10,
  allowedDirectories: [],
};

export default function AgentConfigModal({
  open,
  agent,
  models,
  mcpServers,
  onSave,
  onClose,
}: AgentConfigModalProps) {
  const [form, setForm] = useState<AgentDefinition>(DEFAULT_AGENT);
  const [newDir, setNewDir] = useState("");

  useEffect(() => {
    if (agent) {
      setForm(agent);
    } else {
      setForm(DEFAULT_AGENT);
    }
  }, [agent, open]);

  if (!open) return null;

  const isEditing = !!agent?.id;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSave(form);
  };

  const updateField = <K extends keyof AgentDefinition>(
    field: K,
    value: AgentDefinition[K]
  ) => {
    setForm((prev) => ({ ...prev, [field]: value }));
  };

  const toggleTool = (toolId: string) => {
    setForm((prev) => ({
      ...prev,
      tools: prev.tools.includes(toolId)
        ? prev.tools.filter((t) => t !== toolId)
        : [...prev.tools, toolId],
    }));
  };

  const addDirectory = () => {
    if (!newDir.trim()) return;
    setForm((prev) => ({
      ...prev,
      allowedDirectories: [...prev.allowedDirectories, newDir.trim()],
    }));
    setNewDir("");
  };

  const removeDirectory = (index: number) => {
    setForm((prev) => ({
      ...prev,
      allowedDirectories: prev.allowedDirectories.filter((_, i) => i !== index),
    }));
  };

  const toggleMcpServer = (name: string) => {
    setForm((prev) => ({
      ...prev,
      mcpServers: prev.mcpServers.includes(name)
        ? prev.mcpServers.filter((s) => s !== name)
        : [...prev.mcpServers, name],
    }));
  };

  const handleGeneratePrompt = () => {
    // 模拟AI生成系统提示词
    const name = form.name || "AI助手";
    const desc = form.description || "全能型助手";
    updateField(
      "systemPrompt",
      `你是${name}，${desc}。\n\n请用中文回答用户的问题。回答要简洁、准确、有条理。当你不确定时，请诚实地告诉用户你不知道。`
    );
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* 遮罩 */}
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />

      {/* 弹窗 */}
      <div className="relative bg-surface dark:bg-surface-dark rounded-2xl shadow-2xl border border-gray-200/50 dark:border-gray-800/50 w-[560px] max-h-[85vh] flex flex-col">
        {/* 头部 */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-gray-100 dark:border-gray-800">
          <h2 className="text-base font-semibold text-gray-900 dark:text-white">
            {isEditing ? "编辑 Agent" : "新建 Agent"}
          </h2>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
          >
            <X className="w-4 h-4 text-gray-400" />
          </button>
        </div>

        {/* 表单 */}
        <form
          onSubmit={handleSubmit}
          className="flex-1 overflow-auto px-5 py-4 space-y-4"
        >
          {/* 名称 */}
          <div>
            <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1.5">
              名称
            </label>
            <input
              type="text"
              value={form.name}
              onChange={(e) => updateField("name", e.target.value)}
              placeholder="输入Agent名称"
              className="w-full px-3 py-2 rounded-xl bg-bg dark:bg-bg-dark border border-gray-200 dark:border-gray-700 text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 outline-none focus:border-brand-primary transition-colors"
              required
            />
          </div>

          {/* 描述 */}
          <div>
            <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1.5">
              描述
            </label>
            <input
              type="text"
              value={form.description}
              onChange={(e) => updateField("description", e.target.value)}
              placeholder="简短描述Agent的用途"
              className="w-full px-3 py-2 rounded-xl bg-bg dark:bg-bg-dark border border-gray-200 dark:border-gray-700 text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 outline-none focus:border-brand-primary transition-colors"
            />
          </div>

          {/* 系统提示词 */}
          <div>
            <div className="flex items-center justify-between mb-1.5">
              <label className="text-xs font-medium text-gray-700 dark:text-gray-300">
                系统提示词
              </label>
              <button
                type="button"
                onClick={handleGeneratePrompt}
                className="flex items-center gap-1 text-[11px] text-brand-primary hover:text-brand-primary/80 transition-colors"
              >
                <Sparkles className="w-3 h-3" />
                AI生成
              </button>
            </div>
            <textarea
              value={form.systemPrompt}
              onChange={(e) => updateField("systemPrompt", e.target.value)}
              placeholder="定义Agent的行为和角色..."
              rows={4}
              className="w-full px-3 py-2 rounded-xl bg-bg dark:bg-bg-dark border border-gray-200 dark:border-gray-700 text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 outline-none focus:border-brand-primary transition-colors resize-none font-mono leading-relaxed"
            />
          </div>

          {/* 模型选择 */}
          <div>
            <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1.5">
              使用模型
            </label>
            <select
              value={form.modelId}
              onChange={(e) => updateField("modelId", e.target.value)}
              className="w-full px-3 py-2 rounded-xl bg-bg dark:bg-bg-dark border border-gray-200 dark:border-gray-700 text-sm text-gray-900 dark:text-gray-100 outline-none focus:border-brand-primary transition-colors"
            >
              <option value="auto">自动路由</option>
              {models.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name} ({m.provider})
                </option>
              ))}
            </select>
          </div>

          {/* 工具权限 */}
          <div>
            <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1.5">
              工具权限
            </label>
            <div className="grid grid-cols-3 gap-2">
              {TOOL_OPTIONS.map((tool) => (
                <label
                  key={tool.value}
                  className={`flex items-center gap-2 px-2.5 py-2 rounded-xl border cursor-pointer transition-colors ${
                    form.tools.includes(tool.value)
                      ? "border-brand-primary/30 bg-brand-primary/5 text-brand-primary"
                      : "border-gray-200 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:border-gray-300 dark:hover:border-gray-600"
                  }`}
                >
                  <input
                    type="checkbox"
                    checked={form.tools.includes(tool.value)}
                    onChange={() => toggleTool(tool.value)}
                    className="shrink-0"
                  />
                  <span className="text-xs truncate">{tool.label}</span>
                </label>
              ))}
            </div>
          </div>

          {/* MCP服务器 */}
          {mcpServers.length > 0 && (
            <div>
              <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1.5">
                MCP 服务器
              </label>
              <div className="space-y-1">
                {mcpServers.map((server) => (
                  <label
                    key={server.name}
                    className={`flex items-center gap-2 px-2.5 py-2 rounded-xl border cursor-pointer transition-colors ${
                      form.mcpServers.includes(server.name)
                        ? "border-brand-primary/30 bg-brand-primary/5 text-brand-primary"
                        : "border-gray-200 dark:border-gray-700 text-gray-600 dark:text-gray-400"
                    }`}
                  >
                    <input
                      type="checkbox"
                      checked={form.mcpServers.includes(server.name)}
                      onChange={() => toggleMcpServer(server.name)}
                      className="shrink-0"
                    />
                    <span className="text-xs">{server.name}</span>
                    <span
                      className={`text-[10px] ml-auto ${
                        server.status === "Connected"
                          ? "text-brand-success"
                          : "text-brand-danger"
                      }`}
                    >
                      {server.status}
                    </span>
                  </label>
                ))}
              </div>
            </div>
          )}

          {/* Auto模式 + 最大迭代 */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={form.autoMode}
                  onChange={(e) => updateField("autoMode", e.target.checked)}
                />
                <span className="text-xs text-gray-700 dark:text-gray-300">
                  自动执行模式
                </span>
              </label>
              <p className="text-[10px] text-gray-400 mt-1 ml-6">
                允许Agent自动执行工具调用
              </p>
            </div>
            <div>
              <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1.5">
                最大迭代次数
              </label>
              <input
                type="number"
                value={form.maxIterations}
                onChange={(e) =>
                  updateField("maxIterations", parseInt(e.target.value) || 10)
                }
                min={1}
                max={100}
                className="w-full px-3 py-2 rounded-xl bg-bg dark:bg-bg-dark border border-gray-200 dark:border-gray-700 text-sm text-gray-900 dark:text-gray-100 outline-none focus:border-brand-primary transition-colors"
              />
            </div>
          </div>

          {/* 允许访问目录 */}
          <div>
            <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1.5">
              允许访问目录
            </label>
            <div className="flex gap-2 mb-2">
              <input
                type="text"
                value={newDir}
                onChange={(e) => setNewDir(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && (e.preventDefault(), addDirectory())}
                placeholder="输入目录路径"
                className="flex-1 px-3 py-2 rounded-xl bg-bg dark:bg-bg-dark border border-gray-200 dark:border-gray-700 text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 outline-none focus:border-brand-primary transition-colors"
              />
              <button
                type="button"
                onClick={addDirectory}
                className="px-3 py-2 rounded-xl bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
              >
                <Plus className="w-4 h-4" />
              </button>
            </div>
            {form.allowedDirectories.length > 0 && (
              <div className="flex flex-wrap gap-1.5">
                {form.allowedDirectories.map((dir, index) => (
                  <span
                    key={index}
                    className="inline-flex items-center gap-1 px-2 py-1 rounded-lg bg-gray-100 dark:bg-gray-800 text-[11px] text-gray-600 dark:text-gray-400"
                  >
                    <FolderOpen className="w-3 h-3" />
                    {dir}
                    <button
                      type="button"
                      onClick={() => removeDirectory(index)}
                      className="p-0.5 rounded hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors ml-1"
                    >
                      <Trash2 className="w-2.5 h-2.5" />
                    </button>
                  </span>
                ))}
              </div>
            )}
          </div>
        </form>

        {/* 底部按钮 */}
        <div className="flex items-center justify-end gap-2 px-5 py-4 border-t border-gray-100 dark:border-gray-800">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 rounded-xl text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
          >
            取消
          </button>
          <button
            onClick={handleSubmit}
            disabled={!form.name.trim()}
            className="flex items-center gap-1.5 px-4 py-2 rounded-xl bg-brand-primary text-white text-sm font-medium hover:bg-brand-primary/90 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <Save className="w-4 h-4" />
            保存
          </button>
        </div>
      </div>
    </div>
  );
}
