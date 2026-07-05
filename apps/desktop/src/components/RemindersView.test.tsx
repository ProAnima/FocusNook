import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RemindersView } from "./RemindersView";

const { list, create } = vi.hoisted(() => ({
  list: vi.fn(),
  create: vi.fn(),
}));

vi.mock("../shared/commands", () => ({
  commands: { reminders: { list, create } },
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe("RemindersView", () => {
  it("shows an empty state when there are no reminders", async () => {
    list.mockResolvedValue([]);
    render(<RemindersView />);

    expect(await screen.findByText("Нет активных напоминаний")).toBeInTheDocument();
  });

  it("loads and shows persisted reminders", async () => {
    list.mockResolvedValue([
      {
        id: "1",
        title: "Проверить рендер",
        triggerAtUtc: "2026-07-04T18:30:00.000Z",
        status: "scheduled",
      },
    ]);
    render(<RemindersView />);

    expect(await screen.findByText("Проверить рендер")).toBeInTheDocument();
  });

  it("disables presets until a title is entered, then creates a reminder", async () => {
    list.mockResolvedValue([]);
    create.mockResolvedValue({
      id: "2",
      title: "Позвонить клиенту",
      triggerAtUtc: "2026-07-04T18:30:00.000Z",
      status: "scheduled",
    });
    const user = userEvent.setup();
    render(<RemindersView />);
    await screen.findByText("Нет активных напоминаний");

    const presetButton = screen.getByText("Через 15 мин");
    expect(presetButton).toBeDisabled();

    await user.type(screen.getByPlaceholderText("Напомнить о..."), "Позвонить клиенту");
    expect(presetButton).not.toBeDisabled();

    await user.click(presetButton);

    expect(create).toHaveBeenCalledWith("Позвонить клиенту", expect.any(String));
    expect(await screen.findByText("Позвонить клиенту")).toBeInTheDocument();
  });
});
