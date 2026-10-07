import { dateKeyFromDate, parseDateKey } from "../../shared/dateKeys";
import type { LocaleContextValue } from "../../shared/locale-context";

const MINUTE_MS = 60_000;
export const DEFAULT_CUSTOM_OFFSET_MINUTES = 15;

export function pad(value: number): string {
  return value.toString().padStart(2, "0");
}

/** Стартовое значение для пользовательского времени: «сейчас + 15 минут». */
export function defaultCustomTime(now = Date.now()) {
  const date = new Date(now + DEFAULT_CUSTOM_OFFSET_MINUTES * MINUTE_MS);
  return {
    dateKey: dateKeyFromDate(date),
    hour: date.getHours(),
    minute: date.getMinutes(),
  };
}

/** ISO-время срабатывания или null, если часы/минуты некорректны либо момент уже в прошлом. */
export function computeValidTriggerIso(dateKey: string, hour: number, minute: number, now = Date.now()): string | null {
  if (!Number.isInteger(hour) || !Number.isInteger(minute)) return null;
  if (hour < 0 || hour > 23 || minute < 0 || minute > 59) return null;
  const date = parseDateKey(dateKey);
  const triggerAt = new Date(date.getFullYear(), date.getMonth(), date.getDate(), hour, minute, 0, 0);
  if (Number.isNaN(triggerAt.getTime()) || triggerAt.getTime() <= now) return null;
  return triggerAt.toISOString();
}

export function formatCountdown(triggerAtUtc: string, now: number, t: LocaleContextValue["t"]): string {
  if (now <= 0) return "";
  const diffMs = new Date(triggerAtUtc).getTime() - now;
  if (!Number.isFinite(diffMs) || diffMs <= 0) return t("reminders.countdownDue");
  const totalMinutes = Math.max(1, Math.ceil(diffMs / MINUTE_MS));
  if (totalMinutes < 60) {
    return t("reminders.countdownMinutes").replace("{minutes}", String(totalMinutes));
  }
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (minutes === 0) {
    return t("reminders.countdownHoursOnly").replace("{hours}", String(hours));
  }
  return t("reminders.countdownHours")
    .replace("{hours}", String(hours))
    .replace("{minutes}", String(minutes));
}

/** Циклический шаг значения в диапазоне [min, max]: за max идёт min и наоборот. */
export function wrapTimeValue(value: number, min: number, max: number): number {
  if (value > max) return min;
  if (value < min) return max;
  return value;
}
