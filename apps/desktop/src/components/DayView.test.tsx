import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DayView } from "./DayView";

const { list, listRange, create, toggleDone, cycleProgress, toggleDeferred, toggleLongRunning, moveToDate, rollOverPending, deletePlanItem, listReminders, onRemindersChanged } =
  vi.hoisted(() => ({
    list: vi.fn(),
    listRange: vi.fn(),
    create: vi.fn(),
    toggleDone: vi.fn(),
    cycleProgress: vi.fn(),
    toggleDeferred: vi.fn(),
    toggleLongRunning: vi.fn(),
    moveToDate: vi.fn(),
    rollOverPending: vi.fn(),
    deletePlanItem: vi.fn(),
    listReminders: vi.fn(),
    onRemindersChanged: vi.fn().mockResolvedValue(() => {}),
  }));

vi.mock("../shared/commands", () => ({
  commands: {
    planItems: { list, listRange, create, toggleDone, cycleProgress, toggleDeferred, toggleLongRunning, moveToDate, rollOverPending, delete: deletePlanItem },
    reminders: { list: listReminders, onChanged: onRemindersChanged },
    serverSync: { onCompleted: vi.fn().mockResolvedValue(() => {}) },
  },
}));

function item(overrides = {}) {
  return {
    id: "1",
    title: "Задача",
    status: "open",
    progressPercent: null,
    planDate: "2026-07-06",
    isLongRunning: false,
    ...overrides,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  list.mockResolvedValue([]);
  listRange.mockResolvedValue([]);
  rollOverPending.mockResolvedValue(0);
  listReminders.mockResolvedValue([]);
});

