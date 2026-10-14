import { act, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { usePlanItemLayout } from "./usePlanItemLayout";

function Row({ title = "Task", status = "open" }) {
  const { rowRef, measureRef, stacked } = usePlanItemLayout(title, status);
  return <ul><li ref={rowRef} data-testid="row" data-stacked={stacked} style={{ columnGap: "10px", padding: "0 5px" }}>
    <button className="plan-checkbox" />
    <span ref={measureRef} className="plan-title-measure" />
    <div className="plan-item-actions" />
    {status === "partial" && <button className="plan-progress" />}
  </li></ul>;
}

afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); });
function measurements() {
  let width = 300;
  const callbacks: (() => void)[] = [];
  const disconnect = vi.fn();
  vi.stubGlobal("ResizeObserver", class {
    constructor(callback: () => void) { callbacks.push(callback); }
    observe() {}
    disconnect = disconnect;
  });
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockImplementation(() => width);
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    const measured = this.className === "plan-title-measure" ? 140 : this.className === "plan-item-actions" ? 120 : 20;
    return { width: measured } as DOMRect;
  });
  return { resize: (nextWidth: number) => { width = nextWidth; callbacks.forEach((callback) => callback()); }, disconnect };
}
describe("usePlanItemLayout", () => {
  it("puts actions above the title when they cannot fit and beside it after widening", () => {
    const size = measurements();
    render(<Row />);
    expect(screen.getByTestId("row")).toHaveAttribute("data-stacked", "true");
    act(() => size.resize(400));
    expect(screen.getByTestId("row")).toHaveAttribute("data-stacked", "false");
    act(() => size.resize(280));
    expect(screen.getByTestId("row")).toHaveAttribute("data-stacked", "true");
  });
  it("accounts for progress controls and releases the observer", () => {
    const size = measurements();
    const { rerender, unmount } = render(<Row />);
    act(() => size.resize(315));
    expect(screen.getByTestId("row")).toHaveAttribute("data-stacked", "false");
    rerender(<Row status="partial" />);
    expect(screen.getByTestId("row")).toHaveAttribute("data-stacked", "true");
    unmount();
    expect(size.disconnect).toHaveBeenCalled();
  });
});
