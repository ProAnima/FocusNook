import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import type { KeyboardEvent, PointerEvent } from "react";
import { useHoldToConfirm } from "./useHoldToConfirm";

function pointerDown(button = 0) {
  return { button, pointerId: 1, currentTarget: { setPointerCapture: vi.fn() } } as unknown as PointerEvent;
}

function key(name: string) {
  return { key: name, preventDefault: vi.fn() } as unknown as KeyboardEvent;
}

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe("useHoldToConfirm", () => {
  it("confirms only after the pointer is held long enough", () => {
    const onConfirm = vi.fn();
    const { result } = renderHook(() => useHoldToConfirm(onConfirm));

    act(() => result.current.buttonProps.onPointerDown(pointerDown()));
    expect(result.current.holding).toBe(true);
    act(() => vi.advanceTimersByTime(899));
    expect(onConfirm).not.toHaveBeenCalled();
    act(() => vi.advanceTimersByTime(1));

    expect(onConfirm).toHaveBeenCalledOnce();
    expect(result.current.holding).toBe(false);
  });

  it("cancels when the pointer is released early", () => {
    const onConfirm = vi.fn();
    const { result } = renderHook(() => useHoldToConfirm(onConfirm));

    act(() => result.current.buttonProps.onPointerDown(pointerDown()));
    act(() => result.current.buttonProps.onPointerUp());
    act(() => vi.advanceTimersByTime(2000));

    expect(onConfirm).not.toHaveBeenCalled();
    expect(result.current.holding).toBe(false);
  });

  it("ignores non-primary buttons", () => {
    const onConfirm = vi.fn();
    const { result } = renderHook(() => useHoldToConfirm(onConfirm));

    act(() => result.current.buttonProps.onPointerDown(pointerDown(2)));
    act(() => vi.advanceTimersByTime(2000));

    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("confirms immediately from the keyboard", () => {
    const onConfirm = vi.fn();
    const { result } = renderHook(() => useHoldToConfirm(onConfirm));

    act(() => result.current.buttonProps.onKeyDown(key("Enter")));
    act(() => result.current.buttonProps.onKeyDown(key("a")));

    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it("does not fire after unmount", () => {
    const onConfirm = vi.fn();
    const { result, unmount } = renderHook(() => useHoldToConfirm(onConfirm));

    act(() => result.current.buttonProps.onPointerDown(pointerDown()));
    unmount();
    act(() => vi.advanceTimersByTime(2000));

    expect(onConfirm).not.toHaveBeenCalled();
  });
});
