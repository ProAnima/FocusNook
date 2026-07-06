import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { NotesView } from "./NotesView";

const { list, create, createAudio, getAudio, deleteNote } = vi.hoisted(() => ({
  list: vi.fn(),
  create: vi.fn(),
  createAudio: vi.fn(),
  getAudio: vi.fn(),
  deleteNote: vi.fn(),
}));

vi.mock("../shared/commands", () => ({
  commands: { notes: { list, create, createAudio, getAudio, delete: deleteNote } },
}));

class FakeMediaRecorder {
  ondataavailable: ((event: { data: Blob }) => void) | null = null;
  onstop: (() => void) | null = null;
  constructor(public stream: MediaStream) {}
  start() {}
  stop() {
    this.ondataavailable?.({ data: new Blob(["fake-audio"], { type: "audio/webm" }) });
    this.onstop?.();
  }
}

beforeEach(() => {
  vi.clearAllMocks();
  URL.createObjectURL = vi.fn(() => "blob:mock-url");
  URL.revokeObjectURL = vi.fn();
});

describe("NotesView", () => {
  it("loads and shows persisted notes", async () => {
    list.mockResolvedValue([
      { id: "1", title: null, body: "Идея для раздела 14", kind: "text", audioPath: null },
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
    create.mockResolvedValue({ id: "2", title: null, body: "Новая заметка", kind: "text", audioPath: null });
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

  it("removes a note from the list when deleted", async () => {
    list.mockResolvedValue([
      { id: "1", title: null, body: "Удали меня", kind: "text", audioPath: null },
    ]);
    deleteNote.mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(<NotesView />);

    await screen.findByText("Удали меня");
    await user.click(screen.getByTitle("Удалить"));

    expect(deleteNote).toHaveBeenCalledWith("1");
    expect(screen.queryByText("Удали меня")).not.toBeInTheDocument();
  });

  it("renders an audio player for audio notes", async () => {
    list.mockResolvedValue([
      { id: "1", title: null, body: "", kind: "audio", audioPath: "1.webm" },
    ]);
    getAudio.mockResolvedValue("ZmFrZS1hdWRpbw==");
    const { container } = render(<NotesView />);

    await waitFor(() => expect(getAudio).toHaveBeenCalledWith("1"));
    expect(container.querySelector("audio")).toBeInTheDocument();
  });

  it("shows an error when the microphone is unavailable", async () => {
    list.mockResolvedValue([]);
    const user = userEvent.setup();
    render(<NotesView />);

    await screen.findByText("Пока нет заметок");
    await user.click(screen.getByTitle("Записать голосовую заметку"));

    expect(await screen.findByText("Микрофон недоступен")).toBeInTheDocument();
  });

  it("records and adds an audio note", async () => {
    list.mockResolvedValue([]);
    createAudio.mockResolvedValue({ id: "3", title: null, body: "", kind: "audio", audioPath: "3.webm" });
    vi.stubGlobal("MediaRecorder", FakeMediaRecorder);
    Object.defineProperty(navigator, "mediaDevices", {
      value: { getUserMedia: vi.fn().mockResolvedValue({ getTracks: () => [] }) },
      configurable: true,
    });

    const user = userEvent.setup();
    render(<NotesView />);
    await screen.findByText("Пока нет заметок");

    await user.click(screen.getByTitle("Записать голосовую заметку"));
    await user.click(await screen.findByTitle("Остановить запись"));

    await waitFor(() => expect(createAudio).toHaveBeenCalled());
  });
});
