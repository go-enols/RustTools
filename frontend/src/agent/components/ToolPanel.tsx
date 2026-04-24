import { useState } from "react";
import {
  Terminal,
  Server,
  FolderOpen,
  ChevronDown,
  ChevronRight,
  Wifi,
  WifiOff,
  RefreshCw,
  Wrench,
} from "lucide-react";
import type { ToolPanelProps } from "../types";
import { BUILTIN_TOOLS } from "../hooks/useTools";

type TabKey = "terminal" | "mcp" | "files";

export default function ToolPanel({
  mcpServers,
  onTestMcpServer,
}: ToolPanelProps) {
  const [activeTab, setActiveTab] = useState<TabKey>("mcp");

  const tabs: { key: TabKey; label: string; icon: React.ReactNode }[] = [
    { key: "terminal", label: "终端", icon: <Terminal className="w-3.5 h-3.5" /> },
    { key: "mcp", label: "MCP", icon: <Server className="w-3.5 h-3.5" /> },
    { key: "files", label: "文件", icon: <FolderOpen className="w-3.5 h-3.5" /> },
  ];

  return (
    <div className="w-72 shrink-0 bg-surface dark:bg-surface-dark border-l border-gray-200/50 dark:border-gray-800/50 flex flex-col">
      {/* 标签页 */}
      <div className="flex items-center border-b border-gray-200/50 dark:border-gray-800/50">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={`flex-1 flex items-center justify-center gap-1.5 py-2.5 text-xs font-medium transition-colors border-b-2 ${
              activeTab === tab.key
                ? "border-brand-primary text-brand-primary"
                : "border-transparent text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
            }`}
          >
            {tab.icon}
            {tab.label}
          </button>
        ))}
      </div>

      {/* 内容 */}
      <div className="flex-1 overflow-auto">
        {activeTab === "terminal" && <TerminalTab />}
        {activeTab === "mcp" && (
          <McpTab servers={mcpServers} onTest={onTestMcpServer} />
        )}
        {activeTab === "files" && <FilesTab />}
      </div>
    </div>
  );
}

