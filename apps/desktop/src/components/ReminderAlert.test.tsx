import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ReminderAlert } from "./ReminderAlert";

const { getCurrentAlert, acknowledge, snooze } = vi.hoisted(() => ({
  getCurrentAlert: vi.fn(),
  acknowledge: vi.fn().mockResolvedValue(undefined),
  snooze: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("../shared/commands", () => ({
  commands: { reminders: { getCurrentAlert, acknowledge, snooze } },
}));

vi.mock("../shared/playChime", () => ({ playChime: vi.fn() }));

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
  });

  it("snoozes for 5 minutes with a computed future timestamp", async () => {
    getCurrentAlert.mockResolvedValue({
      id: "2",
      title: "Позвонить",
      triggerAtUtc: "2026-07-04T18:30:00.000Z",
      status: "firing",
    });
    const user = userEvent.setup();
    render(<ReminderAlert />);

    await screen.findByText("Позвонить");
    await user.click(screen.getByText("5 мин"));

    expect(snooze).toHaveBeenCalledWith("2", expect.any(String));
  });
});
