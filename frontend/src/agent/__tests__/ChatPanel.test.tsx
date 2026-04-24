import { describe, it, expect, vi, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import ChatPanel from "../components/ChatPanel";
import type { ChatSession } from "../types";

// Mock scrollIntoView for jsdom
beforeAll(() => {
  Element.prototype.scrollIntoView = vi.fn();
});

const mockSession: ChatSession = {
  id: "test-session",
  agentId: "test-agent",
  title: "测试会话",
  messages: [],
  createdAt: Date.now(),
  updatedAt: Date.now(),
};

const mockSessionWithMessages: ChatSession = {
  id: "test-session-2",
  agentId: "test-agent",
  title: "测试会话2",
  messages: [
    {
      id: "msg-1",
      role: "user",
      content: "你好",
      timestamp: Date.now(),
    },
    {
      id: "msg-2",
      role: "assistant",
      content: "你好！有什么我可以帮助你的吗？",
      timestamp: Date.now(),
    },
  ],
  createdAt: Date.now(),
  updatedAt: Date.now(),
};

const mockSessionWithToolCall: ChatSession = {
  id: "test-session-3",
  agentId: "test-agent",
  title: "工具调用测试",
  messages: [
    {
      id: "msg-3",
      role: "user",
      content: "读取文件",
      timestamp: Date.now(),
    },
    {
      id: "msg-4",
      role: "assistant",
      content: "正在读取文件...",
      timestamp: Date.now(),
      toolCalls: [
        {
          id: "tc-1",
          name: "fs_read",
          arguments: { path: "/tmp/test.txt" },
          result: "文件内容",
          status: "success",
        },
      ],
    },
  ],
  createdAt: Date.now(),
  updatedAt: Date.now(),
};

describe("ChatPanel", () => {
  it("renders empty state when no messages", () => {
    render(
      <ChatPanel
        session={mockSession}
        onSendMessage={vi.fn()}
        onCancel={vi.fn()}
        isLoading={false}
      />
    );

    expect(screen.getByText("AI 助手")).toBeInTheDocument();
    expect(
      screen.getByText(/我可以帮你编写代码/i)
    ).toBeInTheDocument();
  });

  it("renders messages correctly", () => {
    render(
      <ChatPanel
        session={mockSessionWithMessages}
        onSendMessage={vi.fn()}
        onCancel={vi.fn()}
        isLoading={false}
      />
    );

    expect(screen.getByText("你好")).toBeInTheDocument();
    expect(
      screen.getByText("你好！有什么我可以帮助你的吗？")
    ).toBeInTheDocument();
  });

  it("sends message when clicking send button", () => {
    const onSendMessage = vi.fn();
    render(
      <ChatPanel
        session={mockSession}
        onSendMessage={onSendMessage}
        onCancel={vi.fn()}
        isLoading={false}
      />
    );

    // 先新建会话才能看到输入框
    const newSessionBtn = screen.getByText("新建会话");
    fireEvent.click(newSessionBtn);

    const textarea = screen.getByPlaceholderText(/输入消息/i);
    fireEvent.change(textarea, { target: { value: "测试消息" } });

    const sendBtn = screen.getByTitle("发送 (Ctrl+Enter)");
    fireEvent.click(sendBtn);

    expect(onSendMessage).toHaveBeenCalledWith("测试消息");
  });

  it("sends message on Ctrl+Enter", () => {
    const onSendMessage = vi.fn();
    render(
      <ChatPanel
        session={mockSession}
        onSendMessage={onSendMessage}
        onCancel={vi.fn()}
        isLoading={false}
      />
    );

    // 先新建会话
    const newSessionBtn = screen.getByText("新建会话");
    fireEvent.click(newSessionBtn);

    const textarea = screen.getByPlaceholderText(/输入消息/i);
    fireEvent.change(textarea, { target: { value: "快捷键测试" } });
    fireEvent.keyDown(textarea, { key: "Enter", ctrlKey: true });

    expect(onSendMessage).toHaveBeenCalledWith("快捷键测试");
  });

  it("shows loading indicator when isLoading is true", () => {
    const sessionWithUserMsg: ChatSession = {
      ...mockSession,
      messages: [
        {
          id: "msg-loading",
          role: "user",
          content: "测试中",
          timestamp: Date.now(),
        },
      ],
    };

    render(
      <ChatPanel
        session={sessionWithUserMsg}
        onSendMessage={vi.fn()}
        onCancel={vi.fn()}
        isLoading={true}
      />
    );

    expect(screen.getByText("思考中...")).toBeInTheDocument();
  });

  it("shows cancel button when loading", () => {
    const sessionWithUserMsg: ChatSession = {
      ...mockSession,
      messages: [
        {
          id: "msg-loading",
          role: "user",
          content: "测试中",
          timestamp: Date.now(),
        },
      ],
    };

    render(
      <ChatPanel
        session={sessionWithUserMsg}
        onSendMessage={vi.fn()}
        onCancel={vi.fn()}
        isLoading={true}
      />
    );

    expect(screen.getByTitle("停止生成")).toBeInTheDocument();
  });

  it("renders tool calls with expand/collapse", () => {
    render(
      <ChatPanel
        session={mockSessionWithToolCall}
        onSendMessage={vi.fn()}
        onCancel={vi.fn()}
        isLoading={false}
      />
    );

    // 工具调用卡片应该存在
    expect(screen.getByText("fs_read")).toBeInTheDocument();
    expect(screen.getByText("成功")).toBeInTheDocument();

    // 默认折叠，参数不可见
    expect(screen.queryByText("参数")).not.toBeInTheDocument();

    // 点击展开
    const toolCard = screen.getByText("fs_read").closest("button");
    if (toolCard) {
      fireEvent.click(toolCard);
      expect(screen.getByText("参数")).toBeInTheDocument();
      expect(screen.getByText(/path/i)).toBeInTheDocument();
    }
  });

  it("has copy and regenerate buttons for assistant messages", () => {
    render(
      <ChatPanel
        session={mockSessionWithMessages}
        onSendMessage={vi.fn()}
        onCancel={vi.fn()}
        isLoading={false}
      />
    );

    // 复制按钮
    expect(screen.getByTitle("复制")).toBeInTheDocument();
    // 重新生成按钮
    expect(screen.getByTitle("重新生成")).toBeInTheDocument();
  });

  it("disables send button when input is empty", () => {
    render(
      <ChatPanel
        session={mockSession}
        onSendMessage={vi.fn()}
        onCancel={vi.fn()}
        isLoading={false}
      />
    );

    // 先新建会话
    const newSessionBtn = screen.getByText("新建会话");
    fireEvent.click(newSessionBtn);

    const sendBtn = screen.getByTitle("发送 (Ctrl+Enter)");
    expect(sendBtn).toBeDisabled();
  });

  it("renders markdown content in assistant messages", () => {
    const sessionWithMarkdown: ChatSession = {
      ...mockSession,
      messages: [
        {
          id: "msg-md",
          role: "assistant",
          content: "# 标题\n\n这是**粗体**和_斜体_文本\n\n- 列表项1\n- 列表项2",
          timestamp: Date.now(),
        },
      ],
    };

    render(
      <ChatPanel
        session={sessionWithMarkdown}
        onSendMessage={vi.fn()}
        onCancel={vi.fn()}
        isLoading={false}
      />
    );

    expect(screen.getByText("标题")).toBeInTheDocument();
    expect(screen.getByText("列表项1")).toBeInTheDocument();
    expect(screen.getByText("列表项2")).toBeInTheDocument();
  });
});
