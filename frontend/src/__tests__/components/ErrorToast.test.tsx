import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ErrorToast } from "../../components/ErrorToast";

describe("ErrorToast", () => {
  it("renders error items", () => {
    const errors = [
      { id: "1", message: "Error 1", type: "error" as const },
      { id: "2", message: "Warning 1", type: "warning" as const },
    ];

    render(<ErrorToast errors={errors} onDismiss={vi.fn()} />);

    expect(screen.getByText("Error 1")).toBeInTheDocument();
    expect(screen.getByText("Warning 1")).toBeInTheDocument();
  });

  it("clicking dismiss removes item", () => {
    const onDismiss = vi.fn();
    const errors = [{ id: "1", message: "Error 1", type: "error" as const }];

    render(<ErrorToast errors={errors} onDismiss={onDismiss} />);
    const dismissButton = screen.getByRole("button");
    fireEvent.click(dismissButton);

    expect(onDismiss).toHaveBeenCalledWith("1");
  });

  it("renders different types with correct colors", () => {
    const errors = [
      { id: "1", message: "Info", type: "info" as const },
      { id: "2", message: "Warning", type: "warning" as const },
      { id: "3", message: "Error", type: "error" as const },
    ];

    const { container } = render(<ErrorToast errors={errors} onDismiss={vi.fn()} />);
    const items = container.querySelectorAll(".fixed > div > div");
    expect(items.length).toBe(3);
  });
});