function TerminalTab() {
  const [commands] = useState<
    { command: string; output: string; timestamp: number }[]
  >([
    {
      command: "ls -la",
      output: "total 128\ndrwxr-xr-x  12 user staff  384 Jan 15 10:30 .",
      timestamp: Date.now(),
    },
  ]);

  return (
    <div className="p-3">
      {commands.length === 0 ? (
        <div className="text-center text-xs text-gray-400 py-8">
          暂无命令记录
        </div>
      ) : (
        <div className="space-y-2">
          {commands.map((cmd, i) => (
            <div
              key={i}
              className="rounded-xl bg-gray-50 dark:bg-gray-900 border border-gray-100 dark:border-gray-800 overflow-hidden"
            >
              <div className="px-3 py-1.5 bg-gray-100 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 flex items-center gap-2">
                <Terminal className="w-3 h-3 text-gray-400" />
                <span className="text-[11px] font-mono text-gray-600 dark:text-gray-400 truncate">
                  {cmd.command}
                </span>
              </div>
              <pre className="p-2.5 text-[11px] font-mono text-gray-600 dark:text-gray-400 overflow-auto max-h-32">
                {cmd.output}
              </pre>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function McpTab({
  servers,
  onTest,
}: {
  servers: ToolPanelProps["mcpServers"];
  onTest?: (name: string) => void;
}) {
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});

  const toggleExpanded = (name: string) => {
    setExpanded((prev) => ({ ...prev, [name]: !prev[name] }));
  };

  return (
    <div className="p-3">
      {/* 内置工具 */}
      <div className="mb-4">
        <div className="text-[10px] font-medium text-gray-400 uppercase tracking-wider mb-2 px-1">
          内置工具
        </div>
        <div className="space-y-1">
          {BUILTIN_TOOLS.map((tool) => (
            <div
              key={tool.id}
              className="flex items-center gap-2 px-2 py-1.5 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800"
            >
              <Wrench className="w-3 h-3 text-gray-400" />
              <div className="flex-1 min-w-0">
                <div className="text-xs font-medium text-gray-700 dark:text-gray-300 truncate">
                  {tool.name}
                </div>
                <div className="text-[10px] text-gray-400">{tool.description}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* MCP服务器 */}
      <div>
        <div className="text-[10px] font-medium text-gray-400 uppercase tracking-wider mb-2 px-1 flex items-center justify-between">
          <span>MCP 服务器</span>
          <span className="text-gray-300">{servers.length}</span>
        </div>
        {servers.length === 0 ? (
          <div className="text-center text-xs text-gray-400 py-4">
            暂无MCP服务器
          </div>
        ) : (
          <div className="space-y-1">
            {servers.map((server) => (
              <div
                key={server.name}
                className="rounded-lg border border-gray-100 dark:border-gray-800 overflow-hidden"
              >
                <button
                  onClick={() => toggleExpanded(server.name)}
                  className="w-full flex items-center gap-2 px-2 py-2 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                >
                  {expanded[server.name] ? (
                    <ChevronDown className="w-3 h-3 text-gray-400" />
                  ) : (
                    <ChevronRight className="w-3 h-3 text-gray-400" />
                  )}
                  {server.status === "Connected" ? (
                    <Wifi className="w-3 h-3 text-brand-success" />
                  ) : (
                    <WifiOff className="w-3 h-3 text-brand-danger" />
                  )}
                  <span className="text-xs font-medium text-gray-700 dark:text-gray-300 flex-1 text-left truncate">
                    {server.name}
                  </span>
                  {onTest && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onTest(server.name);
                      }}
                      className="p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700"
                      title="测试连接"
                    >
                      <RefreshCw className="w-3 h-3 text-gray-400" />
                    </button>
                  )}
                </button>
                {expanded[server.name] && (
                  <div className="px-2.5 pb-2 border-t border-gray-50 dark:border-gray-800">
                    <div className="mt-1.5 space-y-1 text-[11px] text-gray-500 dark:text-gray-400">
                      <div className="flex justify-between">
                        <span>传输</span>
                        <span>{server.transport}</span>
                      </div>
                      <div className="flex justify-between">
                        <span>状态</span>
                        <span
                          className={
                            server.status === "Connected"
                              ? "text-brand-success"
                              : "text-brand-danger"
                          }
                        >
                          {server.status}
                        </span>
                      </div>
                      <div className="flex justify-between">
                        <span>工具数</span>
                        <span>{server.toolCount}</span>
                      </div>
                      <div className="flex justify-between">
                        <span>资源数</span>
                        <span>{server.resourceCount}</span>
                      </div>
                      {server.lastError && (
                        <div className="text-brand-danger text-[10px] truncate">
                          {server.lastError}
                        </div>
                      )}
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function FilesTab() {
  const [expanded, setExpanded] = useState<Record<string, boolean>>({
    project: true,
  });

  const toggle = (key: string) => {
    setExpanded((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  return (
    <div className="p-3">
      <div className="text-[10px] font-medium text-gray-400 uppercase tracking-wider mb-2 px-1">
        项目文件
      </div>
      <div className="space-y-0.5">
        <FileTreeItem
          name="src"
          type="folder"
          expanded={expanded["src"]}
          onToggle={() => toggle("src")}
          depth={0}
        >
          <FileTreeItem name="main.rs" type="file" depth={1} />
          <FileTreeItem name="lib.rs" type="file" depth={1} />
          <FileTreeItem
            name="agent"
            type="folder"
            expanded={expanded["agent"]}
            onToggle={() => toggle("agent")}
            depth={1}
          >
            <FileTreeItem name="mod.rs" type="file" depth={2} />
            <FileTreeItem name="api_client" type="folder" depth={2} />
          </FileTreeItem>
        </FileTreeItem>
        <FileTreeItem name="Cargo.toml" type="file" depth={0} />
        <FileTreeItem name="README.md" type="file" depth={0} />
      </div>
    </div>
  );
}

function FileTreeItem({
  name,
  type,
  depth,
  expanded,
  onToggle,
  children,
}: {
  name: string;
  type: "folder" | "file";
  depth: number;
  expanded?: boolean;
  onToggle?: () => void;
  children?: React.ReactNode;
}) {
  return (
    <div>
      <button
        onClick={type === "folder" ? onToggle : undefined}
        className="w-full flex items-center gap-1.5 px-1.5 py-1 rounded-md hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-left"
        style={{ paddingLeft: `${depth * 12 + 6}px` }}
      >
        {type === "folder" ? (
          <>
            {expanded ? (
              <ChevronDown className="w-3 h-3 text-gray-400" />
            ) : (
              <ChevronRight className="w-3 h-3 text-gray-400" />
            )}
            <FolderOpen className="w-3.5 h-3.5 text-brand-warning" />
          </>
        ) : (
          <>
            <div className="w-3" />
            <div className="w-3.5 h-3.5 flex items-center justify-center">
              <div className="w-2 h-2 rounded-sm bg-gray-300 dark:bg-gray-600" />
            </div>
          </>
        )}
        <span className="text-xs text-gray-700 dark:text-gray-300 truncate">
          {name}
        </span>
      </button>
      {expanded && children}
    </div>
  );
}
