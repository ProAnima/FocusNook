import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { PlanItemRenameDialog } from "./PlanItemRenameDialog";
import type { PlanItem } from "../shared/commands";

const item: PlanItem = { id: "test", title: "Old", status: "open", planDate: "2026-10-12", progressPercent: null, isLongRunning: false };
describe("PlanItemRenameDialog", () => {
  it("prefills, trims and saves a title", async () => {
    const user = userEvent.setup();
    const save = vi.fn().mockResolvedValue({ ...item, title: "New" });
    const close = vi.fn();
    render(<PlanItemRenameDialog item={item} onSave={save} onClose={close} />);
    const input = screen.getByRole("textbox", { name: "Переименовать" });
    expect(input).toHaveValue("Old");
    await user.clear(input);
    expect(screen.getByRole("button", { name: "Сохранить" })).toBeDisabled();
    await user.type(input, "  New  ");
    await user.click(screen.getByRole("button", { name: "Сохранить" }));
    expect(save).toHaveBeenCalledWith("test", "New");
    expect(close).toHaveBeenCalledOnce();
  });
  it("keeps the draft when saving fails and allows retry", async () => {
    const user = userEvent.setup();
    const save = vi.fn().mockResolvedValue(null);
    const close = vi.fn();
    render(<PlanItemRenameDialog item={item} onSave={save} onClose={close} />);
    await user.click(screen.getByRole("button", { name: "Сохранить" }));
    expect(await screen.findByRole("alert")).toBeInTheDocument();
    expect(close).not.toHaveBeenCalled();
    expect(screen.getByRole("textbox")).toHaveValue("Old");
  });
});
