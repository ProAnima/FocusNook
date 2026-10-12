import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PlanItem } from "../shared/commands";
import { ZenView } from "./ZenView";

const { plan } = vi.hoisted(() => ({ plan: {
  items: [] as PlanItem[], loaded: true, toggleDone: vi.fn(), cycleProgress: vi.fn(),
  toggleDeferred: vi.fn(), toggleLongRunning: vi.fn(), moveToDate: vi.fn(), deleteItem: vi.fn(), rename: vi.fn(),
} }));
vi.mock("../shared/usePlanItems", () => ({ usePlanItems: () => plan }));
const task = (id: string, status: PlanItem["status"] = "open"): PlanItem => ({ id, title: `Task ${id}`, status, progressPercent: null, isLongRunning: false, planDate: "2026-10-12" });
beforeEach(() => { vi.clearAllMocks(); plan.items = [task("a"), task("b"), task("c", "done")]; });
describe("ZenView", () => {
  it("shows one unfinished task and cycles with Next without completing tasks", async () => {
    const user = userEvent.setup();
    render(<ZenView />);
    expect(screen.getByText("Task a")).toBeInTheDocument();
    expect(screen.queryByText("Task b")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Частично выполнено" }));
    expect(plan.cycleProgress).toHaveBeenCalledWith("a");
    await user.click(screen.getByRole("button", { name: "Следующее" }));
    expect(screen.getByText("Task b")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Следующее" }));
    expect(screen.getByText("Task a")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Следующее" })).toBeInTheDocument();
    expect(plan.toggleDone).not.toHaveBeenCalled();
  });
  it("shows the final screen only after every task is completed", async () => {
    const user = userEvent.setup();
    plan.items = [task("a")];
    const { rerender } = render(<ZenView />);
    await user.click(screen.getByRole("button", { name: "Отметить выполненным" }));
    expect(plan.toggleDone).toHaveBeenCalledWith("a");
    plan.items = [task("a", "done")];
    rerender(<ZenView />);
    expect(screen.getByText("На сегодня всё")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Следующее" })).not.toBeInTheDocument();
  });
  it("renames through the shared plan hook", async () => {
    const user = userEvent.setup();
    plan.rename.mockResolvedValue({ ...task("a"), title: "Updated" });
    render(<ZenView />);
    await user.click(screen.getByRole("button", { name: "Переименовать" }));
    const input = screen.getByRole("textbox", { name: "Переименовать" });
    await user.clear(input);
    await user.type(input, "Updated");
    await user.click(screen.getByRole("button", { name: "Сохранить" }));
    expect(plan.rename).toHaveBeenCalledWith("a", "Updated");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
