import type { Locale } from "./commands";
import { INTL_LOCALE_TAG } from "./translations";

const DAY_MS = 24 * 60 * 60 * 1000;

function pad(value: number): string {
  return String(value).padStart(2, "0");
}

export function dateKeyFromDate(date: Date): string {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

export function todayDateKey(): string {
  return dateKeyFromDate(new Date());
}

export function parseDateKey(dateKey: string): Date {
  const [year, month, day] = dateKey.split("-").map(Number);
  return new Date(year, month - 1, day);
}

export function addDays(dateKey: string, days: number): string {
  const date = parseDateKey(dateKey);
  date.setDate(date.getDate() + days);
  return dateKeyFromDate(date);
}

export function formatDayLabel(dateKey: string, locale: Locale): string {
  return new Intl.DateTimeFormat(INTL_LOCALE_TAG[locale], {
    weekday: "long",
    day: "numeric",
    month: "long",
  }).format(parseDateKey(dateKey));
}

export function formatMonthLabel(monthKey: string, locale: Locale): string {
  const [year, month] = monthKey.split("-").map(Number);
  return new Intl.DateTimeFormat(INTL_LOCALE_TAG[locale], {
    month: "long",
    year: "numeric",
  }).format(new Date(year, month - 1, 1));
}

export function monthKeyFromDateKey(dateKey: string): string {
  return dateKey.slice(0, 7);
}

export function addMonths(monthKey: string, months: number): string {
  const [year, month] = monthKey.split("-").map(Number);
  const date = new Date(year, month - 1 + months, 1);
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}`;
}

export function monthGrid(monthKey: string): string[] {
  const [year, month] = monthKey.split("-").map(Number);
  const first = new Date(year, month - 1, 1);
  const start = new Date(first);
  const mondayFirstOffset = (first.getDay() + 6) % 7;
  start.setDate(first.getDate() - mondayFirstOffset);

  return Array.from({ length: 42 }, (_, index) => {
    const date = new Date(start.getTime() + index * DAY_MS);
    return dateKeyFromDate(date);
  });
}

export function monthRange(monthKey: string): { startDate: string; endDate: string } {
  const days = monthGrid(monthKey);
  return { startDate: days[0], endDate: days[days.length - 1] };
}

export function reminderDateKey(triggerAtUtc: string): string {
  return dateKeyFromDate(new Date(triggerAtUtc));
}
