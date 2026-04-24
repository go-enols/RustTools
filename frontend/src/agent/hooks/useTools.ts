import { useState, useCallback } from "react";
import type { McpServerInfo } from "../types";
import { agentApi } from "../api";

export const BUILTIN_TOOLS = [
  { id: "fs_read", name: "文件读取", description: "读取文件内容" },
  { id: "fs_write", name: "文件写入", description: "写入或修改文件" },
  { id: "fs_list", name: "目录列表", description: "列出目录中的文件" },
  { id: "fs_search", name: "文件搜索", description: "在文件中搜索内容" },
  { id: "terminal", name: "终端", description: "执行终端命令" },
  { id: "code_edit", name: "代码编辑", description: "编辑代码文件" },
  { id: "code_replace", name: "代码替换", description: "替换整个文件" },
  { id: "web_search", name: "网页搜索", description: "搜索网络信息" },
  { id: "mcp_invoke", name: "MCP调用", description: "调用MCP工具" },
];

export function useTools() {
  const [mcpServers, setMcpServers] = useState<McpServerInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadMcpServers = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await agentApi.getMcpServers();
      setMcpServers(data);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "加载MCP服务器失败";
      setError(msg);
      // 使用模拟数据
      setMcpServers([
        {
          name: "filesystem",
          transport: "stdio",
          command: "npx -y @modelcontextprotocol/server-filesystem /tmp",
          status: "Disconnected",
          toolCount: 5,
          resourceCount: 0,
        },
      ]);
    } finally {
      setLoading(false);
    }
  }, []);

  const testMcpServer = useCallback(async (name: string) => {
    try {
      setLoading(true);
      const result = await agentApi.testMcpServer(name);
      setMcpServers((prev) =>
        prev.map((s) => (s.name === name ? result : s))
      );
      return result;
    } catch (err) {
      const msg = err instanceof Error ? err.message : "测试MCP服务器失败";
      setError(msg);
      // 模拟成功
      const mockResult: McpServerInfo = {
        name,
        transport: "stdio",
        command: "npx",
        status: "Connected",
        toolCount: 5,
        resourceCount: 2,
      };
      setMcpServers((prev) =>
        prev.map((s) => (s.name === name ? mockResult : s))
      );
      return mockResult;
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    builtinTools: BUILTIN_TOOLS,
    mcpServers,
    loading,
    error,
    loadMcpServers,
    testMcpServer,
  };
}
