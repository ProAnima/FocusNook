import { act, render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useZenWindowHeight, zenContentHeight } from "./useZenWindowHeight";

const { resizeZenWindow } = vi.hoisted(() => ({ resizeZenWindow: vi.fn().mockResolvedValue(undefined) }));
vi.mock("./commands", () => ({ commands: { overlay: { resizeZenWindow } } }));

function Content() {
  const ref = useZenWindowHeight("task");
  return <div className="overlay-shell"><div className="zen-drag-zone" /><main style={{ padding: "0 0 7px" }}>
    <section ref={ref} style={{ rowGap: "4px" }}><div className="plan-list" /><button className="zen-next">Next</button></section>
  </main></div>;
}

describe("zenContentHeight", () => {
  it("resizes when intrinsic card height changes and disconnects on exit", () => {
    let cardHeight = 30;
    let shellHeight = 96;
    let notify = () => {};
    const disconnect = vi.fn();
    vi.stubGlobal("ResizeObserver", class {
      constructor(callback: () => void) { notify = callback; }
      observe() {}
      disconnect = disconnect;
    });
    vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) {
      return this.className === "overlay-shell" ? shellHeight : this.className === "plan-list" ? cardHeight : this.className === "zen-drag-zone" ? 22 : 24;
    });
    try {
      const { unmount } = render(<Content />);
      expect(resizeZenWindow).toHaveBeenLastCalledWith(89);
      act(() => { shellHeight = 89; notify(); });
      expect(resizeZenWindow).toHaveBeenCalledOnce();
      act(() => { shellHeight = 180; notify(); });
      expect(resizeZenWindow).toHaveBeenCalledTimes(2);
      act(() => { cardHeight = 80; notify(); });
      expect(resizeZenWindow).toHaveBeenLastCalledWith(139);
      unmount();
      expect(disconnect).toHaveBeenCalledOnce();
    } finally { vi.restoreAllMocks(); vi.unstubAllGlobals(); }
  });
  it("fits a short task and Next without trailing space", () => {
    expect(zenContentHeight(24, 24, 22, 4, 7)).toBe(83);
  });
  it("grows for stacked actions and multiline text up to the compact limit", () => {
    expect(zenContentHeight(80, 24, 22, 4, 7)).toBe(139);
    expect(zenContentHeight(400, 24, 22, 4, 7)).toBe(180);
  });
  it("keeps the empty state accessible at the minimum height", () => {
    expect(zenContentHeight(20, 0, 22, 0, 7)).toBe(72);
  });
});
