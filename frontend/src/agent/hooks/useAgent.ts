import { useState, useEffect, useCallback } from "react";
import type { AgentDefinition } from "../types";
import { agentApi } from "../api";

const DEFAULT_AGENTS: AgentDefinition[] = [
  {
    id: "default-assistant",
    name: "通用助手",
    description: "全能型AI助手，适用于各种任务",
    systemPrompt:
      "你是一个全能型AI助手。请用中文回答用户的问题。回答要简洁、准确、有条理。",
    modelId: "auto",
    tools: ["fs_read", "terminal", "web_search"],
    mcpServers: [],
    autoMode: false,
    maxIterations: 10,
    allowedDirectories: [],
  },
  {
    id: "code-expert",
    name: "代码专家",
    description: "专注于编程和代码审查",
    systemPrompt:
      "你是一名资深软件工程师，擅长代码编写、代码审查和调试。请提供高质量的代码，并附带必要的注释。",
    modelId: "auto",
    tools: ["fs_read", "fs_write", "code_edit", "terminal"],
    mcpServers: [],
    autoMode: true,
    maxIterations: 15,
    allowedDirectories: [],
  },
];

export function useAgent() {
  const [agents, setAgents] = useState<AgentDefinition[]>(DEFAULT_AGENTS);
  const [currentAgent, setCurrentAgent] = useState<AgentDefinition | null>(
    DEFAULT_AGENTS[0]
  );
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadAgents = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await agentApi.listAgents();
      if (data && data.length > 0) {
        setAgents(data);
        setCurrentAgent(data[0]);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : "加载Agent失败";
      setError(msg);
      // 使用默认Agent数据
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadAgents();
  }, [loadAgents]);

  const createAgent = useCallback(
    async (agent: Omit<AgentDefinition, "id">) => {
      try {
        setLoading(true);
        const id = await agentApi.createAgent(agent);
        const newAgent: AgentDefinition = { ...agent, id };
        setAgents((prev) => [...prev, newAgent]);
        return id;
      } catch (err) {
        const msg = err instanceof Error ? err.message : "创建Agent失败";
        setError(msg);
        // 模拟创建成功
        const fallbackId = `agent-${Date.now()}`;
        const newAgent: AgentDefinition = { ...agent, id: fallbackId };
        setAgents((prev) => [...prev, newAgent]);
        return fallbackId;
      } finally {
        setLoading(false);
      }
    },
    []
  );

  const updateAgent = useCallback(
    async (id: string, agent: AgentDefinition) => {
      try {
        setLoading(true);
        await agentApi.updateAgent(id, agent);
        setAgents((prev) =>
          prev.map((a) => (a.id === id ? agent : a))
        );
        if (currentAgent?.id === id) {
          setCurrentAgent(agent);
        }
        return true;
      } catch (err) {
        const msg = err instanceof Error ? err.message : "更新Agent失败";
        setError(msg);
        // 仍然在前端更新
        setAgents((prev) =>
          prev.map((a) => (a.id === id ? agent : a))
        );
        if (currentAgent?.id === id) {
          setCurrentAgent(agent);
        }
        return true;
      } finally {
        setLoading(false);
      }
    },
    [currentAgent]
  );

  const deleteAgent = useCallback(
    async (id: string) => {
      try {
        setLoading(true);
        await agentApi.deleteAgent(id);
        setAgents((prev) => prev.filter((a) => a.id !== id));
        if (currentAgent?.id === id) {
          setCurrentAgent(agents.find((a) => a.id !== id) || null);
        }
        return true;
      } catch (err) {
        const msg = err instanceof Error ? err.message : "删除Agent失败";
        setError(msg);
        // 仍然在前端删除
        setAgents((prev) => prev.filter((a) => a.id !== id));
        if (currentAgent?.id === id) {
          setCurrentAgent(agents.find((a) => a.id !== id) || null);
        }
        return true;
      } finally {
        setLoading(false);
      }
    },
    [currentAgent, agents]
  );

  const selectAgent = useCallback(
    (agentId: string) => {
      const agent = agents.find((a) => a.id === agentId) || null;
      setCurrentAgent(agent);
    },
    [agents]
  );

  return {
    agents,
    currentAgent,
    loading,
    error,
    createAgent,
    updateAgent,
    deleteAgent,
    selectAgent,
    refresh: loadAgents,
  };
}
