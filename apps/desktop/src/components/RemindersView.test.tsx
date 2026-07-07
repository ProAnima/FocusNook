import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RemindersView } from "./RemindersView";

const { list, create, createAudio, deleteReminder, onChanged, startRecording, stopRecording } = vi.hoisted(() => ({
  list: vi.fn(),
  create: vi.fn(),
  createAudio: vi.fn(),
  deleteReminder: vi.fn(),
  onChanged: vi.fn().mockResolvedValue(() => {}),
  startRecording: vi.fn(),
  stopRecording: vi.fn(),
}));

vi.mock("../shared/commands", () => ({
  commands: {
    reminders: { list, create, createAudio, delete: deleteReminder, onChanged },
    serverSync: { onCompleted: vi.fn().mockResolvedValue(() => {}) },
  },
}));

vi.mock("../shared/useMicrophoneSettings", () => ({
  useMicrophoneSettings: () => ({ selectedDeviceId: null }),
}));

vi.mock("../shared/useAudioRecorder", () => ({
  useAudioRecorder: (onRecorded: (base64: string) => void) => ({
    recording: false,
    error: null,
    start: () => {
      startRecording();
      onRecorded("dm9pY2U=");
    },
    stop: stopRecording,
  }),
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

    expect(screen.getByLabelText("Дата и время напоминания")).toBeInTheDocument();
    await user.click(screen.getByText("Добавить"));

    expect(create).toHaveBeenCalledWith("Встреча с командой", expect.any(String));
    expect(await screen.findByText("Встреча с командой")).toBeInTheDocument();
  });

  it("opens the shared calendar and supports mouse-first time controls", async () => {
    list.mockResolvedValue([]);
    create.mockResolvedValue({
      id: "4",
      title: "custom-time",
      triggerAtUtc: "2030-01-01T18:00:00.000Z",
      status: "scheduled",
    });
    const user = userEvent.setup();
    render(<RemindersView />);
    await screen.findByText("Нет активных напоминаний");

    await user.type(screen.getByPlaceholderText("Напомнить о..."), "Просроченное");
    await user.click(screen.getByText("Своё время"));

    await user.click(screen.getByLabelText("Дата и время напоминания"));
    expect(document.querySelector(".calendar-popover")).toBeInTheDocument();
    await user.click(screen.getByText("+15 мин"));
    expect(within(screen.getByLabelText("Время напоминания")).getAllByText(/\d{2}/)).toHaveLength(2);
    await user.click(screen.getByText("Добавить"));

    expect(create).toHaveBeenCalledWith("Просроченное", expect.any(String));
  });

  it("records and creates a voice reminder", async () => {
    list.mockResolvedValue([]);
    createAudio.mockResolvedValue({
      id: "voice-1",
      title: "Голосовое напоминание",
      audioPath: "reminder-voice-1.webm",
      triggerAtUtc: "2030-01-01T10:00:00.000Z",
      status: "scheduled",
    });
    const user = userEvent.setup();
    render(<RemindersView />);
    await screen.findByText("Нет активных напоминаний");

    await user.click(screen.getByTitle("Записать голосовое напоминание"));
    expect(await screen.findByText("Голос готов")).toBeInTheDocument();
    await user.click(screen.getByText("Через 15 мин"));

    expect(createAudio).toHaveBeenCalledWith("Голосовое напоминание", expect.any(String), "dm9pY2U=");
    expect(await screen.findByText("Голосовое напоминание")).toBeInTheDocument();
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
