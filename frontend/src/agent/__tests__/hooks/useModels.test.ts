import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useModels } from "../../hooks/useModels";

// Mock Tauri API
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

const mockInvoke = vi.mocked(invoke);

describe("useModels", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("initializes with empty models and loading", () => {
    mockInvoke.mockRejectedValue(new Error("Backend not available"));

    const { result } = renderHook(() => useModels());

    expect(result.current.loading).toBe(true);
    expect(result.current.activeModel).toBe("auto");
  });

  it("loads models from backend", async () => {
    const mockModels = [
      {
        id: "gpt-4",
        name: "GPT-4",
        provider: "openai" as const,
        defaultModel: "gpt-4",
        modelsList: ["gpt-4"],
        timeoutMs: 30000,
      },
      {
        id: "claude-3",
        name: "Claude 3",
        provider: "anthropic" as const,
        defaultModel: "claude-3-sonnet",
        modelsList: ["claude-3-sonnet"],
        timeoutMs: 30000,
      },
    ];

    mockInvoke.mockResolvedValue(mockModels);

    const { result } = renderHook(() => useModels());

    await waitFor(() => {
      expect(result.current.models.length).toBe(2);
      expect(result.current.models[0].name).toBe("GPT-4");
      expect(result.current.models[1].name).toBe("Claude 3");
      expect(result.current.loading).toBe(false);
    });
  });

  it("uses default models when backend fails", async () => {
    mockInvoke.mockRejectedValue(new Error("Backend not available"));

    const { result } = renderHook(() => useModels());

    await waitFor(() => {
      // 应该有默认模型数据
      expect(result.current.models.length).toBeGreaterThan(0);
      expect(result.current.error).toBeTruthy();
    });

    // 验证默认模型
    expect(result.current.models.some((m) => m.provider === "openai")).toBe(
      true
    );
    expect(result.current.models.some((m) => m.provider === "ollama")).toBe(
      true
    );
  });

  it("allows changing active model", async () => {
    mockInvoke.mockRejectedValue(new Error("Backend not available"));

    const { result } = renderHook(() => useModels());

    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setActiveModel("gpt-4");
    });

    expect(result.current.activeModel).toBe("gpt-4");
  });

  it("adds a new model", async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { result } = renderHook(() => useModels());

    await waitFor(() => expect(result.current.loading).toBe(false));

    const newModel = {
      id: "custom-model",
      name: "自定义模型",
      provider: "openai_compatible" as const,
      baseUrl: "http://localhost:8080",
      defaultModel: "custom",
      modelsList: ["custom"],
      timeoutMs: 30000,
    };

    await act(async () => {
      await result.current.addModel(newModel);
    });

    expect(
      result.current.models.some((m) => m.id === "custom-model")
    ).toBe(true);
  });

  it("removes a model", async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { result } = renderHook(() => useModels());

    await waitFor(() => expect(result.current.loading).toBe(false));

    const initialCount = result.current.models.length;
    const modelToRemove = result.current.models[0];

    if (modelToRemove) {
      await act(async () => {
        await result.current.removeModel(modelToRemove.id);
      });

      expect(result.current.models.length).toBe(initialCount - 1);
      expect(
        result.current.models.find((m) => m.id === modelToRemove.id)
      ).toBeUndefined();
    }
  });

  it("tests a model connection", async () => {
    mockInvoke.mockResolvedValue({ success: true, latency: 120 });

    const { result } = renderHook(() => useModels());

    await waitFor(() => expect(result.current.loading).toBe(false));

    const testResult = await act(async () => {
      return result.current.testModel("gpt-4");
    });

    expect(testResult.success).toBe(true);
    expect(testResult.latency).toBeGreaterThan(0);
  });

  it("returns mock test result when backend fails", async () => {
    mockInvoke.mockRejectedValue(new Error("Not implemented"));

    const { result } = renderHook(() => useModels());

    await waitFor(() => expect(result.current.loading).toBe(false));

    const testResult = await act(async () => {
      return result.current.testModel("gpt-4");
    });

    expect(testResult.success).toBe(true);
    expect(testResult.latency).toBeGreaterThan(0);
  });
});
