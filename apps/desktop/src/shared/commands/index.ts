import { getCurrentWindow } from "@tauri-apps/api/window";
import { overlayCommands } from "./overlay";
import { profilesCommands } from "./profiles";
import { planItemsCommands } from "./planItems";
import { notesCommands } from "./notes";
import { remindersCommands } from "./reminders";
import { settingsCommands } from "./settings";
import { diagnosticsCommands } from "./diagnostics";
import { syncCommands } from "./sync";
import { serverSyncCommands } from "./serverSync";
import { legalCommands } from "./legal";

export * from "./types";

// Components call this instead of `invoke`/plugin APIs directly (раздел 12 ТЗ).
export const commands = {
  overlay: overlayCommands,
  profiles: profilesCommands,
  planItems: planItemsCommands,
  notes: notesCommands,
  reminders: remindersCommands,
  settings: settingsCommands,
  diagnostics: diagnosticsCommands,
  sync: syncCommands,
  serverSync: serverSyncCommands,
  legal: legalCommands,
};

// Одна фронтенд-сборка обслуживает и main, и reminder-alert окна (раздел 10
// ТЗ) — App.tsx решает, что рендерить, по label текущего окна.
export function isAlertWindow(): boolean {
  try {
    return getCurrentWindow().label === "reminder-alert";
  } catch {
    // Вне Tauri (browser-preview, тесты) метаданных окна нет — считаем
    // главным окном, чтобы не падать в пустой экран.
    return false;
  }
}
