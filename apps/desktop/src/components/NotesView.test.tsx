import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { NotesView } from "./NotesView";

const {
  list,
  listGroups,
  createGroup,
  create,
  createAudio,
  getAudio,
  moveToGroup,
  updateNote,
  deleteNote,
  getMicrophoneDeviceId,
  setMicrophoneDeviceId,
  getNoteFolderSort,
  setNoteFolderSort,
} = vi.hoisted(() => ({
  list: vi.fn(),
  listGroups: vi.fn(),
  createGroup: vi.fn(),
  create: vi.fn(),
  createAudio: vi.fn(),
  getAudio: vi.fn(),
  moveToGroup: vi.fn(),
  updateNote: vi.fn(),
  deleteNote: vi.fn(),
  getMicrophoneDeviceId: vi.fn(),
  setMicrophoneDeviceId: vi.fn(),
  getNoteFolderSort: vi.fn(),
  setNoteFolderSort: vi.fn(),
}));

vi.mock("../shared/commands", () => ({
  commands: {
    notes: { list, listGroups, createGroup, create, createAudio, getAudio, moveToGroup, update: updateNote, delete: deleteNote },
    settings: { getMicrophoneDeviceId, setMicrophoneDeviceId, getNoteFolderSort, setNoteFolderSort },
    serverSync: { onCompleted: vi.fn().mockResolvedValue(() => {}) },
  },
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

function note(overrides = {}) {
  return { id: "1", title: null, body: "Идея", kind: "text", audioPath: null, groupId: null, ...overrides };
}

beforeEach(() => {
  vi.clearAllMocks();
  list.mockResolvedValue([]);
  listGroups.mockResolvedValue([]);
  getMicrophoneDeviceId.mockResolvedValue(null);
  setMicrophoneDeviceId.mockResolvedValue(undefined);
  getNoteFolderSort.mockResolvedValue("recent");
  setNoteFolderSort.mockResolvedValue(undefined);
  URL.createObjectURL = vi.fn(() => "blob:mock-url");
  URL.revokeObjectURL = vi.fn();
});

async function openFolders() {
  await screen.findByTitle("Все");
}

describe("NotesView", () => {
  it("loads and shows persisted notes", async () => {
    list.mockResolvedValue([note({ body: "Идея для раздела 14" })]);
    render(<NotesView />);

    expect(await screen.findByText("Идея для раздела 14")).toBeInTheDocument();
  });

  it("shows an empty state when there are no notes", async () => {
    render(<NotesView />);

    expect(await screen.findByText("Пока нет заметок")).toBeInTheDocument();
  });

  it("adds a note through the quick-add form in the active folder", async () => {
    listGroups.mockResolvedValue([{ id: "g1", name: "Проект" }]);
    create.mockResolvedValue(note({ id: "2", body: "Новая заметка", groupId: "g1" }));
    const user = userEvent.setup();
    render(<NotesView />);

    await openFolders();
    await user.click(await screen.findByTitle("Проект"));
    await user.type(screen.getByPlaceholderText("Новая заметка..."), "Новая заметка{Enter}");

    expect(create).toHaveBeenCalledWith("Новая заметка", "g1");
    expect(await screen.findByText("Новая заметка")).toBeInTheDocument();
  });

  it("creates a folder from the folder composer", async () => {
    createGroup.mockResolvedValue({ id: "g1", name: "Идеи" });
    const user = userEvent.setup();
    render(<NotesView />);

    await openFolders();
    await user.click(await screen.findByTitle("Создать папку"));
    await user.type(await screen.findByPlaceholderText("Новая папка..."), "Идеи{Enter}");

    expect(createGroup).toHaveBeenCalledWith("Идеи");
    expect(await screen.findByTitle("Идеи")).toBeInTheDocument();
  });

  it("selects folders through the mobile bottom sheet", async () => {
    listGroups.mockResolvedValue([{ id: "g1", name: "Мобильное" }]);
    create.mockResolvedValue(note({ id: "2", body: "Заметка с телефона", groupId: "g1" }));
    const user = userEvent.setup();
    render(<NotesView isDesktop={false} />);

    await user.click(await screen.findByRole("button", { name: "Папки" }));
    await user.click(await screen.findByRole("button", { name: /Мобильное/ }));
    await user.type(screen.getByPlaceholderText("Новая заметка..."), "Заметка с телефона{Enter}");

    expect(create).toHaveBeenCalledWith("Заметка с телефона", "g1");
  });

  it("closes the mobile folder sheet with escape", async () => {
    const user = userEvent.setup();
    render(<NotesView isDesktop={false} />);

    await user.click(await screen.findByRole("button", { name: "Папки" }));
    expect(screen.getByRole("dialog", { name: "Папки" })).toBeInTheDocument();
    expect(document.body.style.overflow).toBe("hidden");

    await user.keyboard("{Escape}");

    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Папки" })).not.toBeInTheDocument());
    expect(document.body.style.overflow).toBe("");
  });

  it("moves a note to a folder with drag and drop", async () => {
    list.mockResolvedValue([note({ id: "n1", body: "Перетащи меня" })]);
    listGroups.mockResolvedValue([{ id: "g1", name: "Архив" }]);
    moveToGroup.mockResolvedValue(note({ id: "n1", body: "Перетащи меня", groupId: "g1" }));
    render(<NotesView />);

    const row = await screen.findByText("Перетащи меня");
    await openFolders();
    const folder = await screen.findByTitle("Архив");
    const dataTransfer = {
      data: "",
      effectAllowed: "",
      setData(_type: string, value: string) {
        this.data = value;
      },
      getData() {
        return this.data;
      },
    };

    fireEvent.dragStart(row.closest(".note-item")!, { dataTransfer });
    fireEvent.drop(folder.closest("button")!, { dataTransfer });

    await waitFor(() => expect(moveToGroup).toHaveBeenCalledWith("n1", "g1"));
  });

  it("keeps shift-enter as a line break in the note composer", async () => {
    create.mockResolvedValue(note({ id: "2", body: "Первая\nВторая" }));
    const user = userEvent.setup();
    render(<NotesView />);

    const composer = await screen.findByPlaceholderText("Новая заметка...");
    await user.type(composer, "Первая{Shift>}{Enter}{/Shift}Вторая{Enter}");

    expect(create).toHaveBeenCalledWith("Первая\nВторая", null);
  });

  it("edits a text note inline", async () => {
    list.mockResolvedValue([note({ id: "n1", body: "Черновик" })]);
    updateNote.mockResolvedValue(note({ id: "n1", body: "Готовая мысль" }));
    const user = userEvent.setup();
    render(<NotesView />);

    await screen.findByText("Черновик");
    await user.click(screen.getByTitle("Редактировать"));
    const editor = document.querySelector(".note-editor textarea") as HTMLTextAreaElement;
    await user.clear(editor);
    await user.type(editor, "Готовая мысль");
    await user.click(screen.getByTitle("Сохранить"));

    expect(updateNote).toHaveBeenCalledWith("n1", "Готовая мысль");
    expect(await screen.findByText("Готовая мысль")).toBeInTheDocument();
  });

  it("removes a note from the list when deleted", async () => {
    list.mockResolvedValue([note({ body: "Удали меня" })]);
    deleteNote.mockResolvedValue(undefined);
    render(<NotesView />);

    await screen.findByText("Удали меня");
    vi.useFakeTimers();
    fireEvent.pointerDown(screen.getByTitle("Удалить"), { button: 0, pointerId: 1 });
    await act(async () => {
      vi.advanceTimersByTime(950);
    });
    vi.useRealTimers();

    expect(deleteNote).toHaveBeenCalledWith("1");
    expect(screen.queryByText("Удали меня")).not.toBeInTheDocument();
  });

  it("renders an audio player for audio notes", async () => {
    list.mockResolvedValue([note({ body: "", kind: "audio", audioPath: "1.webm" })]);
    getAudio.mockResolvedValue("ZmFrZS1hdWRpbw==");
    const { container } = render(<NotesView />);

    await waitFor(() => expect(getAudio).toHaveBeenCalledWith("1"));
    expect(container.querySelector("audio")).toBeInTheDocument();
  });

  it("shows an error when the microphone is unavailable", async () => {
    const user = userEvent.setup();
    render(<NotesView />);

    await screen.findByText("Пока нет заметок");
    await user.click(screen.getByTitle("Записать голосовую заметку"));

    expect(await screen.findByText("Микрофон недоступен")).toBeInTheDocument();
  });

  it("records and adds an audio note", async () => {
    createAudio.mockResolvedValue(note({ id: "3", body: "", kind: "audio", audioPath: "3.webm" }));
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

    await waitFor(() => expect(createAudio).toHaveBeenCalledWith(expect.any(String), null));
  });
});
