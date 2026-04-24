import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import ModelSelector from "../components/ModelSelector";
import type { ModelConfig } from "../types";

const mockModels: ModelConfig[] = [
  {
    id: "gpt-4",
    name: "GPT-4",
    provider: "openai",
    defaultModel: "gpt-4",
    modelsList: ["gpt-4"],
    timeoutMs: 30000,
  },
  {
    id: "claude-3",
    name: "Claude 3",
    provider: "anthropic",
    defaultModel: "claude-3-sonnet",
    modelsList: ["claude-3-sonnet"],
    timeoutMs: 30000,
  },
  {
    id: "ollama-llama3",
    name: "Llama 3 本地",
    provider: "ollama",
    baseUrl: "http://localhost:11434",
    defaultModel: "llama3",
    modelsList: ["llama3"],
    timeoutMs: 60000,
  },
];

describe("ModelSelector", () => {
  const onChange = vi.fn();
  const onManage = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders with active model name", () => {
    render(
      <ModelSelector
        models={mockModels}
        activeModel="gpt-4"
        onChange={onChange}
        onManage={onManage}
      />
    );

    expect(screen.getByText("GPT-4")).toBeInTheDocument();
  });

  it("shows Auto when in auto mode", () => {
    render(
      <ModelSelector
        models={mockModels}
        activeModel="auto"
        onChange={onChange}
        onManage={onManage}
      />
    );

    expect(screen.getByText("Auto")).toBeInTheDocument();
  });

  it("opens dropdown when clicked", () => {
    render(
      <ModelSelector
        models={mockModels}
        activeModel="gpt-4"
        onChange={onChange}
        onManage={onManage}
      />
    );

    const button = screen.getByTitle("选择模型");
    fireEvent.click(button);

    // 下拉菜单中应该显示模型
    expect(screen.getByText("GPT-4")).toBeInTheDocument();
    expect(screen.getByText("Claude 3")).toBeInTheDocument();
    expect(screen.getByText("Llama 3 本地")).toBeInTheDocument();
  });

  it("switches model when clicked in dropdown", () => {
    render(
      <ModelSelector
        models={mockModels}
        activeModel="gpt-4"
        onChange={onChange}
        onManage={onManage}
      />
    );

    const button = screen.getByTitle("选择模型");
    fireEvent.click(button);

    // 点击Claude 3
    const claudeOption = screen.getByText("Claude 3");
    fireEvent.click(claudeOption);

    expect(onChange).toHaveBeenCalledWith("claude-3");
  });

  it("shows auto router option in dropdown", () => {
    render(
      <ModelSelector
        models={mockModels}
        activeModel="gpt-4"
        onChange={onChange}
        onManage={onManage}
      />
    );

    const button = screen.getByTitle("选择模型");
    fireEvent.click(button);

    expect(screen.getByText("自动路由")).toBeInTheDocument();
  });

  it("toggles auto mode when auto router is clicked", () => {
    render(
      <ModelSelector
        models={mockModels}
        activeModel="gpt-4"
        onChange={onChange}
        onManage={onManage}
      />
    );

    const button = screen.getByTitle("选择模型");
    fireEvent.click(button);

    // 点击自动路由
    const autoButton = screen.getByText("自动路由");
    fireEvent.click(autoButton);

    expect(onChange).toHaveBeenCalledWith("auto");
  });

  it("shows manage models button", () => {
    render(
      <ModelSelector
        models={mockModels}
        activeModel="gpt-4"
        onChange={onChange}
        onManage={onManage}
      />
    );

    const button = screen.getByTitle("选择模型");
    fireEvent.click(button);

    const manageBtn = screen.getByText("管理模型");
    expect(manageBtn).toBeInTheDocument();

    fireEvent.click(manageBtn);
    expect(onManage).toHaveBeenCalled();
  });

  it("shows router rules tooltip when auto mode is active", () => {
    render(
      <ModelSelector
        models={mockModels}
        activeModel="auto"
        onChange={onChange}
        onManage={onManage}
      />
    );

    // Auto模式应该显示路由规则提示
    expect(screen.getByText("自动路由规则")).toBeInTheDocument();
    expect(screen.getByText(/代码任务/i)).toBeInTheDocument();
    expect(screen.getByText(/简单对话/i)).toBeInTheDocument();
    expect(screen.getByText(/长文本/i)).toBeInTheDocument();
  });

  it("groups models by provider type", () => {
    render(
      <ModelSelector
        models={mockModels}
        activeModel="gpt-4"
        onChange={onChange}
        onManage={onManage}
      />
    );

    const button = screen.getByTitle("选择模型");
    fireEvent.click(button);

    // 检查分组标题
    expect(screen.getByText("云模型")).toBeInTheDocument();
    expect(screen.getByText("本地模型")).toBeInTheDocument();
  });

  it("closes dropdown when clicking outside", () => {
    render(
      <div>
        <div data-testid="outside">外部区域</div>
        <ModelSelector
          models={mockModels}
          activeModel="gpt-4"
          onChange={onChange}
          onManage={onManage}
        />
      </div>
    );

    // 打开下拉
    const button = screen.getByTitle("选择模型");
    fireEvent.click(button);
    expect(screen.getByText("管理模型")).toBeInTheDocument();

    // 点击外部
    fireEvent.mouseDown(screen.getByTestId("outside"));

    // 下拉菜单应该关闭
    waitFor(() => {
      expect(screen.queryByText("管理模型")).not.toBeInTheDocument();
    });
  });
});
