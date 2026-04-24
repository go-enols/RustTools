import { useState } from "react";
import { FileCode, Copy, Check } from "lucide-react";

interface EditorPanelProps {
  fileName?: string;
  content?: string;
  language?: string;
}

export default function EditorPanel({
  fileName = "untitled",
  content = "",
  language = "text",
}: EditorPanelProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // 忽略
    }
  };

  return (
    <div className="flex-1 flex flex-col min-w-0 bg-bg dark:bg-bg-dark">
      {/* 标签栏 */}
      <div className="h-9 flex items-center border-b border-gray-200/50 dark:border-gray-800/50 bg-surface dark:bg-surface-dark">
        <div className="flex items-center gap-2 px-3 py-1.5 bg-bg dark:bg-bg-dark border-t-2 border-brand-primary">
          <FileCode className="w-3.5 h-3.5 text-gray-400" />
          <span className="text-xs font-medium text-gray-700 dark:text-gray-300">
            {fileName}
          </span>
          <span className="text-[10px] text-gray-400 uppercase">
            {language}
          </span>
        </div>
      </div>

      {/* 编辑器内容 */}
      <div className="flex-1 overflow-auto relative">
        <div className="absolute top-2 right-2 z-10">
          <button
            onClick={handleCopy}
            className="p-1.5 rounded-lg bg-surface dark:bg-surface-dark border border-gray-200 dark:border-gray-700 shadow-sm hover:shadow transition-all"
            title="复制"
          >
            {copied ? (
              <Check className="w-3.5 h-3.5 text-brand-success" />
            ) : (
              <Copy className="w-3.5 h-3.5 text-gray-400" />
            )}
          </button>
        </div>
        <pre className="p-4 text-[13px] font-mono leading-relaxed text-gray-800 dark:text-gray-200">
          <code>{content || "// 选择文件以查看内容"}</code>
        </pre>
      </div>
    </div>
  );
}
