import type { Locale } from "./commands";

// Раздел 22 ТЗ: "i18n ru/en минимум, структура под 10 языков". Плоский
// словарь ключ -> строка на локаль (а не вложенные объекты) — проще добавить
// 3-й/10-й язык (одна запись в LOCALES + один объект здесь), чем мигрировать
// вложенную структуру. `en` типизирован как Record<keyof typeof ru, string> —
// TypeScript не даст забыть перевод при добавлении нового ключа в ru.
const ru = {
  "nav.day": "День",
  "nav.notes": "Заметки",
  "nav.reminders": "Напоминания",
  "nav.settings": "Настройки",

  "header.pinOff": "Убрать с переднего плана",
  "header.pinOn": "Показать поверх окон",
  "header.settings": "Настройки",
  "header.close": "Закрыть",

  "profile.newPlaceholder": "Новый профиль...",

  "common.delete": "Удалить",
  "common.cancel": "Отмена",

  "notes.empty": "Пока нет заметок",
  "notes.newPlaceholder": "Новая заметка...",
  "notes.record": "Записать голосовую заметку",
  "notes.stopRecording": "Остановить запись",
  "notes.micUnavailable": "Микрофон недоступен",

  "day.partial": "Частично выполнено",
  "day.resume": "Вернуть в работу",
  "day.defer": "Отложить",
  "day.markUndone": "Отметить невыполненным",
  "day.markDone": "Отметить выполненным",
  "day.changeProgress": "Изменить прогресс",
  "day.empty": "На сегодня пока ничего не запланировано",
  "day.addPlaceholder": "Добавить дело...",

  "reminders.customTime": "Своё время",
  "reminders.dateTimeLabel": "Дата и время напоминания",
  "reminders.add": "Добавить",
  "reminders.addPlaceholder": "Напомнить о...",
  "reminders.empty": "Нет активных напоминаний",
  "reminders.preset15m": "Через 15 мин",
  "reminders.preset1h": "Через час",
  "reminders.presetTomorrow9": "Завтра в 9:00",
  "reminders.today": "Сегодня",
  "reminders.tomorrow": "Завтра",

  "alert.acknowledge": "Услышал",
  "alert.snooze5": "5 мин",
  "alert.snooze30": "30 мин",
  "alert.snoozeTomorrow": "Завтра",

  "settings.title": "Настройки",
  "settings.theme": "Тема",
  "settings.themeSystem": "Системная",
  "settings.themeLight": "Светлая",
  "settings.themeDark": "Тёмная",
  "settings.liveThemes": "Живые темы",
  "settings.themeAurora": "Аврора",
  "settings.themeSunset": "Закат",
  "settings.themeOcean": "Океан",
  "settings.themeForest": "Лес",
  "settings.autostart": "Автозапуск",
  "settings.autostartLabel": "Запускать вместе с Windows",
  "settings.shortcutLabel": "Хоткей «поверх окон»",
  "settings.shortcutFallback": "— запасной, основной был занят другой программой",
  "settings.language": "Язык",
  "settings.diagnostics": "Диагностика",
  "settings.exportDiagnostics": "Экспортировать диагностику",
  "settings.diagnosticsSaved": "Сохранено в",
  "settings.diagnosticsError": "Не удалось экспортировать диагностику",
  "settings.sync": "Синхронизация",
  "settings.syncGoogleDrive": "Google Drive",
  "settings.syncYandexDisk": "Yandex Disk",
  "settings.syncConnected": "Подключено",
  "settings.syncNotConnected": "Не подключено",
  "settings.syncConnecting": "Подключение...",
  "settings.syncConnect": "Подключить",
  "settings.syncDisconnect": "Отключить",
  "settings.syncError": "Не удалось подключить — проверьте настройку провайдера",
} as const;

const en: Record<keyof typeof ru, string> = {
  "nav.day": "Day",
  "nav.notes": "Notes",
  "nav.reminders": "Reminders",
  "nav.settings": "Settings",

  "header.pinOff": "Remove from front",
  "header.pinOn": "Show on top",
  "header.settings": "Settings",
  "header.close": "Close",

  "profile.newPlaceholder": "New profile...",

  "common.delete": "Delete",
  "common.cancel": "Cancel",

  "notes.empty": "No notes yet",
  "notes.newPlaceholder": "New note...",
  "notes.record": "Record a voice note",
  "notes.stopRecording": "Stop recording",
  "notes.micUnavailable": "Microphone unavailable",

  "day.partial": "Mark partially done",
  "day.resume": "Resume",
  "day.defer": "Defer",
  "day.markUndone": "Mark as not done",
  "day.markDone": "Mark as done",
  "day.changeProgress": "Change progress",
  "day.empty": "Nothing planned for today yet",
  "day.addPlaceholder": "Add a task...",

  "reminders.customTime": "Custom time",
  "reminders.dateTimeLabel": "Reminder date and time",
  "reminders.add": "Add",
  "reminders.addPlaceholder": "Remind me about...",
  "reminders.empty": "No active reminders",
  "reminders.preset15m": "In 15 min",
  "reminders.preset1h": "In 1 hour",
  "reminders.presetTomorrow9": "Tomorrow at 9:00",
  "reminders.today": "Today",
  "reminders.tomorrow": "Tomorrow",

  "alert.acknowledge": "Got it",
  "alert.snooze5": "5 min",
  "alert.snooze30": "30 min",
  "alert.snoozeTomorrow": "Tomorrow",

  "settings.title": "Settings",
  "settings.theme": "Theme",
  "settings.themeSystem": "System",
  "settings.themeLight": "Light",
  "settings.themeDark": "Dark",
  "settings.liveThemes": "Live themes",
  "settings.themeAurora": "Aurora",
  "settings.themeSunset": "Sunset",
  "settings.themeOcean": "Ocean",
  "settings.themeForest": "Forest",
  "settings.autostart": "Autostart",
  "settings.autostartLabel": "Launch with Windows",
  "settings.shortcutLabel": "“Always on top” hotkey",
  "settings.shortcutFallback": "— fallback, the primary was taken by another app",
  "settings.language": "Language",
  "settings.diagnostics": "Diagnostics",
  "settings.exportDiagnostics": "Export diagnostics",
  "settings.diagnosticsSaved": "Saved to",
  "settings.diagnosticsError": "Failed to export diagnostics",
  "settings.sync": "Sync",
  "settings.syncGoogleDrive": "Google Drive",
  "settings.syncYandexDisk": "Yandex Disk",
  "settings.syncConnected": "Connected",
  "settings.syncNotConnected": "Not connected",
  "settings.syncConnecting": "Connecting...",
  "settings.syncConnect": "Connect",
  "settings.syncDisconnect": "Disconnect",
  "settings.syncError": "Failed to connect — check the provider configuration",
};

export type TranslationKey = keyof typeof ru;

const DICTIONARIES: Record<Locale, Record<TranslationKey, string>> = { ru, en };

export const LOCALES: readonly Locale[] = ["ru", "en"];

export const LOCALE_LABELS: Record<Locale, string> = {
  ru: "Русский",
  en: "English",
};

// BCP-47 теги для Intl.DateTimeFormat — держим рядом с остальными locale-
// таблицами, а не в reminderPresets.ts, чтобы новый язык добавлялся в одном месте.
export const INTL_LOCALE_TAG: Record<Locale, string> = {
  ru: "ru-RU",
  en: "en-US",
};

// Обычная функция (не хук) — нужна и вне React-дерева компонентов
// (reminderPresets.ts генерирует подписи пресетов не из компонента).
export function translate(locale: Locale, key: TranslationKey): string {
  return DICTIONARIES[locale][key];
}
