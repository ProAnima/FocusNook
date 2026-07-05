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

// Полноценный ввод времени/natural-language parser — Iteration 3 ТЗ
// (universal quick capture). Для первого среза достаточно пресетов.
export const REMINDER_PRESETS: ReminderPreset[] = [
  { key: "15m", label: "Через 15 мин", computeTriggerAtUtc: () => inMinutes(15) },
  { key: "1h", label: "Через час", computeTriggerAtUtc: () => inMinutes(60) },
  { key: "tomorrow-9", label: "Завтра в 9:00", computeTriggerAtUtc: () => tomorrowAt(9, 0) },
];

export function formatReminderTime(triggerAtUtc: string): string {
  const date = new Date(triggerAtUtc);
  const now = new Date();
  const tomorrow = new Date(now);
  tomorrow.setDate(now.getDate() + 1);

  const time = new Intl.DateTimeFormat("ru-RU", {
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);

  if (date.toDateString() === now.toDateString()) return `Сегодня, ${time}`;
  if (date.toDateString() === tomorrow.toDateString()) return `Завтра, ${time}`;

  const day = new Intl.DateTimeFormat("ru-RU", {
    day: "numeric",
    month: "long",
  }).format(date);
  return `${day}, ${time}`;
}
