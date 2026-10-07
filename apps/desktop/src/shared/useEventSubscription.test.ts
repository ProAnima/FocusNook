import { describe, expect, it, vi } from "vitest";
import { renderHook } from "@testing-library/react";
import { useEventSubscription } from "./useEventSubscription";

describe("useEventSubscription", () => {
  it("subscribes once and unsubscribes on unmount", async () => {
    const unlisten = vi.fn();
    const subscribe = vi.fn().mockResolvedValue(unlisten);
    const handler = vi.fn();

    const { unmount } = renderHook(() => useEventSubscription(subscribe, handler));
    await Promise.resolve();
    expect(subscribe).toHaveBeenCalledTimes(1);
    expect(subscribe).toHaveBeenCalledWith(expect.any(Function));

    unmount();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("calls the latest handler without re-subscribing", async () => {
    const subscribe = vi.fn().mockResolvedValue(vi.fn());
    const first = vi.fn();
    const second = vi.fn();

    const { rerender } = renderHook(({ handler }) => useEventSubscription(subscribe, handler), {
      initialProps: { handler: first },
    });
    await Promise.resolve();
    rerender({ handler: second });

    const emit = subscribe.mock.calls[0][0] as () => void;
    emit();
    expect(subscribe).toHaveBeenCalledTimes(1);
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledTimes(1);
  });

  it("removes the listener even when unmounted before the subscription resolves", async () => {
    const unlisten = vi.fn();
    let resolve!: (cleanup: () => void) => void;
    const subscribe = vi.fn().mockReturnValue(new Promise<() => void>((r) => (resolve = r)));

    const { unmount } = renderHook(() => useEventSubscription(subscribe, vi.fn()));
    unmount();
    expect(unlisten).not.toHaveBeenCalled();

    resolve(unlisten);
    await Promise.resolve();
    await Promise.resolve();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("ignores a failing subscription", async () => {
    const subscribe = vi.fn().mockRejectedValue(new Error("no tauri"));
    const { unmount } = renderHook(() => useEventSubscription(subscribe, vi.fn()));
    await Promise.resolve();
    expect(() => unmount()).not.toThrow();
  });
});
