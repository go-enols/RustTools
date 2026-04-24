import { useState, useRef, useEffect, useCallback } from "react";
import {
  Send,
  Square,
  RotateCcw,
  Copy,
  Check,
  Bot,
  User,
} from "lucide-react";
import type { ChatPanelProps, ChatMessage } from "../types";
import MessageRenderer from "./MessageRenderer";

export default function ChatPanel({
  session,
  onSendMessage,
  onCancel,
  isLoading,
}: ChatPanelProps) {
  const [input, setInput] = useState("");
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // 自动滚动到底部
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [session.messages, isLoading]);

  // Ctrl+Enter 发送
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleSend = () => {
    if (!input.trim() || isLoading) return;
    onSendMessage(input.trim());
    setInput("");
    // 重置textarea高度
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
    }
  };

  const handleCopy = async (msg: ChatMessage) => {
    try {
      await navigator.clipboard.writeText(msg.content);
      setCopiedId(msg.id);
      setTimeout(() => setCopiedId(null), 2000);
    } catch {
      // 忽略复制失败
    }
  };

  const handleRegenerate = (msg: ChatMessage) => {
    // 找到这条AI消息对应的用户消息，重新发送
    const msgIndex = session.messages.findIndex((m) => m.id === msg.id);
    if (msgIndex > 0) {
      const userMsg = session.messages[msgIndex - 1];
      if (userMsg && userMsg.role === "user") {
        onSendMessage(userMsg.content);
      }
    }
  };

  // 自动调整textarea高度
  const handleInput = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      setInput(e.target.value);
      const el = e.target;
      el.style.height = "auto";
      el.style.height = Math.min(el.scrollHeight, 200) + "px";
    },
    []
  );

  const isEmpty = session.messages.length === 0;

  return (
    <div className="flex-1 flex flex-col min-w-0 bg-bg dark:bg-bg-dark">
      {/* 消息列表 */}
      <div className="flex-1 overflow-auto px-4 py-4">
        {isEmpty ? (
          <EmptyState />
        ) : (
          <div className="max-w-3xl mx-auto space-y-4">
            {session.messages.map((msg) => (
              <MessageItem
                key={msg.id}
                message={msg}
                isCopied={copiedId === msg.id}
                onCopy={() => handleCopy(msg)}
                onRegenerate={() => handleRegenerate(msg)}
              />
            ))}
            {isLoading &&
              session.messages[session.messages.length - 1]?.role ===
                "user" && <LoadingIndicator />}
            <div ref={messagesEndRef} />
          </div>
        )}
      </div>

      {/* 输入区域 */}
      <div className="shrink-0 border-t border-gray-200/50 dark:border-gray-800/50 px-4 py-3">
        <div className="max-w-3xl mx-auto">
          <div className="relative flex items-end gap-2 bg-surface dark:bg-surface-dark rounded-2xl border border-gray-200/50 dark:border-gray-800/50 px-3 py-2 shadow-sm">
            <textarea
              ref={textareaRef}
              value={input}
              onChange={handleInput}
              onKeyDown={handleKeyDown}
              placeholder="输入消息... (Ctrl+Enter 发送)"
              rows={1}
              className="flex-1 bg-transparent border-none outline-none resize-none text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 max-h-[200px] py-1.5"
              disabled={isLoading}
            />
            <div className="flex items-center gap-1 shrink-0 pb-1">
              {isLoading ? (
                <button
                  onClick={onCancel}
                  className="p-2 rounded-xl bg-brand-danger/10 text-brand-danger hover:bg-brand-danger/20 transition-colors"
                  title="停止生成"
                >
                  <Square className="w-4 h-4" />
                </button>
              ) : (
                <button
                  onClick={handleSend}
                  disabled={!input.trim()}
                  className="p-2 rounded-xl bg-brand-primary text-white hover:bg-brand-primary/90 transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
                  title="发送 (Ctrl+Enter)"
                >
                  <Send className="w-4 h-4" />
                </button>
              )}
            </div>
          </div>
          <div className="text-center mt-1.5">
            <span className="text-[10px] text-gray-400">
              Ctrl + Enter 发送
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

function MessageItem({
  message,
  isCopied,
  onCopy,
  onRegenerate,
}: {
  message: ChatMessage;
  isCopied: boolean;
  onCopy: () => void;
  onRegenerate: () => void;
}) {
  const isUser = message.role === "user";

  return (
    <div
      className={`flex gap-3 ${isUser ? "flex-row-reverse" : "flex-row"}`}
    >
      {/* 头像 */}
      <div
        className={`shrink-0 w-7 h-7 rounded-xl flex items-center justify-center ${
          isUser
            ? "bg-brand-primary/10 text-brand-primary"
            : "bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400"
        }`}
      >
        {isUser ? (
          <User className="w-4 h-4" />
        ) : (
          <Bot className="w-4 h-4" />
        )}
      </div>

      {/* 消息内容 */}
      <div
        className={`flex-1 min-w-0 ${isUser ? "max-w-[80%]" : "max-w-[85%]"}`}
      >
        <div
          className={`rounded-2xl px-4 py-3 ${
            isUser
              ? "bg-brand-primary text-white ml-auto"
              : "bg-surface dark:bg-surface-dark border border-gray-200/50 dark:border-gray-800/50"
          }`}
        >
          {isUser ? (
            <p className="text-sm leading-relaxed whitespace-pre-wrap">
              {message.content}
            </p>
          ) : (
            <MessageRenderer
              content={message.content}
              toolCalls={message.toolCalls}
            />
          )}
        </div>

        {/* 操作按钮 */}
        {!isUser && (
          <div className="flex items-center gap-1 mt-1 px-1">
            <button
              onClick={onCopy}
              className="p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
              title="复制"
            >
              {isCopied ? (
                <Check className="w-3 h-3 text-brand-success" />
              ) : (
                <Copy className="w-3 h-3" />
              )}
            </button>
            <button
              onClick={onRegenerate}
              className="p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
              title="重新生成"
            >
              <RotateCcw className="w-3 h-3" />
            </button>
          </div>
        )}

        {/* 时间戳 */}
        <div
          className={`text-[10px] text-gray-400 mt-0.5 ${
            isUser ? "text-right" : "text-left"
          }`}
        >
          {new Date(message.timestamp).toLocaleTimeString("zh-CN", {
            hour: "2-digit",
            minute: "2-digit",
          })}
        </div>
      </div>
    </div>
  );
}

function LoadingIndicator() {
  return (
    <div className="flex gap-3">
      <div className="shrink-0 w-7 h-7 rounded-xl flex items-center justify-center bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400">
        <Bot className="w-4 h-4" />
      </div>
      <div className="flex-1 max-w-[85%]">
        <div className="rounded-2xl px-4 py-3 bg-surface dark:bg-surface-dark border border-gray-200/50 dark:border-gray-800/50">
          <div className="flex items-center gap-1.5">
            <div className="w-1.5 h-1.5 bg-brand-primary rounded-full animate-bounce [animation-delay:0ms]" />
            <div className="w-1.5 h-1.5 bg-brand-primary rounded-full animate-bounce [animation-delay:150ms]" />
            <div className="w-1.5 h-1.5 bg-brand-primary rounded-full animate-bounce [animation-delay:300ms]" />
            <span className="text-xs text-gray-400 ml-1">思考中...</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="h-full flex flex-col items-center justify-center text-center px-6">
      <div className="w-14 h-14 rounded-2xl bg-brand-primary/10 flex items-center justify-center mb-4">
        <Bot className="w-7 h-7 text-brand-primary" />
      </div>
      <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
        AI 助手
      </h3>
      <p className="text-sm text-gray-500 dark:text-gray-400 max-w-sm mb-6">
        我可以帮你编写代码、分析文件、执行命令等。开始一个新会话吧！
      </p>
      <div className="grid grid-cols-2 gap-3 max-w-sm w-full">
        <SuggestionCard text="解释一段代码的工作原理" />
        <SuggestionCard text="帮我写一个 Rust 函数" />
        <SuggestionCard text="分析项目结构" />
        <SuggestionCard text="查找并修复Bug" />
      </div>
    </div>
  );
}

function SuggestionCard({ text }: { text: string }) {
  return (
    <div className="px-3 py-2.5 rounded-xl bg-surface dark:bg-surface-dark border border-gray-200/50 dark:border-gray-800/50 text-xs text-gray-600 dark:text-gray-400 hover:border-brand-primary/30 hover:text-brand-primary transition-colors cursor-pointer text-center">
      {text}
    </div>
  );
}
