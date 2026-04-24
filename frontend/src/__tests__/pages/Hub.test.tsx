import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import Hub from "../../pages/Hub";
import { invoke } from "@tauri-apps/api/core";
import { AppErrorContext } from "../../contexts/AppErrorContext";
import { ProjectProvider } from "../../contexts/ProjectContext";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

function renderWithProviders(ui: React.ReactNode) {
  return render(
    <MemoryRouter>
      <AppErrorContext.Provider value={{ addError: vi.fn(), dismissError: vi.fn() }}>
        <ProjectProvider>
          {ui}
        </ProjectProvider>
      </AppErrorContext.Provider>
    </MemoryRouter>
  );
}

describe("Hub", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockClear();
    localStorage.clear();
  });

  it("fetches env status on mount", async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "get_env_status") {
        return Promise.resolve({
          python_available: true,
          python_version: "3.10.0",
          torch_available: true,
          torch_version: "2.0.0",
          cuda_available: true,
        });
      }
      if (cmd === "get_current_project") return Promise.resolve(null);
      if (cmd === "scan_project") return Promise.resolve({ train_images: 0, val_images: 0, total_annotations: 0 });
      return Promise.resolve({});
    });

    renderWithProviders(<Hub />);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_env_status");
    });
  });

  it("renders env status rows", async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "get_env_status") {
        return Promise.resolve({
          python_available: true,
          python_version: "3.10.0",
          torch_available: false,
          cuda_available: false,
        });
      }
      if (cmd === "get_current_project") return Promise.resolve(null);
      if (cmd === "scan_project") return Promise.resolve({ train_images: 0, val_images: 0, total_annotations: 0 });
      return Promise.resolve({});
    });

    renderWithProviders(<Hub />);

    await waitFor(() => {
      expect(screen.getByText("Python")).toBeInTheDocument();
      expect(screen.getByText("PyTorch")).toBeInTheDocument();
      expect(screen.getByText("CUDA")).toBeInTheDocument();
    });
  });

  it("renders 4 yolo module cards", async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "get_env_status") {
        return Promise.resolve({
          python_available: true,
          torch_available: true,
          cuda_available: true,
        });
      }
      if (cmd === "get_current_project") return Promise.resolve(null);
      if (cmd === "scan_project") return Promise.resolve({ train_images: 0, val_images: 0, total_annotations: 0 });
      return Promise.resolve({});
    });

    renderWithProviders(<Hub />);

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: "项目管理" })).toBeInTheDocument();
      expect(screen.getByRole("heading", { name: "图像标注" })).toBeInTheDocument();
      expect(screen.getByRole("heading", { name: "模型训练" })).toBeInTheDocument();
      expect(screen.getByRole("heading", { name: "视频推理" })).toBeInTheDocument();
    });
  });
});
