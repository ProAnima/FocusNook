import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { load, type Store } from "@tauri-apps/plugin-store";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";

export type ThemeMode = "system" | "light" | "dark";

export interface ShortcutInfo {
  shortcut: string;
  isFallback: boolean;
}

export type PlanItemStatus = "open" | "done" | "partial" | "deferred";

export interface PlanItem {
  id: string;
  title: string;
  status: PlanItemStatus;
  progressPercent: number | null;
}

export interface Note {
  id: string;
  title: string | null;
  body: string;
  kind: "text" | "audio" | "transcript" | "audio_with_transcript";
}

export interface Reminder {
  id: string;
  title: string;
  triggerAtUtc: string;
  status: string;
}

let storePromise: Promise<Store> | null = null;

function settingsStore() {
  if (!storePromise) {
    storePromise = load("settings.json", { autoSave: true, defaults: {} });
  }
  return storePromise;
}

// Components call this instead of `invoke`/plugin APIs directly (раздел 12 ТЗ).
export const commands = {
  overlay: {
    // Rust хранит front/back как единственный источник правды и сам решает,
    // на что переключиться — и клик, и глобальный хоткей идут сюда же.
    async toggle(): Promise<boolean> {
      return invoke<boolean>("toggle_overlay_layer");
    },
    onLayerChanged(handler: (front: boolean) => void) {
      return listen<boolean>("layer-changed", (event) => handler(event.payload));
    },
    async getShortcutStatus(): Promise<ShortcutInfo | null> {
      return invoke<ShortcutInfo | null>("get_shortcut_status");
    },
    async close() {
      // Прячет в tray (см. lib.rs::CloseRequested) — реально выходит только
      // пункт трея "Выход".
      await getCurrentWindow().close();
    },
  },
  planItems: {
    async list(): Promise<PlanItem[]> {
      return invoke<PlanItem[]>("list_plan_items");
    },
    async create(title: string): Promise<PlanItem> {
      return invoke<PlanItem>("create_plan_item", { title });
    },
    async toggleDone(id: string): Promise<PlanItem> {
      return invoke<PlanItem>("toggle_plan_item_done", { id });
    },
  },
  notes: {
    async list(): Promise<Note[]> {
      return invoke<Note[]>("list_notes");
    },
    async create(body: string): Promise<Note> {
      return invoke<Note>("create_note", { body });
    },
  },
  reminders: {
    async list(): Promise<Reminder[]> {
      return invoke<Reminder[]>("list_reminders");
    },
    async create(title: string, triggerAtUtc: string): Promise<Reminder> {
      return invoke<Reminder>("create_reminder", { title, triggerAtUtc });
    },
    async getCurrentAlert(): Promise<Reminder | null> {
      return invoke<Reminder | null>("get_current_alert");
    },
    async acknowledge(id: string) {
      await invoke("acknowledge_reminder", { id });
    },
    async snooze(id: string, newTriggerAtUtc: string) {
      await invoke("snooze_reminder", { id, newTriggerAtUtc });
    },
  },
  settings: {
    async getTheme(): Promise<ThemeMode | null> {
      const store = await settingsStore();
      const value = await store.get<ThemeMode>("theme");
      return value ?? null;
    },
    async setTheme(theme: ThemeMode) {
      const store = await settingsStore();
      await store.set("theme", theme);
    },
    async getAutostart(): Promise<boolean> {
      return isEnabled();
    },
    async setAutostart(value: boolean) {
      if (value) {
        await enable();
      } else {
        await disable();
      }
    },
  },
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
