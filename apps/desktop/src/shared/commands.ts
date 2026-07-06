import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { load, type Store } from "@tauri-apps/plugin-store";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";

// aurora/sunset/ocean/forest — "живые" темы с анимированным фоном (см.
// theme.css и useLiveBackgroundPointer), а не просто другая палитра.
export type ThemeMode =
  | "system"
  | "light"
  | "dark"
  | "aurora"
  | "sunset"
  | "ocean"
  | "forest";

// Раздел 22 ТЗ: "i18n ru/en минимум, структура под 10 языков" — Locale как
// союз-тип (не string) специально узкий, чтобы TypeScript сам не давал
// пропустить перевод новой строки при добавлении языка (см. shared/translations.ts).
export type Locale = "ru" | "en";

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
  audioPath: string | null;
}

export interface Reminder {
  id: string;
  title: string;
  triggerAtUtc: string;
  status: string;
}

export interface Profile {
  id: string;
  displayName: string;
  avatarColor: string;
}

export interface ProfilesResponse {
  profiles: Profile[];
  activeProfileId: string;
}

// Раздел 14 ТЗ, sync — только аутентификация в этом шаге (см. oauth.rs).
export type SyncProvider = "google_drive" | "yandex_disk";

export interface ConnectionStatus {
  connected: boolean;
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
    async isDesktop(): Promise<boolean> {
      return invoke<boolean>("is_desktop_platform");
    },
    async close() {
      // Прячет в tray (см. lib.rs::CloseRequested) — реально выходит только
      // пункт трея "Выход".
      await getCurrentWindow().close();
    },
  },
  profiles: {
    async list(): Promise<ProfilesResponse> {
      return invoke<ProfilesResponse>("list_profiles");
    },
    async create(displayName: string): Promise<ProfilesResponse> {
      return invoke<ProfilesResponse>("create_profile", { displayName });
    },
    async switchTo(id: string): Promise<ProfilesResponse> {
      return invoke<ProfilesResponse>("switch_active_profile", { id });
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
    async cycleProgress(id: string): Promise<PlanItem> {
      return invoke<PlanItem>("cycle_plan_item_progress", { id });
    },
    async toggleDeferred(id: string): Promise<PlanItem> {
      return invoke<PlanItem>("toggle_plan_item_deferred", { id });
    },
    async delete(id: string) {
      await invoke("delete_plan_item", { id });
    },
  },
  notes: {
    async list(): Promise<Note[]> {
      return invoke<Note[]>("list_notes");
    },
    async create(body: string): Promise<Note> {
      return invoke<Note>("create_note", { body });
    },
    async createAudio(audioBase64: string): Promise<Note> {
      return invoke<Note>("create_audio_note", { audioBase64 });
    },
    async getAudio(id: string): Promise<string> {
      return invoke<string>("get_note_audio", { id });
    },
    async delete(id: string) {
      await invoke("delete_note", { id });
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
    async delete(id: string) {
      await invoke("delete_reminder", { id });
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
    async getLocale(): Promise<Locale | null> {
      const store = await settingsStore();
      const value = await store.get<Locale>("locale");
      return value ?? null;
    },
    async setLocale(locale: Locale) {
      const store = await settingsStore();
      await store.set("locale", locale);
    },
  },
  diagnostics: {
    async export(): Promise<string> {
      return invoke<string>("export_diagnostics");
    },
  },
  sync: {
    async start(provider: SyncProvider) {
      await invoke("start_provider_auth", { provider });
    },
    async status(provider: SyncProvider): Promise<ConnectionStatus> {
      return invoke<ConnectionStatus>("connection_status", { provider });
    },
    async disconnect(provider: SyncProvider) {
      await invoke("disconnect_provider", { provider });
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
