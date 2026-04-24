import { useState, useEffect, useCallback, useRef } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ChatSession, ChatMessage } from "../types";
import { agentApi } from "../api";

let sessionCounter = 0;

function generateId(): string {
  return `${Date.now()}-${++sessionCounter}`;
}

function createNewSession(agentId: string, title?: string): ChatSession {
  const now = Date.now();
  return {
    id: `session-${generateId()}`,
    agentId,
    title: title || "新会话",
    messages: [],
    createdAt: now,
    updatedAt: now,
  };
}

export function useChat(agentId?: string) {
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [currentSession, setCurrentSession] = useState<ChatSession | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [streamingContent, setStreamingContent] = useState("");
  const unlistenRef = useRef<UnlistenFn | null>(null);

  // 监听流式消息
  useEffect(() => {
    let cancelled = false;

    async function setupListener() {
      try {
        const unlisten = await listen<{ sessionId: string; chunk: string }>(
          "agent://chat-chunk",
          (event) => {
            if (cancelled) return;
            const { sessionId, chunk } = event.payload;
            setSessions((prev) =>
              prev.map((s) => {
                if (s.id !== sessionId) return s;
                const msgs = [...s.messages];
                const lastMsg = msgs[msgs.length - 1];
                if (lastMsg && lastMsg.role === "assistant") {
                  msgs[msgs.length - 1] = {
                    ...lastMsg,
                    content: lastMsg.content + chunk,
                  };
                } else {
                  msgs.push({
                    id: `msg-${generateId()}`,
                    role: "assistant",
                    content: chunk,
                    timestamp: Date.now(),
                  });
                }
                return { ...s, messages: msgs, updatedAt: Date.now() };
              })
            );
            setStreamingContent((prev) => prev + chunk);
          }
        );
        if (!cancelled) {
          unlistenRef.current = unlisten;
        } else {
          unlisten();
        }
      } catch {
        // 如果事件监听失败（后端未实现），忽略错误
      }
    }

    setupListener();

    return () => {
      cancelled = true;
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, []);

  const createSession = useCallback(
    (targetAgentId?: string, title?: string) => {
      const aid = targetAgentId || agentId || "default-assistant";
      const session = createNewSession(aid, title);
      setSessions((prev) => [session, ...prev]);
      setCurrentSession(session);
      return session;
    },
    [agentId]
  );

  const sendMessage = useCallback(
    async (content: string) => {
      if (!content.trim()) return;

      let session = currentSession;
      if (!session) {
        session = createSession();
      }

      const userMsg: ChatMessage = {
        id: `msg-${generateId()}`,
        role: "user",
        content: content.trim(),
        timestamp: Date.now(),
      };

      // 添加用户消息
      const updatedSession: ChatSession = {
        ...session,
        messages: [...session.messages, userMsg],
        updatedAt: Date.now(),
        title:
          session.messages.length === 0
            ? content.trim().slice(0, 30)
            : session.title,
      };

      setCurrentSession(updatedSession);
      setSessions((prev) =>
        prev.map((s) => (s.id === updatedSession.id ? updatedSession : s))
      );
      setIsLoading(true);
      setStreamingContent("");

      try {
        await agentApi.sendMessage(
          updatedSession.id,
          content.trim(),
          updatedSession.agentId
        );
      } catch (err) {
        // 如果后端未实现，模拟AI回复
        console.warn("聊天API未实现，使用模拟回复:", err);
        await new Promise((resolve) => setTimeout(resolve, 500));

        const assistantMsg: ChatMessage = {
          id: `msg-${generateId()}`,
          role: "assistant",
          content: `收到你的消息：**"${content.trim()}"**\n\n> 注意：当前后端Agent模块尚未完全实现，这是前端模拟的回复。\n\n你可以:\n1. 检查后端是否正确加载了Agent模块\n2. 查看Tauri命令是否正确注册\n3. 确认模型配置是否正确`,
          timestamp: Date.now(),
        };

        const finalSession: ChatSession = {
          ...updatedSession,
          messages: [...updatedSession.messages, assistantMsg],
          updatedAt: Date.now(),
        };

        setCurrentSession(finalSession);
        setSessions((prev) =>
          prev.map((s) => (s.id === finalSession.id ? finalSession : s))
        );
      } finally {
        setIsLoading(false);
      }
    },
    [currentSession, createSession]
  );

  const cancelChat = useCallback(async () => {
    if (!currentSession) return;
    try {
      await agentApi.cancelChat(currentSession.id);
    } catch {
      // 忽略取消错误
    }
    setIsLoading(false);
  }, [currentSession]);

  const selectSession = useCallback((sessionId: string) => {
    setSessions((prev) => {
      const session = prev.find((s) => s.id === sessionId);
      if (session) {
        setCurrentSession(session);
      }
      return prev;
    });
  }, []);

  const deleteSession = useCallback(
    (sessionId: string) => {
      setSessions((prev) => prev.filter((s) => s.id !== sessionId));
      if (currentSession?.id === sessionId) {
        setCurrentSession(null);
      }
    },
    [currentSession]
  );

  return {
    sessions,
    currentSession,
    isLoading,
    streamingContent,
    sendMessage,
    cancelChat,
    createSession,
    selectSession,
    deleteSession,
  };
}
