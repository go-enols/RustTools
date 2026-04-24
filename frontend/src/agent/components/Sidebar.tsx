import { useState } from "react";
import {
  MessageSquare,
  Plus,
  Bot,
  Trash2,
  ChevronDown,
  ChevronRight,
  Clock,
} from "lucide-react";
import type { SidebarProps, ChatSession, AgentDefinition } from "../types";

export default function Sidebar({
  sessions,
  agents,
  currentSessionId,
  currentAgentId,
  onSelectSession,
  onSelectAgent,
  onNewSession,
  onNewAgent,
  onDeleteSession,
  onDeleteAgent,
}: SidebarProps) {
  const [sessionsExpanded, setSessionsExpanded] = useState(true);
  const [agentsExpanded, setAgentsExpanded] = useState(true);

  return (
    <div className="w-56 shrink-0 bg-surface dark:bg-surface-dark border-r border-gray-200/50 dark:border-gray-800/50 flex flex-col overflow-hidden">
      {/* 会话列表 */}
      <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
        {/* 会话头部 */}
        <div className="flex items-center justify-between px-3 py-2 border-b border-gray-100 dark:border-gray-800">
          <button
            onClick={() => setSessionsExpanded(!sessionsExpanded)}
            className="flex items-center gap-1 text-[10px] font-medium text-gray-400 uppercase tracking-wider hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
          >
            {sessionsExpanded ? (
              <ChevronDown className="w-3 h-3" />
            ) : (
              <ChevronRight className="w-3 h-3" />
            )}
            会话 ({sessions.length})
          </button>
          <button
            onClick={onNewSession}
            className="p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-400 hover:text-brand-primary transition-colors"
            title="新建会话"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
        </div>

        {sessionsExpanded && (
          <div className="flex-1 overflow-auto py-1">
            {sessions.length === 0 ? (
              <div className="text-center text-[11px] text-gray-400 py-6 px-4">
                暂无会话
                <br />
                点击 + 新建会话
              </div>
            ) : (
              <div className="space-y-0.5 px-1.5">
                {sessions.map((session) => (
                  <SessionItem
                    key={session.id}
                    session={session}
                    active={session.id === currentSessionId}
                    onClick={() => onSelectSession(session.id)}
                    onDelete={() => onDeleteSession(session.id)}
                  />
                ))}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Agent列表 */}
      <div className="border-t border-gray-200/50 dark:border-gray-800/50">
        <div className="flex items-center justify-between px-3 py-2 border-b border-gray-100 dark:border-gray-800">
          <button
            onClick={() => setAgentsExpanded(!agentsExpanded)}
            className="flex items-center gap-1 text-[10px] font-medium text-gray-400 uppercase tracking-wider hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
          >
            {agentsExpanded ? (
              <ChevronDown className="w-3 h-3" />
            ) : (
              <ChevronRight className="w-3 h-3" />
            )}
            Agent ({agents.length})
          </button>
          <button
            onClick={onNewAgent}
            className="p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-400 hover:text-brand-primary transition-colors"
            title="新建Agent"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
        </div>

        {agentsExpanded && (
          <div className="max-h-48 overflow-auto py-1">
            <div className="space-y-0.5 px-1.5">
              {agents.map((agent) => (
                <AgentItem
                  key={agent.id}
                  agent={agent}
                  active={agent.id === currentAgentId}
                  onClick={() => onSelectAgent(agent.id)}
                  onDelete={() => onDeleteAgent(agent.id)}
                />
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function SessionItem({
  session,
  active,
  onClick,
  onDelete,
}: {
  session: ChatSession;
  active: boolean;
  onClick: () => void;
  onDelete: () => void;
}) {
  const [showDelete, setShowDelete] = useState(false);

  return (
    <div
      className="group relative"
      onMouseEnter={() => setShowDelete(true)}
      onMouseLeave={() => setShowDelete(false)}
    >
      <button
        onClick={onClick}
        className={`w-full flex items-start gap-2 px-2 py-2 rounded-xl transition-all text-left ${
          active
            ? "bg-brand-primary/10 text-brand-primary"
            : "hover:bg-gray-50 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300"
        }`}
      >
        <MessageSquare className="w-3.5 h-3.5 shrink-0 mt-0.5" />
        <div className="flex-1 min-w-0">
          <div className="text-xs font-medium truncate">{session.title}</div>
          <div className="flex items-center gap-1 mt-0.5">
            <Clock className="w-2.5 h-2.5 text-gray-400" />
            <span className="text-[10px] text-gray-400">
              {new Date(session.updatedAt).toLocaleDateString("zh-CN")}
            </span>
            <span className="text-[10px] text-gray-300 dark:text-gray-600 ml-1">
              {session.messages.length} 条
            </span>
          </div>
        </div>
      </button>
      {showDelete && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          className="absolute right-1.5 top-1/2 -translate-y-1/2 p-1 rounded-md hover:bg-brand-danger/10 text-gray-300 hover:text-brand-danger transition-colors"
        >
          <Trash2 className="w-3 h-3" />
        </button>
      )}
    </div>
  );
}

function AgentItem({
  agent,
  active,
  onClick,
  onDelete,
}: {
  agent: AgentDefinition;
  active: boolean;
  onClick: () => void;
  onDelete: () => void;
}) {
  const [showDelete, setShowDelete] = useState(false);

  return (
    <div
      className="group relative"
      onMouseEnter={() => setShowDelete(true)}
      onMouseLeave={() => setShowDelete(false)}
    >
      <button
        onClick={onClick}
        className={`w-full flex items-center gap-2 px-2 py-2 rounded-xl transition-all text-left ${
          active
            ? "bg-brand-primary/10 text-brand-primary"
            : "hover:bg-gray-50 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300"
        }`}
      >
        <Bot className="w-3.5 h-3.5 shrink-0" />
        <div className="flex-1 min-w-0">
          <div className="text-xs font-medium truncate">{agent.name}</div>
          <div className="text-[10px] text-gray-400 truncate">
            {agent.description}
          </div>
        </div>
        {agent.autoMode && (
          <span className="text-[9px] px-1 py-0.5 rounded bg-brand-primary/10 text-brand-primary shrink-0">
            Auto
          </span>
        )}
      </button>
      {showDelete && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          className="absolute right-1.5 top-1/2 -translate-y-1/2 p-1 rounded-md hover:bg-brand-danger/10 text-gray-300 hover:text-brand-danger transition-colors"
        >
          <Trash2 className="w-3 h-3" />
        </button>
      )}
    </div>
  );
}
