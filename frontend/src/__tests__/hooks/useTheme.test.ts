import { describe, it, expect } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useTheme } from "../../hooks/useTheme";

describe("useTheme", () => {
  it("initializes from document class", () => {
    document.documentElement.classList.remove("dark");
    const { result } = renderHook(() => useTheme());
    expect(result.current.dark).toBe(false);
  });

  it("toggle switches dark mode", () => {
    document.documentElement.classList.remove("dark");
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.toggle();
    });

    expect(result.current.dark).toBe(true);
    expect(document.documentElement.classList.contains("dark")).toBe(true);

    act(() => {
      result.current.toggle();
    });

    expect(result.current.dark).toBe(false);
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });
});
