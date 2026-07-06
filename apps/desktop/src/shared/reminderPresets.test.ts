import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { formatReminderTime } from "./reminderPresets";

// Фиксируем время — иначе тест "сегодня" мог бы стать "завтра" при запуске
// в 23:xx и наоборот, флейково от времени суток на машине CI/разработчика.
const NOON = new Date("2026-07-04T12:00:00.000Z");

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(NOON);
});

afterEach(() => {
  vi.useRealTimers();
});

describe("formatReminderTime", () => {
  it("labels a time later today as 'Сегодня'", () => {
    const inTwoHours = new Date(NOON.getTime() + 2 * 60 * 60 * 1000).toISOString();
    expect(formatReminderTime(inTwoHours, "ru")).toMatch(/^Сегодня, \d{2}:\d{2}$/);
  });

  it("labels a time tomorrow as 'Завтра'", () => {
    const tomorrow = new Date(NOON);
    tomorrow.setDate(tomorrow.getDate() + 1);
    expect(formatReminderTime(tomorrow.toISOString(), "ru")).toMatch(/^Завтра, \d{2}:\d{2}$/);
  });

  it("shows the date for anything further out", () => {
    const nextWeek = new Date(NOON);
    nextWeek.setDate(nextWeek.getDate() + 7);
    const result = formatReminderTime(nextWeek.toISOString(), "ru");
    expect(result).not.toMatch(/^Сегодня/);
    expect(result).not.toMatch(/^Завтра/);
  });

  it("labels a time later today as 'Today' in English", () => {
    const inTwoHours = new Date(NOON.getTime() + 2 * 60 * 60 * 1000).toISOString();
    expect(formatReminderTime(inTwoHours, "en")).toMatch(/^Today, \d{1,2}:\d{2} [AP]M$/);
  });
});
