import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { currentMonitor, cursorPosition, getCurrentWindow } from "@tauri-apps/api/window";
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
  | "forest"
  | "glacier"
  | "nebula"
  | "ember"
  | "prism";

// Раздел 22 ТЗ: "i18n ru/en минимум, структура под 10 языков" — Locale как
// союз-тип (не string) специально узкий, чтобы TypeScript сам не давал
// пропустить перевод новой строки при добавлении языка (см. shared/translations.ts).
export type Locale = "ru" | "en" | "es" | "de" | "fr" | "pt" | "zh" | "ja" | "ko" | "hi";

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
  planDate: string;
}

export interface Note {
  id: string;
  title: string | null;
  body: string;
  kind: "text" | "audio" | "transcript" | "audio_with_transcript";
  audioPath: string | null;
  groupId: string | null;
}

export interface NoteGroup {
  id: string;
  name: string;
}

export interface Reminder {
  id: string;
  title: string;
  audioPath: string | null;
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

export interface CursorClientPosition {
  x: number;
  y: number;
}

// Раздел 14 ТЗ, sync — только аутентификация в этом шаге (см. oauth.rs).
export type SyncProvider = "google_drive" | "yandex_disk"; export type NoteFolderSort = "recent" | "name"; export type FolderRailSide = "left" | "right";

export interface ConnectionStatus {
  connected: boolean;
}

export interface ServerSyncStatus {
  available: boolean;
  connected: boolean;
  endpoint: string | null;
  message: string | null;
}

export interface SyncReadinessStatus {
  profileIdHash: string;
  deviceIdHash: string | null;
  operationCount: number;
  lastOperationAt: string | null;
  lastOperationHlc: string | null;
}

let storePromise: Promise<Store> | null = null;

function settingsStore() {
  if (!storePromise) {
    storePromise = load("settings.json", { autoSave: true, defaults: {} });
  }
  return storePromise;
}

async function resolveFolderRailSide(positionX?: number): Promise<FolderRailSide> {
  try {
    const win = getCurrentWindow();
    const [position, size, monitor] = await Promise.all([
      positionX === undefined ? win.outerPosition() : Promise.resolve({ x: positionX }),
      win.outerSize(),
      currentMonitor(),
    ]);
    const workArea = monitor?.workArea;
    if (!workArea) return "left";
    const windowCenter = position.x + size.width / 2;
    const monitorCenter = workArea.position.x + workArea.size.width / 2;
    return windowCenter < monitorCenter ? "right" : "left";
  } catch {
    return "left";
  }
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
    getFolderRailSide: resolveFolderRailSide,
    async onFolderRailSideChanged(handler: (side: FolderRailSide) => void) {
      try {
        return await getCurrentWindow().onMoved(({ payload }) => void resolveFolderRailSide(payload.x).then(handler));
      } catch {
        return () => {};
      }
    },
    async getCursorClientPosition(): Promise<CursorClientPosition> {
      const win = getCurrentWindow();
      const [cursor, position, scaleFactor] = await Promise.all([
        cursorPosition(),
        win.outerPosition(),
        win.scaleFactor(),
      ]);
      return {
        x: (cursor.x - position.x) / scaleFactor,
        y: (cursor.y - position.y) / scaleFactor,
      };
    },
    async setIgnoreCursorEvents(ignore: boolean): Promise<void> {
      await getCurrentWindow().setIgnoreCursorEvents(ignore);
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
    async list(planDate: string): Promise<PlanItem[]> {
      return invoke<PlanItem[]>("list_plan_items", { planDate });
    },
    async listRange(startDate: string, endDate: string): Promise<PlanItem[]> {
      return invoke<PlanItem[]>("list_plan_items_range", { startDate, endDate });
    },
    async create(title: string, planDate: string): Promise<PlanItem> {
      return invoke<PlanItem>("create_plan_item", { title, planDate });
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
    async moveToDate(id: string, planDate: string): Promise<PlanItem> {
      return invoke<PlanItem>("move_plan_item_to_date", { id, planDate });
    },
    async rollOverPending(targetDate: string): Promise<number> {
      return invoke<number>("roll_over_pending_plan_items", { targetDate });
    },
    async delete(id: string) {
      await invoke("delete_plan_item", { id });
    },
  },
  notes: {
    async list(): Promise<Note[]> {
      return invoke<Note[]>("list_notes");
    },
    async listGroups(): Promise<NoteGroup[]> {
      return invoke<NoteGroup[]>("list_note_groups");
    },
    async createGroup(name: string): Promise<NoteGroup> {
      return invoke<NoteGroup>("create_note_group", { name });
    },
    async create(body: string, groupId: string | null): Promise<Note> {
      return invoke<Note>("create_note", { body, groupId });
    },
    async createAudio(audioBase64: string, groupId: string | null): Promise<Note> {
      return invoke<Note>("create_audio_note", { audioBase64, groupId });
    },
    async getAudio(id: string): Promise<string> {
      return invoke<string>("get_note_audio", { id });
    },
    async moveToGroup(id: string, groupId: string | null): Promise<Note> {
      return invoke<Note>("move_note_to_group", { id, groupId });
    },
    async update(id: string, body: string): Promise<Note> {
      return invoke<Note>("update_note", { id, body });
    },
    async delete(id: string) {
      await invoke("delete_note", { id });
    },
  },
  reminders: {
    onChanged(handler: () => void) {
      return listen("reminders-changed", handler);
    },
    async list(): Promise<Reminder[]> {
      return invoke<Reminder[]>("list_reminders");
    },
    async create(title: string, triggerAtUtc: string): Promise<Reminder> {
      return invoke<Reminder>("create_reminder", { title, triggerAtUtc });
    },
    async createAudio(title: string, triggerAtUtc: string, audioBase64: string): Promise<Reminder> {
      return invoke<Reminder>("create_audio_reminder", { request: { title, triggerAtUtc, audioBase64 } });
    },
    async getCurrentAlert(): Promise<Reminder | null> {
      return invoke<Reminder | null>("get_current_alert");
    },
    async getAudio(id: string): Promise<string> {
      return invoke<string>("get_reminder_audio", { id });
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
    async getMicrophoneDeviceId(): Promise<string | null> {
      const store = await settingsStore();
      const value = await store.get<string>("microphoneDeviceId");
      return value ?? null;
    },
    async setMicrophoneDeviceId(deviceId: string | null) {
      const store = await settingsStore();
      await store.set("microphoneDeviceId", deviceId);
    },
    async getNoteFolderSort(): Promise<NoteFolderSort> {
      const store = await settingsStore();
      const value = await store.get<NoteFolderSort>("noteFolderSort");
      return value === "name" ? "name" : "recent";
    },
    async setNoteFolderSort(value: NoteFolderSort) {
      const store = await settingsStore();
      await store.set("noteFolderSort", value);
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
    async readiness(): Promise<SyncReadinessStatus> {
      return invoke<SyncReadinessStatus>("sync_readiness_status");
    },
    async status(provider: SyncProvider): Promise<ConnectionStatus> {
      return invoke<ConnectionStatus>("connection_status", { provider });
    },
    async disconnect(provider: SyncProvider) {
      await invoke("disconnect_provider", { provider });
    },
  },
  serverSync: {
    async status(): Promise<ServerSyncStatus> {
      return invoke<ServerSyncStatus>("server_sync_status");
    },
    async connectDefault(): Promise<ServerSyncStatus> {
      return invoke<ServerSyncStatus>("connect_default_server_sync");
    },
    async connect(endpoint: string, token: string): Promise<ServerSyncStatus> {
      return invoke<ServerSyncStatus>("connect_server_sync", { endpoint, token });
    },
    async disconnect() {
      await invoke("disconnect_server_sync");
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