describe("DayView", () => {
  it("loads persisted items and shows the done count", async () => {
    list.mockResolvedValue([
      item({ id: "1", title: "Проверить рендер" }),
      item({ id: "2", title: "Готово", status: "done" }),
    ]);
    render(<DayView />);

    expect(await screen.findByText("Проверить рендер")).toBeInTheDocument();
    expect(screen.getByText("1/2")).toBeInTheDocument();
    expect(list).toHaveBeenCalledWith(expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/));
  });

  it("opens the complete task text and closes it with Escape", async () => {
    const title =
      "Очень длинная задача, текст которой не должен теряться из-за обрезки в строке списка";
    list.mockResolvedValue([item({ title })]);
    const user = userEvent.setup();
    render(<DayView />);

    await user.click(await screen.findByRole("button", { name: title }));
    const dialog = screen.getByRole("dialog", { name: title });
    expect(within(dialog).getByText(title)).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog", { name: title })).not.toBeInTheDocument();
  });

  it("shows an empty state when there are no items", async () => {
    render(<DayView />);

    expect(await screen.findByText("На сегодня пока ничего не запланировано")).toBeInTheDocument();
  });

  it("adds a new item through the quick-add form", async () => {
    create.mockResolvedValue(item({ id: "3", title: "Новое дело" }));
    const user = userEvent.setup();
    render(<DayView />);

    await screen.findByText("На сегодня пока ничего не запланировано");
    await user.type(screen.getByPlaceholderText("Добавить дело..."), "Новое дело{Enter}");

    expect(create).toHaveBeenCalledWith("Новое дело", expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/));
    expect(await screen.findByText("Новое дело")).toBeInTheDocument();
  });

  it("marks an item done when its checkbox is clicked", async () => {
    list.mockResolvedValue([item()]);
    toggleDone.mockResolvedValue(item({ status: "done" }));
    const user = userEvent.setup();
    render(<DayView />);

    await user.click(await screen.findByRole("button", { name: "Отметить выполненным" }));

    expect(toggleDone).toHaveBeenCalledWith("1");
    expect(await screen.findByText("1/1")).toBeInTheDocument();
  });

  it("steps progress forward when the partial button is clicked", async () => {
    list.mockResolvedValue([item()]);
    cycleProgress.mockResolvedValue(item({ status: "partial", progressPercent: 25 }));
    const user = userEvent.setup();
    render(<DayView />);

    await user.click(await screen.findByTitle("Частично выполнено"));

    expect(cycleProgress).toHaveBeenCalledWith("1");
    expect(await screen.findByText("25%")).toBeInTheDocument();
  });

  it("marks a 75 percent item done when progress is clicked", async () => {
    list.mockResolvedValue([item({ status: "partial", progressPercent: 75 })]);
    cycleProgress.mockResolvedValue(item({ status: "done", progressPercent: null }));
    const user = userEvent.setup();
    render(<DayView />);

    await user.click(await screen.findByText("75%"));

    expect(cycleProgress).toHaveBeenCalledWith("1");
    expect(await screen.findByText("1/1")).toBeInTheDocument();
  });

  it("defers an item and can bring it back", async () => {
    list.mockResolvedValue([item()]);
    toggleDeferred.mockResolvedValue(item({ status: "deferred" }));
    const user = userEvent.setup();
    render(<DayView />);

    await user.click(await screen.findByTitle("Отложить"));

    expect(toggleDeferred).toHaveBeenCalledWith("1");
    expect(await screen.findByTitle("Вернуть в работу")).toBeInTheDocument();
  });

  it("moves a marked task into the long-running group at the top", async () => {
    list.mockResolvedValue([
      item({ id: "1", title: "Обычная" }),
      item({ id: "2", title: "На несколько дней" }),
    ]);
    toggleLongRunning.mockResolvedValue(
      item({ id: "1", title: "Обычная", isLongRunning: true }),
    );
    const user = userEvent.setup();
    const { container } = render(<DayView />);

    await user.click((await screen.findAllByTitle("Сделать протяжённой"))[0]);

    expect(toggleLongRunning).toHaveBeenCalledWith("1");
    const group = await screen.findByRole("region", { name: "Протяжённые" });
    expect(within(group).getByText("Обычная")).toBeInTheDocument();
    expect(container.querySelector(".plan-groups")?.firstElementChild).toBe(group);
  });

  it("shows a long-running task on every selected day without counting it as a daily task", async () => {
    let initialDate = "";
    list.mockImplementation((date: string) => {
      if (!initialDate) initialDate = date;
      return Promise.resolve(
        date === initialDate
          ? [
              item({ id: "global", title: "Большой проект", planDate: "2026-06-01", isLongRunning: true }),
              item({ id: "daily", title: "Дело дня", planDate: date }),
            ]
          : [item({ id: "global", title: "Большой проект", planDate: "2026-06-01", isLongRunning: true })],
      );
    });
    const user = userEvent.setup();
    render(<DayView />);

    const group = await screen.findByRole("region", { name: "Протяжённые" });
    expect(within(group).getByText("Большой проект")).toBeInTheDocument();
    expect(screen.getByText("0/1")).toBeInTheDocument();
    expect(within(group).queryByTitle("Перенести на следующий день")).not.toBeInTheDocument();

    await user.click(screen.getByTitle("Следующий день"));

    await waitFor(() => expect(list).toHaveBeenCalledTimes(2));
    expect(screen.getByText("Большой проект")).toBeInTheDocument();
    expect(screen.queryByText("Дело дня")).not.toBeInTheDocument();
    expect(screen.getByText("0/0")).toBeInTheDocument();
  });

  it("removes a global task from a day when its long-running marker is cleared", async () => {
    list.mockResolvedValue([
      item({ title: "Большой проект", planDate: "2026-06-01", isLongRunning: true }),
    ]);
    toggleLongRunning.mockResolvedValue(
      item({ title: "Большой проект", planDate: "2026-06-01", isLongRunning: false }),
    );
    const user = userEvent.setup();
    render(<DayView />);

    await user.click(await screen.findByTitle("Убрать из протяжённых"));

    expect(toggleLongRunning).toHaveBeenCalledWith("1");
    expect(screen.queryByText("Большой проект")).not.toBeInTheDocument();
  });

  it("toggles long-running mode from task details for touch layouts", async () => {
    list.mockResolvedValue([item({ title: "Большой проект" })]);
    toggleLongRunning.mockResolvedValue(
      item({ title: "Большой проект", isLongRunning: true }),
    );
    const user = userEvent.setup();
    render(<DayView />);

    await user.click(await screen.findByRole("button", { name: "Большой проект" }));
    const dialog = screen.getByRole("dialog", { name: "Большой проект" });
    await user.click(within(dialog).getByRole("button", { name: /Сделать протяжённой/ }));

    expect(toggleLongRunning).toHaveBeenCalledWith("1");
    expect(await within(dialog).findByRole("button", { name: /Протяжённая задача/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("moves an unfinished item to the next day", async () => {
    list.mockResolvedValue([item()]);
    moveToDate.mockImplementation((_id: string, targetDate: string) =>
      Promise.resolve(item({ planDate: targetDate })),
    );
    const user = userEvent.setup();
    render(<DayView />);

    await screen.findByText("Задача");
    await user.click(screen.getByTitle("Перенести на следующий день"));

    expect(moveToDate).toHaveBeenCalledWith("1", expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/));
    await waitFor(() => expect(screen.queryByText("Задача")).not.toBeInTheDocument());
  });

  it("opens the calendar and selects another day", async () => {
    const user = userEvent.setup();
    const { container } = render(<DayView />);

    await user.click(screen.getByTitle("Открыть календарь"));
    expect(listRange).toHaveBeenCalledWith(expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/), expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/));
    const nextDateButton = container.querySelector<HTMLButtonElement>(".calendar-day:not(.is-selected):not(.is-muted)");
    expect(nextDateButton).toBeTruthy();
    await user.click(nextDateButton as HTMLButtonElement);

    await waitFor(() => expect(list).toHaveBeenCalledTimes(2));
  });

  it("removes an item from the list when deleted", async () => {
    list.mockResolvedValue([item()]);
    deletePlanItem.mockResolvedValue(undefined);
    render(<DayView />);

    await screen.findByText("Задача");
    vi.useFakeTimers();
    fireEvent.pointerDown(screen.getByTitle("Удалить"), { button: 0, pointerId: 1 });
    await act(async () => {
      vi.advanceTimersByTime(950);
    });
    vi.useRealTimers();

    expect(deletePlanItem).toHaveBeenCalledWith("1");
    expect(screen.queryByText("Задача")).not.toBeInTheDocument();
  });

  it("keeps an item when delete hold is released early", async () => {
    list.mockResolvedValue([item()]);
    deletePlanItem.mockResolvedValue(undefined);
    render(<DayView />);

    await screen.findByText("Задача");
    vi.useFakeTimers();
    const deleteButton = screen.getByTitle("Удалить");
    fireEvent.pointerDown(deleteButton, { button: 0, pointerId: 1 });
    await act(async () => {
      vi.advanceTimersByTime(350);
    });
    fireEvent.pointerUp(deleteButton, { button: 0, pointerId: 1 });
    await act(async () => {
      vi.advanceTimersByTime(700);
    });
    vi.useRealTimers();

    expect(deletePlanItem).not.toHaveBeenCalled();
    expect(screen.getByText("Задача")).toBeInTheDocument();
  });
});
