import type { Locale } from "./commands";
import { INTL_LOCALE_TAG, translate } from "./translations";

export interface ReminderPreset {
  key: string;
  label: string;
  computeTriggerAtUtc: () => string;
}

function inMinutes(minutes: number): string {
  return new Date(Date.now() + minutes * 60_000).toISOString();
}

function tomorrowAt(hour: number, minute: number): string {
  const date = new Date();
  date.setDate(date.getDate() + 1);
  date.setHours(hour, minute, 0, 0);
  return date.toISOString();
}

// Полноценный natural-language parser — Iteration 3 ТЗ (universal quick
// capture). Для быстрых случаев — пресеты, для остального — "Своё время"
// с обычным datetime-local вводом (RemindersView). Функция, а не статический
// массив — подписи пресетов зависят от locale (раздел 22 ТЗ, i18n).
export function getReminderPresets(locale: Locale): ReminderPreset[] {
  return [
    { key: "15m", label: translate(locale, "reminders.preset15m"), computeTriggerAtUtc: () => inMinutes(15) },
    { key: "1h", label: translate(locale, "reminders.preset1h"), computeTriggerAtUtc: () => inMinutes(60) },
    {
      key: "tomorrow-9",
      label: translate(locale, "reminders.presetTomorrow9"),
      computeTriggerAtUtc: () => tomorrowAt(9, 0),
    },
  ];
}

// Значение для min= у <input type="datetime-local">: локальное время без
// секунд/таймзоны — ровно то, что этот тип input ожидает и производит.
export function nowAsDatetimeLocal(): string {
  const now = new Date();
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}T${pad(now.getHours())}:${pad(now.getMinutes())}`;
}

export function formatReminderTime(triggerAtUtc: string, locale: Locale): string {
  const date = new Date(triggerAtUtc);
  const now = new Date();
  const tomorrow = new Date(now);
  tomorrow.setDate(now.getDate() + 1);
  const localeTag = INTL_LOCALE_TAG[locale];

  const time = new Intl.DateTimeFormat(localeTag, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);

  if (date.toDateString() === now.toDateString()) {
    return `${translate(locale, "reminders.today")}, ${time}`;
  }
  if (date.toDateString() === tomorrow.toDateString()) {
    return `${translate(locale, "reminders.tomorrow")}, ${time}`;
  }

  const day = new Intl.DateTimeFormat(localeTag, {
    day: "numeric",
    month: "long",
  }).format(date);
  return `${day}, ${time}`;
}
