import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import AgentIDE from "../components/AgentIDE";

// Mock Tauri API
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

describe("AgentIDE", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders AgentIDE with correct layout structure", () => {
    render(<AgentIDE />);

    // 顶部栏元素
    expect(screen.getByTitle("切换侧边栏")).toBeInTheDocument();
    expect(screen.getByText("新建会话")).toBeInTheDocument();
    expect(screen.getByTitle("Agent设置")).toBeInTheDocument();
    expect(screen.getByTitle("切换工具面板")).toBeInTheDocument();
  });

  it("renders ModelSelector in top bar", () => {
    render(<AgentIDE />);

    // 模型选择器应该存在 (Brain图标)
    const brainButton = screen.getByTitle("选择模型");
    expect(brainButton).toBeInTheDocument();
  });

  it("renders sidebar with sessions and agents", async () => {
    render(<AgentIDE />);

    // 侧边栏应该有会话和Agent标签 - 使用精确匹配避免匹配多个元素
    expect(screen.getByText(/会话 \(\d+\)/i)).toBeInTheDocument();
    expect(screen.getByText(/Agent \(\d+\)/i)).toBeInTheDocument();
  });

  it("renders chat panel with empty state", () => {
    render(<AgentIDE />);

    // 空状态提示
    expect(screen.getByText("AI 助手")).toBeInTheDocument();
    expect(
      screen.getByText(/我可以帮你编写代码/i)
    ).toBeInTheDocument();
  });

  it("renders status bar", () => {
    render(<AgentIDE />);

    // 状态栏元素
    expect(screen.getByText("AI Agent IDE v0.1.0")).toBeInTheDocument();
  });

  it("toggles sidebar visibility", () => {
    render(<AgentIDE />);

    const toggleBtn = screen.getByTitle("切换侧边栏");

    // 侧边栏默认可见 - 使用更精确的匹配
    expect(screen.getByText(/会话 \(\d+\)/i)).toBeInTheDocument();

    // 点击隐藏侧边栏
    fireEvent.click(toggleBtn);

    // 侧边栏应该隐藏 - 会话计数文字不再可见
    expect(screen.queryByText(/会话 \(\d+\)/i)).not.toBeInTheDocument();
  });

  it("toggles right panel visibility", () => {
    render(<AgentIDE />);

    const toggleBtn = screen.getByTitle("切换工具面板");

    // 默认不可见

    // 点击显示右侧面板
    fireEvent.click(toggleBtn);

    // MCP标签应该出现
    expect(screen.getByText("MCP")).toBeInTheDocument();
  });

  it("has working new session button", () => {
    render(<AgentIDE />);

    const newSessionBtn = screen.getByText("新建会话");
    expect(newSessionBtn).toBeInTheDocument();

    fireEvent.click(newSessionBtn);

    // 新建会话后应该显示聊天输入框
    expect(
      screen.getByPlaceholderText(/输入消息/i)
    ).toBeInTheDocument();
  });

  it("renders agent selector dropdown", () => {
    render(<AgentIDE />);

    // Agent选择器应该存在
    const select = screen.getByDisplayValue("通用助手");
    expect(select).toBeInTheDocument();
  });
});
