// ===================== 模型相关 =====================

export type ModelProvider = 'openai' | 'anthropic' | 'gemini' | 'ollama' | 'openai_compatible';

export interface ModelConfig {
  id: string;
  name: string;
  provider: ModelProvider;
  apiKey?: string;
  baseUrl?: string;
  modelsList: string[];
  defaultModel: string;
  timeoutMs: number;
}

export interface RouterRule {
  condition: TaskCondition;
  targetModel: string;
  priority: number;
}

export interface TaskCondition {
  taskType?: 'Chat' | 'Code' | 'Analysis' | 'Image' | 'Long';
  complexity?: 'Simple' | 'Medium' | 'Complex';
  contextSize?: { min?: number; max?: number };
  requiredCapability?: string;
}

// ===================== Agent相关 =====================

export type AgentCapability =
  | 'CodeGeneration' | 'CodeReview' | 'Testing'
  | 'Documentation' | 'Analysis' | 'Planning';

export interface AgentDefinition {
  id: string;
  name: string;
  description: string;
  systemPrompt: string;
  modelId: string;
  tools: string[];
  mcpServers: string[];
  autoMode: boolean;
  maxIterations: number;
  allowedDirectories: string[];
}

// ===================== 聊天相关 =====================

export type MessageRole = 'user' | 'assistant' | 'system' | 'tool';

export interface ToolCall {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
  result?: string;
  status: 'pending' | 'success' | 'error';
}

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  toolCalls?: ToolCall[];
  timestamp: number;
}

export interface ChatSession {
  id: string;
  agentId: string;
  title: string;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
}

// ===================== MCP相关 =====================

export type McpTransport = 'stdio' | 'sse' | 'websocket';

export interface McpServerConfig {
  name: string;
  transport: McpTransport;
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  url?: string;
  enabled: boolean;
}

export interface McpServerInfo {
  name: string;
  transport: string;
  command: string;
  status: 'Disconnected' | 'Connecting' | 'Connected' | string;
  toolCount: number;
  resourceCount: number;
  lastError?: string;
}

// ===================== 全局配置 =====================

export interface SkillConfig {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
}

export interface AgentConfig {
  version: string;
  activeModel: string;
  models: ModelConfig[];
  autoRouterRules: RouterRule[];
  agents: AgentDefinition[];
  mcpServers: McpServerConfig[];
  skills: SkillConfig[];
}

// ===================== 组件Props =====================

export interface ModelSelectorProps {
  models: ModelConfig[];
  activeModel: string;
  onChange: (modelId: string) => void;
  onManage: () => void;
}

export interface ChatPanelProps {
  session: ChatSession;
  onSendMessage: (content: string) => void;
  onCancel: () => void;
  isLoading: boolean;
}

export interface MessageRendererProps {
  content: string;
  toolCalls?: ToolCall[];
}

export interface StatusBarProps {
  modelName: string;
  agentName: string;
  connected: boolean;
  tokenCount?: number;
  taskProgress?: { current: number; total: number };
}

export interface AgentConfigModalProps {
  open: boolean;
  agent?: AgentDefinition | null;
  models: ModelConfig[];
  mcpServers: McpServerInfo[];
  onSave: (agent: AgentDefinition) => void;
  onClose: () => void;
}

export interface ToolPanelProps {
  mcpServers: McpServerInfo[];
  onTestMcpServer?: (name: string) => void;
}

export interface SidebarProps {
  sessions: ChatSession[];
  agents: AgentDefinition[];
  currentSessionId?: string;
  currentAgentId?: string;
  onSelectSession: (sessionId: string) => void;
  onSelectAgent: (agentId: string) => void;
  onNewSession: () => void;
  onNewAgent: () => void;
  onDeleteSession: (sessionId: string) => void;
  onDeleteAgent: (agentId: string) => void;
}
