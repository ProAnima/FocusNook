import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ReminderAlert } from "./ReminderAlert";

const { getCurrentAlert, getAudio, acknowledge, snooze, stopChime } = vi.hoisted(() => ({
  getCurrentAlert: vi.fn(),
  getAudio: vi.fn(),
  acknowledge: vi.fn().mockResolvedValue(undefined),
  snooze: vi.fn().mockResolvedValue(undefined),
  stopChime: vi.fn(),
}));

vi.mock("../shared/commands", () => ({
  commands: { reminders: { getCurrentAlert, getAudio, acknowledge, snooze } },
}));

vi.mock("../shared/playChime", () => ({
  playChime: vi.fn(() => ({ done: Promise.resolve(), stop: stopChime })),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe("ReminderAlert", () => {
  it("renders nothing while there is no current alert", async () => {
    getCurrentAlert.mockResolvedValue(null);
    const { container } = render(<ReminderAlert />);

    await vi.waitFor(() => expect(getCurrentAlert).toHaveBeenCalled());
    expect(container).toBeEmptyDOMElement();
  });

  it("shows the reminder and acknowledges it", async () => {
    getCurrentAlert.mockResolvedValue({
      id: "1",
      title: "Проверить рендер",
      triggerAtUtc: "2026-07-04T18:30:00.000Z",
      status: "firing",
    });
    const user = userEvent.setup();
    render(<ReminderAlert />);

    await screen.findByText("Проверить рендер");
    await user.click(screen.getByText("Услышал"));

    expect(acknowledge).toHaveBeenCalledWith("1");
    expect(stopChime).toHaveBeenCalled();
  });

  it("snoozes for 10 minutes with a computed future timestamp", async () => {
    getCurrentAlert.mockResolvedValue({
      id: "2",
      title: "Позвонить",
      triggerAtUtc: "2026-07-04T18:30:00.000Z",
      status: "firing",
    });
    const user = userEvent.setup();
    render(<ReminderAlert />);

    await screen.findByText("Позвонить");
    await user.click(screen.getByText("10 мин"));

    expect(snooze).toHaveBeenCalledWith("2", expect.any(String));
  });

  it("moves keyboard focus to the primary action when it appears", async () => {
    getCurrentAlert.mockResolvedValue({
      id: "3",
      title: "Фокус на кнопке",
      triggerAtUtc: "2026-07-04T18:30:00.000Z",
      status: "firing",
    });
    render(<ReminderAlert />);

    const primary = await screen.findByText("Услышал");
    await vi.waitFor(() => expect(primary).toHaveFocus());
  });

  it("plays voice reminder audio after the chime", async () => {
    getCurrentAlert.mockResolvedValue({
      id: "4",
      title: "Голосовое напоминание",
      audioPath: "reminder-4.webm",
      triggerAtUtc: "2026-07-04T18:30:00.000Z",
      status: "firing",
    });
    getAudio.mockResolvedValue("dm9pY2U=");
    const playAudio = vi.fn().mockResolvedValue(undefined);
    class MockAudio {
      currentTime = 0;
      src = "";
      pause = vi.fn();
      play = playAudio;

      constructor(src: string) {
        this.src = src;
      }
    }
    vi.stubGlobal("Audio", MockAudio);

    render(<ReminderAlert />);

    await screen.findByText("Голосовое напоминание");
    await vi.waitFor(() => expect(getAudio).toHaveBeenCalledWith("4"));
    expect(playAudio).toHaveBeenCalled();
  });
});
