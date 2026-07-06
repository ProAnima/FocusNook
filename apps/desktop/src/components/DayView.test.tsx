import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DayView } from "./DayView";

const { list, create, toggleDone, cycleProgress, toggleDeferred, deletePlanItem } = vi.hoisted(() => ({
  list: vi.fn(),
  create: vi.fn(),
  toggleDone: vi.fn(),
  cycleProgress: vi.fn(),
  toggleDeferred: vi.fn(),
  deletePlanItem: vi.fn(),
}));

vi.mock("../shared/commands", () => ({
  commands: {
    planItems: { list, create, toggleDone, cycleProgress, toggleDeferred, delete: deletePlanItem },
  },
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe("DayView", () => {
  it("loads persisted items and shows the done count", async () => {
    list.mockResolvedValue([
      { id: "1", title: "Проверить рендер", status: "open", progressPercent: null },
      { id: "2", title: "Готово", status: "done", progressPercent: null },
    ]);
    render(<DayView />);

    expect(await screen.findByText("Проверить рендер")).toBeInTheDocument();
    expect(screen.getByText("1/2")).toBeInTheDocument();
  });

  it("shows an empty state when there are no items", async () => {
    list.mockResolvedValue([]);
    render(<DayView />);

    expect(
      await screen.findByText("На сегодня пока ничего не запланировано"),
    ).toBeInTheDocument();
  });

  it("adds a new item through the quick-add form", async () => {
    list.mockResolvedValue([]);
    create.mockResolvedValue({
      id: "3",
      title: "Новое дело",
      status: "open",
      progressPercent: null,
    });
    const user = userEvent.setup();
    render(<DayView />);

    await screen.findByText("На сегодня пока ничего не запланировано");
    await user.type(
      screen.getByPlaceholderText("Добавить дело..."),
      "Новое дело{Enter}",
    );

    expect(create).toHaveBeenCalledWith("Новое дело");
    expect(await screen.findByText("Новое дело")).toBeInTheDocument();
  });

  it("marks an item done when its checkbox is clicked", async () => {
    list.mockResolvedValue([
      { id: "1", title: "Задача", status: "open", progressPercent: null },
    ]);
    toggleDone.mockResolvedValue({
      id: "1",
      title: "Задача",
      status: "done",
      progressPercent: null,
    });
    const user = userEvent.setup();
    render(<DayView />);

    await user.click(
      await screen.findByRole("button", { name: "Отметить выполненным" }),
    );

    expect(toggleDone).toHaveBeenCalledWith("1");
    expect(await screen.findByText("1/1")).toBeInTheDocument();
  });

  it("steps progress forward when the partial button is clicked", async () => {
    list.mockResolvedValue([
      { id: "1", title: "Задача", status: "open", progressPercent: null },
    ]);
    cycleProgress.mockResolvedValue({
      id: "1",
      title: "Задача",
      status: "partial",
      progressPercent: 25,
    });
    const user = userEvent.setup();
    render(<DayView />);

    await user.click(await screen.findByTitle("Частично выполнено"));

    expect(cycleProgress).toHaveBeenCalledWith("1");
    expect(await screen.findByText("25%")).toBeInTheDocument();
  });

  it("defers an item and can bring it back", async () => {
    list.mockResolvedValue([
      { id: "1", title: "Задача", status: "open", progressPercent: null },
    ]);
    toggleDeferred.mockResolvedValue({
      id: "1",
      title: "Задача",
      status: "deferred",
      progressPercent: null,
    });
    const user = userEvent.setup();
    render(<DayView />);

    await user.click(await screen.findByTitle("Отложить"));

    expect(toggleDeferred).toHaveBeenCalledWith("1");
    expect(await screen.findByTitle("Вернуть в работу")).toBeInTheDocument();
  });

  it("removes an item from the list when deleted", async () => {
    list.mockResolvedValue([
      { id: "1", title: "Задача", status: "open", progressPercent: null },
    ]);
    deletePlanItem.mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(<DayView />);

    await screen.findByText("Задача");
    await user.click(screen.getByTitle("Удалить"));

    expect(deletePlanItem).toHaveBeenCalledWith("1");
    expect(screen.queryByText("Задача")).not.toBeInTheDocument();
  });
});
