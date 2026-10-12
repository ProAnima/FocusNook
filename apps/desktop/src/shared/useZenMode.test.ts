import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useZenMode } from "./useZenMode";

const { setZenMode, getZenMode } = vi.hoisted(() => ({ setZenMode: vi.fn(), getZenMode: vi.fn() }));
vi.mock("./commands", () => ({ commands: { overlay: { setZenMode, getZenMode } } }));
beforeEach(() => { vi.clearAllMocks(); setZenMode.mockResolvedValue(undefined); getZenMode.mockResolvedValue(false); });
describe("useZenMode", () => {
  it("restores native mode after a renderer reload", async () => {
    getZenMode.mockResolvedValue(true);
    const { result } = renderHook(useZenMode);
    await waitFor(() => expect(result.current.active).toBe(true));
    expect(setZenMode).not.toHaveBeenCalled();
  });
  it("ignores a stale initial response after the user has toggled", async () => {
    let finish: (enabled: boolean) => void = () => {};
    getZenMode.mockImplementation(() => new Promise<boolean>((resolve) => { finish = resolve; }));
    const { result } = renderHook(useZenMode);
    await act(() => result.current.toggle());
    await act(async () => { finish(false); });
    expect(result.current.active).toBe(true);
  });
  it("waits for native geometry and restores the mode on exit", async () => {
    const { result } = renderHook(useZenMode);
    await act(() => result.current.toggle());
    expect(setZenMode).toHaveBeenLastCalledWith(true);
    expect(result.current.active).toBe(true);
    await act(() => result.current.toggle());
    expect(setZenMode).toHaveBeenLastCalledWith(false);
    expect(result.current.active).toBe(false);
  });
  it("ignores repeated clicks while a window transition is pending", async () => {
    let finish: () => void = () => {};
    setZenMode.mockImplementation(() => new Promise<void>((resolve) => { finish = resolve; }));
    const { result } = renderHook(useZenMode);
    await act(async () => {
      const first = result.current.toggle();
      await result.current.toggle();
      expect(setZenMode).toHaveBeenCalledOnce();
      finish();
      await first;
    });
    expect(result.current.active).toBe(true);
  });
  it("supports browser preview when native geometry is unavailable", async () => {
    setZenMode.mockRejectedValue(new Error("No native window"));
    const { result } = renderHook(useZenMode);
    await act(() => result.current.toggle());
    expect(result.current.active).toBe(true);
  });
  it("preserves the mode on native failure", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: {}, configurable: true });
    try {
      setZenMode.mockRejectedValue(new Error("Resize failed"));
      const { result } = renderHook(useZenMode);
      await act(() => result.current.toggle());
      expect(result.current.active).toBe(false);
    } finally { Reflect.deleteProperty(window, "__TAURI_INTERNALS__"); }
  });
});
