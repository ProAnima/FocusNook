import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useAudioSource } from "./useAudioSource";

beforeEach(() => {
  URL.createObjectURL = vi.fn(() => "blob:audio-1");
  URL.revokeObjectURL = vi.fn();
});

afterEach(() => vi.useRealTimers());

describe("useAudioSource", () => {
  it("loads audio into a blob URL and revokes it on unmount", async () => {
    const loadAudio = vi.fn().mockResolvedValue(btoa("abc"));
    const { result, unmount } = renderHook(() => useAudioSource("a1", loadAudio));

    await waitFor(() => expect(result.current.src).toBe("blob:audio-1"));
    expect(loadAudio).toHaveBeenCalledWith("a1");
    expect(result.current.error).toBe(false);

    unmount();
    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:audio-1");
  });

  it("reports an error when loading fails and retries on demand", async () => {
    const loadAudio = vi.fn().mockRejectedValueOnce(new Error("missing")).mockResolvedValueOnce(btoa("abc"));
    const { result } = renderHook(() => useAudioSource("a1", loadAudio));

    await waitFor(() => expect(result.current.error).toBe(true));
    act(() => result.current.retry());

    await waitFor(() => expect(result.current.src).toBe("blob:audio-1"));
    expect(result.current.error).toBe(false);
    expect(loadAudio).toHaveBeenCalledTimes(2);
  });

  it("times out a load that never settles", () => {
    vi.useFakeTimers();
    const loadAudio = vi.fn().mockReturnValue(new Promise<string>(() => {}));
    const { result } = renderHook(() => useAudioSource("a1", loadAudio));

    act(() => vi.advanceTimersByTime(15_000));

    expect(result.current.error).toBe(true);
  });
});
