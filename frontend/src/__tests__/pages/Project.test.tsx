import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import Project from "../../pages/Project";
import { invoke } from "@tauri-apps/api/core";
import { ProjectProvider } from "../../contexts/ProjectContext";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("Project", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockClear();
    localStorage.clear();
  });

  it("renders project management page", () => {
    render(
      <ProjectProvider>
        <Project />
      </ProjectProvider>
    );
    expect(screen.getByText("项目管理")).toBeInTheDocument();
    expect(screen.getByText("新建项目")).toBeInTheDocument();
  });

  it("renders create project form", () => {
    render(
      <ProjectProvider>
        <Project />
      </ProjectProvider>
    );
    expect(screen.getByPlaceholderText("输入项目名称")).toBeInTheDocument();
    expect(screen.getByText("创建")).toBeInTheDocument();
  });
});
