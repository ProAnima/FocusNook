import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RemindersView } from "./RemindersView";

const { list, create, deleteReminder } = vi.hoisted(() => ({
  list: vi.fn(),
  create: vi.fn(),
  deleteReminder: vi.fn(),
}));

vi.mock("../shared/commands", () => ({
  commands: { reminders: { list, create, delete: deleteReminder } },
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

  it("creates a reminder with a custom date and time", async () => {
    list.mockResolvedValue([]);
    create.mockResolvedValue({
      id: "3",
      title: "Встреча с командой",
      triggerAtUtc: "2030-01-01T10:00:00.000Z",
      status: "scheduled",
    });
    const user = userEvent.setup();
    render(<RemindersView />);
    await screen.findByText("Нет активных напоминаний");

    await user.type(screen.getByPlaceholderText("Напомнить о..."), "Встреча с командой");
    await user.click(screen.getByText("Своё время"));

    const dateInput = screen.getByLabelText("Дата и время напоминания");
    fireEvent.change(dateInput, { target: { value: "2030-01-01T10:00" } });
    await user.click(screen.getByText("Добавить"));

    expect(create).toHaveBeenCalledWith("Встреча с командой", expect.any(String));
    expect(await screen.findByText("Встреча с командой")).toBeInTheDocument();
  });

  it("does not submit the custom time form for a past date", async () => {
    list.mockResolvedValue([]);
    const user = userEvent.setup();
    render(<RemindersView />);
    await screen.findByText("Нет активных напоминаний");

    await user.type(screen.getByPlaceholderText("Напомнить о..."), "Просроченное");
    await user.click(screen.getByText("Своё время"));

    const dateInput = screen.getByLabelText("Дата и время напоминания");
    fireEvent.change(dateInput, { target: { value: "2000-01-01T10:00" } });
    await user.click(screen.getByText("Добавить"));

    expect(create).not.toHaveBeenCalled();
  });

  it("removes a reminder from the list when deleted", async () => {
    list.mockResolvedValue([
      {
        id: "1",
        title: "Напоминание",
        triggerAtUtc: "2030-01-01T10:00:00.000Z",
        status: "scheduled",
      },
    ]);
    deleteReminder.mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(<RemindersView />);

    await screen.findByText("Напоминание");
    await user.click(screen.getByTitle("Удалить"));

    expect(deleteReminder).toHaveBeenCalledWith("1");
    expect(screen.queryByText("Напоминание")).not.toBeInTheDocument();
  });
});
