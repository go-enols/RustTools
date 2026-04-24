import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import Welcome from "../../pages/Welcome";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("Welcome", () => {
  it("renders brand title", () => {
    render(
      <MemoryRouter>
        <Welcome />
      </MemoryRouter>
    );
    expect(screen.getByText("RustTools")).toBeInTheDocument();
    expect(screen.getByText("一站式高性能 Rust 工具箱")).toBeInTheDocument();
  });

  it("renders YOLO module card", () => {
    render(
      <MemoryRouter>
        <Welcome />
      </MemoryRouter>
    );
    expect(screen.getByText("YOLO 视觉")).toBeInTheDocument();
    expect(screen.queryByText("桌面捕获")).not.toBeInTheDocument();
    expect(screen.queryByText("环境设置")).not.toBeInTheDocument();
  });

  it("navigates to hub when YOLO card clicked", () => {
    render(
      <MemoryRouter initialEntries={["/"]}>
        <Welcome />
      </MemoryRouter>
    );
    const yoloCard = screen.getByText("YOLO 视觉").closest("button");
    expect(yoloCard).toBeInTheDocument();
  });
});
