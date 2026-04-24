import { invoke } from "@tauri-apps/api/core";
import type {
  ModelConfig,
  AgentDefinition,
  McpServerConfig,
  McpServerInfo,
  AgentConfig,
} from "./types";

export const agentApi = {
  // 模型管理
  getModels: () => invoke<ModelConfig[]>("agent_get_models"),

  addModel: (config: ModelConfig) =>
    invoke<void>("agent_add_model", { config }),

  removeModel: (id: string) => invoke<void>("agent_remove_model", { id }),

  testModel: (id: string) =>
    invoke<{ success: boolean; latency: number }>("agent_test_model", { id }),

  // Agent管理
  listAgents: () => invoke<AgentDefinition[]>("agent_list_agents"),

  createAgent: (agent: Omit<AgentDefinition, "id">) =>
    invoke<string>("agent_create_agent", { agent }),

  updateAgent: (id: string, agent: AgentDefinition) =>
    invoke<void>("agent_update_agent", { id, agent }),

  deleteAgent: (id: string) => invoke<void>("agent_delete_agent", { id }),

  // 聊天
  sendMessage: (sessionId: string, message: string, agentId: string) =>
    invoke<void>("agent_send_message", { sessionId, message, agentId }),

  cancelChat: (sessionId: string) =>
    invoke<void>("agent_cancel_chat", { sessionId }),

  // MCP管理
  getMcpServers: () => invoke<McpServerInfo[]>("agent_get_mcp_servers"),

  addMcpServer: (config: McpServerConfig) =>
    invoke<void>("agent_add_mcp_server", { config }),

  removeMcpServer: (name: string) =>
    invoke<void>("agent_remove_mcp_server", { name }),

  testMcpServer: (name: string) =>
    invoke<McpServerInfo>("agent_test_mcp_server", { name }),

  translateSkill: (skillJson: string, targetLang: string) =>
    invoke<string>("agent_translate_skill", { skillJson, targetLang }),

  // 配置
  loadConfig: () => invoke<AgentConfig>("agent_load_config"),

  saveConfig: (config: AgentConfig) =>
    invoke<void>("agent_save_config", { config }),
};
