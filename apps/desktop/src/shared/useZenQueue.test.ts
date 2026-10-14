import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { PlanItem } from "./commands";
import { useZenQueue } from "./useZenQueue";

const task = (id: string, status: PlanItem["status"] = "open"): PlanItem => ({ id, title: id, status, planDate: "2026-10-12", progressPercent: null, isLongRunning: false });

describe("useZenQueue", () => {
  it("cycles pending tasks without changing task status", () => {
    const items = [task("done", "done"), task("a"), task("b", "partial")];
    const { result } = renderHook(() => useZenQueue(items));
    expect(result.current.current?.id).toBe("a");
    act(() => result.current.next());
    expect(result.current.current?.id).toBe("b");
    act(() => result.current.next());
    expect(result.current.current?.id).toBe("a");
    expect(items[1].status).toBe("open");
  });
  it("does not skip successors when the current task is completed or removed", () => {
    const { result, rerender } = renderHook(({ items }) => useZenQueue(items), { initialProps: { items: [task("a"), task("b"), task("c")] } });
    rerender({ items: [task("a", "done"), task("b"), task("c")] });
    expect(result.current.current?.id).toBe("b");
    rerender({ items: [task("a", "done"), task("c")] });
    expect(result.current.current?.id).toBe("c");
  });
  it("handles loading and renamed or synced tasks without losing the current task", () => {
    const { result, rerender } = renderHook(({ items }) => useZenQueue(items), { initialProps: { items: [] as PlanItem[] } });
    rerender({ items: [task("a"), task("b")] });
    act(() => result.current.next());
    rerender({ items: [task("a"), { ...task("b"), title: "Renamed" }, task("c")] });
    expect(result.current.current?.title).toBe("Renamed");
  });
  it("removes completed tasks from the ring and advances to their successor", () => {
    const { result, rerender } = renderHook(({ items }) => useZenQueue(items), { initialProps: { items: [task("a"), task("b"), task("c")] } });
    act(() => result.current.next());
    expect(result.current.current?.id).toBe("b");
    rerender({ items: [task("a"), task("b", "done"), task("c")] });
    expect(result.current.current?.id).toBe("c");
    act(() => result.current.next());
    expect(result.current.current?.id).toBe("a");
    rerender({ items: [task("a", "done"), task("b", "done"), task("c", "done")] });
    expect(result.current.current).toBeUndefined();
  });
  it("keeps a single task in the ring until it is completed", () => {
    const { result } = renderHook(() => useZenQueue([task("a")]));
    act(() => result.current.next());
    expect(result.current.current?.id).toBe("a");
  });
});
