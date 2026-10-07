import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { load, type Store } from "@tauri-apps/plugin-store";
import type { Locale, NoteFolderSort, ThemeMode } from "./types";

let storePromise: Promise<Store> | null = null;

function settingsStore() {
  if (!storePromise) {
    storePromise = load("settings.json", { autoSave: true, defaults: {} });
  }
  return storePromise;
}

export const settingsCommands = {
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
};
