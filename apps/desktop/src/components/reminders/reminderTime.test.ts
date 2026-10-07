import { describe, expect, it } from "vitest";
import { computeValidTriggerIso, formatCountdown, pad, wrapTimeValue } from "./reminderTime";

const t = (key: string) => key;

describe("reminderTime", () => {
  it("pads values to two digits", () => {
    expect(pad(5)).toBe("05");
    expect(pad(12)).toBe("12");
  });

  it("wraps stepper values around the range", () => {
    expect(wrapTimeValue(24, 0, 23)).toBe(0);
    expect(wrapTimeValue(-1, 0, 23)).toBe(23);
    expect(wrapTimeValue(7, 0, 23)).toBe(7);
  });

  it("rejects out-of-range, non-integer and past trigger times", () => {
    const now = new Date(2026, 6, 1, 10, 0).getTime();
    expect(computeValidTriggerIso("2026-07-01", 24, 0, now)).toBeNull();
    expect(computeValidTriggerIso("2026-07-01", 10, 60, now)).toBeNull();
    expect(computeValidTriggerIso("2026-07-01", 10.5, 0, now)).toBeNull();
    expect(computeValidTriggerIso("2026-07-01", 10, 0, now)).toBeNull();
  });

  it("returns an ISO string for a future local time", () => {
    const now = new Date(2026, 6, 1, 10, 0).getTime();
    const iso = computeValidTriggerIso("2026-07-01", 18, 30, now);
    expect(iso).toBe(new Date(2026, 6, 1, 18, 30).toISOString());
  });

  it("formats countdowns for due, minutes and hours", () => {
    const now = Date.UTC(2026, 6, 1, 10, 0);
    const at = (minutes: number) => new Date(now + minutes * 60_000).toISOString();
    expect(formatCountdown(at(-1), now, t)).toBe("reminders.countdownDue");
    expect(formatCountdown(at(5), now, t)).toBe("reminders.countdownMinutes");
    expect(formatCountdown(at(120), now, t)).toBe("reminders.countdownHoursOnly");
    expect(formatCountdown(at(90), now, t)).toBe("reminders.countdownHours");
    expect(formatCountdown(at(5), 0, t)).toBe("");
  });
});
