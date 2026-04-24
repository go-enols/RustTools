import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import Device from "../../pages/Device";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("Device", () => {
  it("fetches device info on mount", async () => {
    vi.mocked(invoke).mockResolvedValue({
      cpu: { model: "Intel i7", cores: 8, threads: 16 },
      memory: { total_mb: 16384, used_mb: 8192 },
      gpus: [{ name: "RTX 4090", memory_mb: 24564, cuda_available: true }],
      os: "linux",
      arch: "x86_64",
    });

    render(<Device />);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_device_info");
    });
  });

  it("renders cpu info", async () => {
    vi.mocked(invoke).mockResolvedValue({
      cpu: { model: "Intel i7", cores: 8, threads: 16 },
      memory: { total_mb: 16384, used_mb: 8192 },
      gpus: [],
      os: "linux",
      arch: "x86_64",
    });

    render(<Device />);

    await waitFor(() => {
      expect(screen.getByText("处理器")).toBeInTheDocument();
      expect(screen.getByText("Intel i7")).toBeInTheDocument();
    });
  });

  it("renders gpu list when available", async () => {
    vi.mocked(invoke).mockResolvedValue({
      cpu: { model: "Intel i7", cores: 8, threads: 16 },
      memory: { total_mb: 16384, used_mb: 8192 },
      gpus: [{ name: "RTX 4090", memory_mb: 24564, cuda_available: true }],
      os: "linux",
      arch: "x86_64",
    });

    render(<Device />);

    await waitFor(() => {
      expect(screen.getByText("GPU 0")).toBeInTheDocument();
      expect(screen.getByText("RTX 4090")).toBeInTheDocument();
    });
  });
});
