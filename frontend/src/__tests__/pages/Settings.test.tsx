import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import Settings from "../../pages/Settings";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("Settings", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockClear();
  });

  it("fetches env report on mount", async () => {
    vi.mocked(invoke).mockResolvedValue({
      system: { os: "Linux", arch: "x86_64", cpu_cores: 8, total_memory_mb: 16384 },
      uv_installed: true,
      uv_version: "0.4.0",
      python_installed: true,
      python_version: "3.10.0",
      torch_available: true,
      torch_cuda: true,
      ort_available: true,
      ort_cuda: false,
      cuda: { available: true, driver_version: "535", runtime_version: "12.2", gpus: [] },
    });

    render(<Settings />);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("generate_env_report");
    });
  });

  it("renders system info section", async () => {
    vi.mocked(invoke).mockResolvedValue({
      system: { os: "Linux", arch: "x86_64", cpu_cores: 8, total_memory_mb: 16384 },
      uv_installed: true,
      uv_version: "0.4.0",
      python_installed: false,
      torch_available: false,
      torch_cuda: false,
      ort_available: false,
      ort_cuda: false,
      cuda: { available: false, gpus: [] },
    });

    render(<Settings />);

    await waitFor(() => {
      expect(screen.getByText(/操作系统/i)).toBeInTheDocument();
      expect(screen.getByText(/CPU 核心/i)).toBeInTheDocument();
    });
  });

  it("renders install button", async () => {
    vi.mocked(invoke).mockResolvedValue({
      system: { os: "Linux", arch: "x86_64", cpu_cores: 8, total_memory_mb: 16384 },
      uv_installed: true,
      python_installed: false,
      torch_available: false,
      torch_cuda: false,
      ort_available: false,
      ort_cuda: false,
      cuda: { available: false, gpus: [] },
    });

    render(<Settings />);

    await waitFor(() => {
      expect(screen.getByText(/一键安装环境/i)).toBeInTheDocument();
    });
  });
});
