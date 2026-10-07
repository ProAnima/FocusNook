import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useMicrophoneTest } from "./useMicrophoneTest";

const { openLevelMeter } = vi.hoisted(() => ({ openLevelMeter: vi.fn() }));
vi.mock("./microphoneLevel", () => ({ openLevelMeter }));

function deferredMeter() {
  const release = vi.fn();
  let resolve!: (release: () => void) => void;
  const promise = new Promise<() => void>((r) => (resolve = r));
  return { release, promise, settle: () => resolve(release) };
}

beforeEach(() => vi.clearAllMocks());

describe("useMicrophoneTest", () => {
  it("starts and stops the level meter", async () => {
    const release = vi.fn();
    openLevelMeter.mockResolvedValue(release);
    const { result } = renderHook(() => useMicrophoneTest("mic-1"));

    await act(async () => result.current.toggleMicrophoneTest());
    expect(openLevelMeter).toHaveBeenCalledWith("mic-1", expect.any(Function));
    expect(result.current.testing).toBe(true);

    await act(async () => result.current.toggleMicrophoneTest());
    expect(release).toHaveBeenCalledOnce();
    expect(result.current.testing).toBe(false);
  });

  it("flags a failed test when the microphone cannot be opened", async () => {
    openLevelMeter.mockRejectedValue(new Error("denied"));
    const { result } = renderHook(() => useMicrophoneTest(null));

    await act(async () => result.current.toggleMicrophoneTest());

    expect(result.current.testFailed).toBe(true);
    expect(result.current.testing).toBe(false);
  });

  it("releases the meter when the hook unmounts before the microphone opens", async () => {
    const pending = deferredMeter();
    openLevelMeter.mockReturnValue(pending.promise);
    const { result, unmount } = renderHook(() => useMicrophoneTest(null));

    let started!: Promise<void>;
    act(() => {
      started = result.current.toggleMicrophoneTest();
    });
    unmount();
    await act(async () => {
      pending.settle();
      await started;
    });

    expect(pending.release).toHaveBeenCalledOnce();
  });

  it("releases the first meter when a second start supersedes it", async () => {
    const first = deferredMeter();
    const second = deferredMeter();
    openLevelMeter.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const { result } = renderHook(() => useMicrophoneTest(null));

    let a!: Promise<void>;
    let b!: Promise<void>;
    act(() => {
      a = result.current.toggleMicrophoneTest();
    });
    act(() => {
      b = result.current.toggleMicrophoneTest();
    });
    await act(async () => {
      first.settle();
      second.settle();
      await Promise.all([a, b]);
    });

    expect(first.release).toHaveBeenCalledOnce();
    expect(second.release).not.toHaveBeenCalled();
    expect(result.current.testing).toBe(true);
  });
});
