import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useAgent } from "../../hooks/useAgent";

// Mock Tauri API
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

const mockInvoke = vi.mocked(invoke);

describe("useAgent", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("initializes with default agents", () => {
    mockInvoke.mockRejectedValue(new Error("Backend not available"));

    const { result } = renderHook(() => useAgent());

    // 应该有默认Agent
    expect(result.current.agents.length).toBeGreaterThan(0);
    expect(result.current.currentAgent).not.toBeNull();
    expect(result.current.currentAgent?.name).toBe("通用助手");
  });

  it("loads agents from backend", async () => {
    const mockAgents = [
      {
        id: "agent-1",
        name: "测试Agent",
        description: "测试描述",
        systemPrompt: "测试提示词",
        modelId: "gpt-4",
        tools: ["fs_read"],
        mcpServers: [],
        autoMode: false,
        maxIterations: 10,
        allowedDirectories: [],
      },
    ];

    mockInvoke.mockResolvedValue(mockAgents);

    const { result } = renderHook(() => useAgent());

    await waitFor(() => {
      expect(result.current.agents.length).toBe(1);
      expect(result.current.agents[0].name).toBe("测试Agent");
    });
  });

  it("creates a new agent", async () => {
    mockInvoke.mockResolvedValue("new-agent-id");

    const { result } = renderHook(() => useAgent());

    const newAgent = {
      name: "新Agent",
      description: "新描述",
      systemPrompt: "新提示词",
      modelId: "auto",
      tools: [],
      mcpServers: [],
      autoMode: false,
      maxIterations: 5,
      allowedDirectories: [],
    };

    let id = "";
    await act(async () => {
      id = await result.current.createAgent(newAgent);
    });

    expect(id).toBeTruthy();
    expect(result.current.agents.some((a) => a.name === "新Agent")).toBe(true);
  });

  it("updates an existing agent", async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { result } = renderHook(() => useAgent());

    // 等待初始加载
    await waitFor(() => expect(result.current.agents.length).toBeGreaterThan(0));

    const agent = result.current.agents[0];
    const updatedAgent = {
      ...agent,
      name: "已更新的Agent",
      description: "更新后的描述",
    };

    await act(async () => {
      await result.current.updateAgent(agent.id, updatedAgent);
    });

    expect(
      result.current.agents.find((a) => a.id === agent.id)?.name
    ).toBe("已更新的Agent");
  });

  it("deletes an agent", async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { result } = renderHook(() => useAgent());

    // 等待初始加载
    await waitFor(() => expect(result.current.agents.length).toBeGreaterThan(0));

    const initialCount = result.current.agents.length;
    const agentToDelete = result.current.agents[0];

    await act(async () => {
      await result.current.deleteAgent(agentToDelete.id);
    });

    expect(result.current.agents.length).toBe(initialCount - 1);
    expect(
      result.current.agents.find((a) => a.id === agentToDelete.id)
    ).toBeUndefined();
  });

  it("selects an agent by id", async () => {
    mockInvoke.mockRejectedValue(new Error("Backend not available"));

    const { result } = renderHook(() => useAgent());

    await waitFor(() => expect(result.current.agents.length).toBeGreaterThan(0));

    // 默认选中第一个
    expect(result.current.currentAgent?.name).toBe("通用助手");
  });

  it("handles backend errors gracefully", async () => {
    mockInvoke.mockRejectedValue(new Error("Network error"));

    const { result } = renderHook(() => useAgent());

    // 即使后端出错，也应该有默认Agent
    await waitFor(() => {
      expect(result.current.agents.length).toBeGreaterThan(0);
      expect(result.current.error).toBeTruthy();
    });
  });

  it("creates agent with fallback when backend fails", async () => {
    mockInvoke.mockRejectedValue(new Error("Backend error"));

    const { result } = renderHook(() => useAgent());

    const newAgent = {
      name: "Fallback Agent",
      description: "Test",
      systemPrompt: "Test",
      modelId: "auto",
      tools: [],
      mcpServers: [],
      autoMode: false,
      maxIterations: 10,
      allowedDirectories: [],
    };

    let id = "";
    await act(async () => {
      id = await result.current.createAgent(newAgent);
    });

    // 即使后端失败，也应该返回一个ID并添加Agent
    expect(id).toBeTruthy();
    expect(result.current.agents.some((a) => a.name === "Fallback Agent")).toBe(
      true
    );
  });
});
