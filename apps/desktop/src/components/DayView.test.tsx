import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DayView } from "./DayView";

const { list, listRange, create, toggleDone, cycleProgress, toggleDeferred, moveToDate, rollOverPending, deletePlanItem, listReminders, onRemindersChanged } =
  vi.hoisted(() => ({
    list: vi.fn(),
    listRange: vi.fn(),
    create: vi.fn(),
    toggleDone: vi.fn(),
    cycleProgress: vi.fn(),
    toggleDeferred: vi.fn(),
    moveToDate: vi.fn(),
    rollOverPending: vi.fn(),
    deletePlanItem: vi.fn(),
    listReminders: vi.fn(),
    onRemindersChanged: vi.fn().mockResolvedValue(() => {}),
  }));

vi.mock("../shared/commands", () => ({
  commands: {
    planItems: { list, listRange, create, toggleDone, cycleProgress, toggleDeferred, moveToDate, rollOverPending, delete: deletePlanItem },
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
    const user = userEvent.setup();
    render(<DayView />);

    await screen.findByText("Задача");
    await user.click(screen.getByTitle("Удалить"));

    expect(deletePlanItem).toHaveBeenCalledWith("1");
    expect(screen.queryByText("Задача")).not.toBeInTheDocument();
  });
});
