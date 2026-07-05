import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { NotesView } from "./NotesView";

const { list, create } = vi.hoisted(() => ({
  list: vi.fn(),
  create: vi.fn(),
}));

vi.mock("../shared/commands", () => ({
  commands: { notes: { list, create } },
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe("NotesView", () => {
  it("loads and shows persisted notes", async () => {
    list.mockResolvedValue([
      { id: "1", title: null, body: "Идея для раздела 14", kind: "text" },
    ]);
    render(<NotesView />);

    expect(await screen.findByText("Идея для раздела 14")).toBeInTheDocument();
  });

  it("shows an empty state when there are no notes", async () => {
    list.mockResolvedValue([]);
    render(<NotesView />);

    expect(await screen.findByText("Пока нет заметок")).toBeInTheDocument();
  });

  it("adds a note through the quick-add form", async () => {
    list.mockResolvedValue([]);
    create.mockResolvedValue({ id: "2", title: null, body: "Новая заметка", kind: "text" });
    const user = userEvent.setup();
    render(<NotesView />);

    await screen.findByText("Пока нет заметок");
    await user.type(
      screen.getByPlaceholderText("Новая заметка..."),
      "Новая заметка{Enter}",
    );

    expect(create).toHaveBeenCalledWith("Новая заметка");
    expect(await screen.findByText("Новая заметка")).toBeInTheDocument();
  });
});
