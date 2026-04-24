import { useState, useCallback } from "react";
import {
  PanelLeft,
  PanelRight,
  Plus,
  Settings,
  Brain,
} from "lucide-react";
import { useModels } from "../hooks/useModels";
import { useAgent } from "../hooks/useAgent";
import { useChat } from "../hooks/useChat";
import { useTools } from "../hooks/useTools";
import ModelSelector from "./ModelSelector";
import ChatPanel from "./ChatPanel";
import Sidebar from "./Sidebar";
import ToolPanel from "./ToolPanel";
import StatusBar from "./StatusBar";
import AgentConfigModal from "./AgentConfigModal";
import type { AgentDefinition } from "../types";

export default function AgentIDE() {
  const [sidebarVisible, setSidebarVisible] = useState(true);
  const [rightPanelVisible, setRightPanelVisible] = useState(false);
  const [configModalOpen, setConfigModalOpen] = useState(false);
  const [editingAgent, setEditingAgent] = useState<AgentDefinition | null>(null);

  const { models, activeModel, setActiveModel } = useModels();
  const {
    agents,
    currentAgent,
    createAgent,
    updateAgent,
    deleteAgent,
    selectAgent,
  } = useAgent();
  const {
    sessions,
    currentSession,
    isLoading,
    sendMessage,
    cancelChat,
    createSession,
    selectSession,
    deleteSession,
  } = useChat(currentAgent?.id);
  const { mcpServers, loadMcpServers, testMcpServer } = useTools();

  // 打开MCP面板时加载
  const handleToggleRightPanel = useCallback(() => {
    setRightPanelVisible((prev) => {
      const next = !prev;
      if (next) loadMcpServers();
      return next;
    });
  }, [loadMcpServers]);

  const handleNewAgent = useCallback(() => {
    setEditingAgent(null);
    setConfigModalOpen(true);
  }, []);

  const handleEditAgent = useCallback((agent: AgentDefinition) => {
    setEditingAgent(agent);
    setConfigModalOpen(true);
  }, []);

  const handleSaveAgent = useCallback(
    async (agent: AgentDefinition) => {
      if (editingAgent) {
        await updateAgent(agent.id, agent);
      } else {
        await createAgent(agent);
      }
      setConfigModalOpen(false);
      setEditingAgent(null);
    },
    [editingAgent, createAgent, updateAgent]
  );

  const handleNewSession = useCallback(() => {
    createSession(currentAgent?.id);
  }, [createSession, currentAgent]);

  return (
    <div className="h-full flex flex-col bg-bg dark:bg-bg-dark">
      {/* 顶部栏 */}
      <div className="h-11 shrink-0 flex items-center gap-2 px-3 border-b border-gray-200/50 dark:border-gray-800/50 bg-surface dark:bg-surface-dark">
        <button
          onClick={() => setSidebarVisible(!sidebarVisible)}
          className={`p-1.5 rounded-lg transition-colors ${
            sidebarVisible
              ? "bg-brand-primary/10 text-brand-primary"
              : "text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
          }`}
          title="切换侧边栏"
        >
          <PanelLeft className="w-4 h-4" />
        </button>

        <ModelSelector
          models={models}
          activeModel={activeModel}
          onChange={setActiveModel}
          onManage={() => {}}
        />

        <div className="w-px h-5 bg-gray-200 dark:bg-gray-700" />

        {/* Agent选择 */}
        <div className="flex items-center gap-1.5">
          <Brain className="w-4 h-4 text-gray-400" />
          <select
            value={currentAgent?.id || ""}
            onChange={(e) => selectAgent(e.target.value)}
            className="text-xs bg-transparent border-none outline-none text-gray-700 dark:text-gray-300 cursor-pointer"
          >
            {agents.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </select>
        </div>

        <div className="flex-1" />

        <button
          onClick={handleNewSession}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-brand-primary/10 text-brand-primary hover:bg-brand-primary/20 transition-colors text-xs font-medium"
          title="新建会话"
        >
          <Plus className="w-3.5 h-3.5" />
          新建会话
        </button>

        <button
          onClick={() => currentAgent && handleEditAgent(currentAgent)}
          className="p-1.5 rounded-lg text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
          title="Agent设置"
        >
          <Settings className="w-4 h-4" />
        </button>

        <button
          onClick={handleToggleRightPanel}
          className={`p-1.5 rounded-lg transition-colors ${
            rightPanelVisible
              ? "bg-brand-primary/10 text-brand-primary"
              : "text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
          }`}
          title="切换工具面板"
        >
          <PanelRight className="w-4 h-4" />
        </button>
      </div>

      {/* 主内容 */}
      <div className="flex-1 flex overflow-hidden">
        {sidebarVisible && (
          <Sidebar
            sessions={sessions}
            agents={agents}
            currentSessionId={currentSession?.id}
            currentAgentId={currentAgent?.id}
            onSelectSession={selectSession}
            onSelectAgent={selectAgent}
            onNewSession={handleNewSession}
            onNewAgent={handleNewAgent}
            onDeleteSession={deleteSession}
            onDeleteAgent={deleteAgent}
          />
        )}

        <ChatPanel
          session={currentSession || { id: "", agentId: "", title: "", messages: [], createdAt: 0, updatedAt: 0 }}
          onSendMessage={sendMessage}
          onCancel={cancelChat}
          isLoading={isLoading}
        />

        {rightPanelVisible && (
          <ToolPanel
            mcpServers={mcpServers}
            onTestMcpServer={testMcpServer}
          />
        )}
      </div>

      {/* 状态栏 */}
      <StatusBar
        modelName={activeModel === "auto" ? "自动路由" : models.find((m) => m.id === activeModel)?.name || activeModel}
        agentName={currentAgent?.name || ""}
        connected={true}
        tokenCount={currentSession?.messages.reduce((acc, m) => acc + m.content.length, 0)}
      />

      {/* Agent配置弹窗 */}
      <AgentConfigModal
        open={configModalOpen}
        agent={editingAgent}
        models={models}
        mcpServers={mcpServers}
        onSave={handleSaveAgent}
        onClose={() => {
          setConfigModalOpen(false);
          setEditingAgent(null);
        }}
      />
    </div>
  );
}
