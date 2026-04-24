import { useState } from "react";
import { ChevronDown, ChevronRight, Copy, Check } from "lucide-react";
import type { MessageRendererProps, ToolCall } from "../types";

export default function MessageRenderer({
  content,
  toolCalls,
}: MessageRendererProps) {
  return (
    <div className="prose prose-sm dark:prose-invert max-w-none">
      <MarkdownContent content={content} />
      {toolCalls && toolCalls.length > 0 && (
        <div className="mt-3 flex flex-col gap-2">
          {toolCalls.map((tc) => (
            <ToolCallCard key={tc.id} toolCall={tc} />
          ))}
        </div>
      )}
    </div>
  );
}

function ToolCallCard({ toolCall }: { toolCall: ToolCall }) {
  const [expanded, setExpanded] = useState(false);

  const statusColor =
    toolCall.status === "success"
      ? "border-brand-success/30 bg-brand-success/5"
      : toolCall.status === "error"
      ? "border-brand-danger/30 bg-brand-danger/5"
      : "border-brand-warning/30 bg-brand-warning/5";

  const statusDot =
    toolCall.status === "success"
      ? "bg-brand-success"
      : toolCall.status === "error"
      ? "bg-brand-danger"
      : "bg-brand-warning animate-pulse";

  return (
    <div
      className={`rounded-xl border ${statusColor} overflow-hidden`}
    >
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
      >
        {expanded ? (
          <ChevronDown className="w-3.5 h-3.5 text-gray-400" />
        ) : (
          <ChevronRight className="w-3.5 h-3.5 text-gray-400" />
        )}
        <div className={`w-2 h-2 rounded-full ${statusDot}`} />
        <span className="text-xs font-mono font-medium text-gray-700 dark:text-gray-300">
          {toolCall.name}
        </span>
        <span className="text-[10px] text-gray-400 ml-auto capitalize">
          {toolCall.status === "pending"
            ? "执行中"
            : toolCall.status === "success"
            ? "成功"
            : "失败"}
        </span>
      </button>

      {expanded && (
        <div className="px-3 pb-3 border-t border-gray-100 dark:border-gray-800 pt-2">
          <div className="mb-2">
            <span className="text-[10px] font-medium text-gray-400 uppercase tracking-wider">
              参数
            </span>
            <pre className="mt-1 text-[11px] bg-gray-50 dark:bg-gray-900 rounded-lg p-2 overflow-auto font-mono text-gray-700 dark:text-gray-300">
              {JSON.stringify(toolCall.arguments, null, 2)}
            </pre>
          </div>
          {toolCall.result && (
            <div>
              <span className="text-[10px] font-medium text-gray-400 uppercase tracking-wider">
                结果
              </span>
              <pre className="mt-1 text-[11px] bg-gray-50 dark:bg-gray-900 rounded-lg p-2 overflow-auto font-mono text-gray-700 dark:text-gray-300 max-h-40">
                {toolCall.result.length > 500
                  ? toolCall.result.slice(0, 500) + "..."
                  : toolCall.result}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function MarkdownContent({ content }: { content: string }) {
  const lines = content.split("\n");
  const elements: React.ReactNode[] = [];
  let i = 0;
  let inCodeBlock = false;
  let codeContent = "";
  let codeLang = "";

  while (i < lines.length) {
    const line = lines[i];

    if (line.startsWith("```")) {
      if (!inCodeBlock) {
        inCodeBlock = true;
        codeLang = line.slice(3).trim();
        codeContent = "";
      } else {
        inCodeBlock = false;
        elements.push(
          <CodeBlock key={i} lang={codeLang} content={codeContent.trimEnd()} />
        );
        codeContent = "";
        codeLang = "";
      }
      i++;
      continue;
    }

    if (inCodeBlock) {
      codeContent += line + "\n";
      i++;
      continue;
    }

    // 表格
    if (line.startsWith("|")) {
      const tableLines: string[] = [];
      while (i < lines.length && lines[i].startsWith("|")) {
        tableLines.push(lines[i]);
        i++;
      }
      elements.push(<MarkdownTable key={i} lines={tableLines} />);
      continue;
    }

    // 标题
    if (line.startsWith("### ")) {
      elements.push(
        <h3
          key={i}
          className="text-base font-semibold text-gray-900 dark:text-gray-100 mt-4 mb-2"
        >
          {parseInline(line.slice(4))}
        </h3>
      );
      i++;
      continue;
    }
    if (line.startsWith("## ")) {
      elements.push(
        <h2
          key={i}
          className="text-lg font-semibold text-gray-900 dark:text-gray-100 mt-5 mb-2"
        >
          {parseInline(line.slice(3))}
        </h2>
      );
      i++;
      continue;
    }
    if (line.startsWith("# ")) {
      elements.push(
        <h1
          key={i}
          className="text-xl font-bold text-gray-900 dark:text-gray-100 mt-6 mb-3"
        >
          {parseInline(line.slice(2))}
        </h1>
      );
      i++;
      continue;
    }

    // 引用
    if (line.startsWith("> ")) {
      elements.push(
        <blockquote
          key={i}
          className="border-l-2 border-gray-300 dark:border-gray-600 pl-3 my-2 text-gray-600 dark:text-gray-400 italic"
        >
          {parseInline(line.slice(2))}
        </blockquote>
      );
      i++;
      continue;
    }

    // 无序列表
    if (line.match(/^[-*]\s/)) {
      const items: string[] = [];
      while (i < lines.length && lines[i].match(/^[-*]\s/)) {
        items.push(lines[i].slice(2));
        i++;
      }
      elements.push(
        <ul key={i} className="list-disc list-inside my-2 space-y-1">
          {items.map((item, idx) => (
            <li key={idx} className="text-sm text-gray-700 dark:text-gray-300">
              {parseInline(item)}
            </li>
          ))}
        </ul>
      );
      continue;
    }

    // 有序列表
    if (line.match(/^\d+\.\s/)) {
      const items: string[] = [];
      while (i < lines.length && lines[i].match(/^\d+\.\s/)) {
        items.push(lines[i].replace(/^\d+\.\s/, ""));
        i++;
      }
      elements.push(
        <ol key={i} className="list-decimal list-inside my-2 space-y-1">
          {items.map((item, idx) => (
            <li key={idx} className="text-sm text-gray-700 dark:text-gray-300">
              {parseInline(item)}
            </li>
          ))}
        </ol>
      );
      continue;
    }

    // 分隔线
    if (line.match(/^-{3,}$/) || line.match(/^\*{3,}$/)) {
      elements.push(
        <hr
          key={i}
          className="my-4 border-gray-200 dark:border-gray-700"
        />
      );
      i++;
      continue;
    }

    // 空行
    if (line.trim() === "") {
      elements.push(<div key={i} className="h-2" />);
      i++;
      continue;
    }

    // 普通段落
    elements.push(
      <p key={i} className="text-sm text-gray-700 dark:text-gray-300 my-1.5 leading-relaxed">
        {parseInline(line)}
      </p>
    );
    i++;
  }

  // 如果内容在代码块中结束但未关闭
  if (inCodeBlock && codeContent) {
    elements.push(
      <CodeBlock key="unclosed" lang={codeLang} content={codeContent.trimEnd()} />
    );
  }

  return <>{elements}</>;
}

function CodeBlock({ lang, content }: { lang: string; content: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // 忽略复制失败
    }
  };

  return (
    <div className="my-3 rounded-xl overflow-hidden border border-gray-200 dark:border-gray-700">
      <div className="flex items-center justify-between px-3 py-1.5 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
        <span className="text-[10px] font-mono text-gray-500 dark:text-gray-400 uppercase">
          {lang || "code"}
        </span>
        <button
          onClick={handleCopy}
          className="p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
          title="复制"
        >
          {copied ? (
            <Check className="w-3 h-3 text-brand-success" />
          ) : (
            <Copy className="w-3 h-3 text-gray-400" />
          )}
        </button>
      </div>
      <pre className="p-3 overflow-auto bg-gray-50 dark:bg-gray-900">
        <code className="text-[13px] font-mono leading-relaxed text-gray-800 dark:text-gray-200">
          {content}
        </code>
      </pre>
    </div>
  );
}

function MarkdownTable({ lines }: { lines: string[] }) {
  if (lines.length < 2) return null;

  // 过滤掉分隔线行 (|:---:|:---|)
  const dataLines = lines.filter((l) => !l.match(/^\|[\s:|-]+\|$/));
  if (dataLines.length === 0) return null;

  const headers = dataLines[0]
    .split("|")
    .filter((c) => c.trim() !== "")
    .map((c) => c.trim());
  const rows = dataLines.slice(1).map((line) =>
    line
      .split("|")
      .filter((c) => c.trim() !== "")
      .map((c) => c.trim())
  );

  return (
    <div className="my-3 overflow-auto">
      <table className="min-w-full text-sm border-collapse">
        <thead>
          <tr className="border-b border-gray-200 dark:border-gray-700">
            {headers.map((h, i) => (
              <th
                key={i}
                className="text-left px-3 py-2 font-semibold text-gray-700 dark:text-gray-300 text-xs"
              >
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, ri) => (
            <tr
              key={ri}
              className="border-b border-gray-100 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-800/50"
            >
              {row.map((cell, ci) => (
                <td
                  key={ci}
                  className="px-3 py-2 text-gray-600 dark:text-gray-400 text-xs"
                >
                  {cell}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function parseInline(text: string): React.ReactNode {
  const parts: React.ReactNode[] = [];
  let key = 0;

  // 粗体 **text**
  const boldRegex = /\*\*(.+?)\*\*/g;
  let match;
  let lastIndex = 0;

  while ((match = boldRegex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push(
        ...parseInlineItalic(text.slice(lastIndex, match.index), key++)
      );
      key += 2;
    }
    parts.push(
      <strong key={key++} className="font-semibold text-gray-900 dark:text-gray-100">
        {parseInlineItalic(match[1], key)}
      </strong>
    );
    key += 10;
    lastIndex = match.index + match[0].length;
  }

  if (lastIndex < text.length) {
    parts.push(...parseInlineItalic(text.slice(lastIndex), key++));
  }

  return parts.length > 0 ? parts : text;
}

function parseInlineItalic(
  text: string,
  startKey: number
): React.ReactNode[] {
  const parts: React.ReactNode[] = [];
  const italicRegex = /_(.+?)_/g;
  let match;
  let lastIndex = 0;
  let key = startKey;

  while ((match = italicRegex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push(
        <span key={key++}>{text.slice(lastIndex, match.index)}</span>
      );
    }
    parts.push(
      <em key={key++} className="italic text-gray-600 dark:text-gray-400">
        {match[1]}
      </em>
    );
    lastIndex = match.index + match[0].length;
  }

  if (lastIndex < text.length) {
    parts.push(<span key={key++}>{text.slice(lastIndex)}</span>);
  }

  if (parts.length === 0) {
    parts.push(<span key={key++}>{text}</span>);
  }

  return parts;
}
