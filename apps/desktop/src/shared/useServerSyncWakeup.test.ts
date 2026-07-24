import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useServerSyncWakeup } from "./useServerSyncWakeup";

const { request } = vi.hoisted(() => ({
  request: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("./commands", () => ({
  commands: { serverSync: { request } },
}));

describe("useServerSyncWakeup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "visible",
    });
  });

  it("requests sync on mount and periodically while visible", () => {
    const { unmount } = renderHook(() => useServerSyncWakeup());
    expect(request).toHaveBeenCalledTimes(1);

    act(() => vi.advanceTimersByTime(20_000));
    expect(request).toHaveBeenCalledTimes(2);
    unmount();
    vi.useRealTimers();
  });

  it("requests sync when the app returns to the foreground", () => {
    const { unmount } = renderHook(() => useServerSyncWakeup());
    request.mockClear();

    act(() => window.dispatchEvent(new Event("focus")));
    expect(request).toHaveBeenCalledTimes(1);
    unmount();
    vi.useRealTimers();
  });
});
