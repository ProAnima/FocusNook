import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { AccountWindowChrome } from "./AccountWindowChrome";

const { close } = vi.hoisted(() => ({ close: vi.fn().mockResolvedValue(undefined) }));

vi.mock("../shared/commands", () => ({
  commands: {
    overlay: {
      close,
      resizeBy: vi.fn().mockResolvedValue(undefined),
      startResize: vi.fn().mockResolvedValue(undefined),
    },
  },
}));

describe("AccountWindowChrome", () => {
  it("keeps native move, resize, and close controls available", async () => {
    const user = userEvent.setup();
    const { container } = render(<AccountWindowChrome><span>Account form</span></AccountWindowChrome>);

    expect(container.querySelector("header[data-tauri-drag-region]")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Изменить размер окна" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Закрыть" }));
    expect(close).toHaveBeenCalledOnce();
  });
});
