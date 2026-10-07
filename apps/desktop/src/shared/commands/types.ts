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

export type ResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

// Раздел 22 ТЗ: "i18n ru/en минимум, структура под 10 языков" — Locale как
// союз-тип (не string) специально узкий, чтобы TypeScript сам не давал
// пропустить перевод новой строки при добавлении языка (см. shared/translations/).
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
  isLongRunning: boolean;
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
  email: string | null;
  accountConfigured: boolean;
  syncEnabled: boolean;
}

export interface ProfilesResponse {
  profiles: Profile[];
  activeProfileId: string;
  sessionLocked: boolean;
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

export interface CloudSyncStatus {
  connected: boolean;
  lastOperationHlc: string | null;
  message: string | null;
  provider: SyncProvider;
}

export interface ServerSyncStatus {
  available: boolean;
  accountEmail: string | null;
  accountUserId: string | null;
  connected: boolean;
  displayName: string | null;
  endpoint: string | null;
  mediaReady: boolean;
  message: string | null;
}

export interface SyncReadinessStatus {
  profileIdHash: string;
  deviceIdHash: string | null;
  operationCount: number;
  lastOperationAt: string | null;
  lastOperationHlc: string | null;
}
