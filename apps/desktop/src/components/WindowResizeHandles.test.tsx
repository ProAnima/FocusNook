import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { WindowResizeHandles } from "./WindowResizeHandles";

const { resizeBy, startResize } = vi.hoisted(() => ({
  resizeBy: vi.fn().mockResolvedValue(undefined),
  startResize: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("../shared/commands", () => ({
  commands: { overlay: { resizeBy, startResize } },
}));

vi.mock("../shared/useLocale", () => ({
  useLocale: () => ({ t: (key: string) => key }),
}));

describe("WindowResizeHandles", () => {
  beforeEach(() => vi.clearAllMocks());

  it("starts native southeast resize from the visible grip", () => {
    render(<WindowResizeHandles />);
    fireEvent.pointerDown(screen.getByRole("button", { name: "header.resize" }));

    expect(startResize).toHaveBeenCalledWith("SouthEast");
  });

  it("supports keyboard resizing in bounded increments", () => {
    render(<WindowResizeHandles />);
    fireEvent.keyDown(screen.getByRole("button", { name: "header.resize" }), {
      key: "ArrowRight",
    });

    expect(resizeBy).toHaveBeenCalledWith(16, 0);
  });
});
