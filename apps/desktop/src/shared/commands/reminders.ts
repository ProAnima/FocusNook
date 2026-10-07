import { invoke } from "@tauri-apps/api/core";
import { safeListen } from "./safeListen";
import type { Reminder } from "./types";

export const remindersCommands = {
  onChanged(handler: () => void) {
    return safeListen("reminders-changed", handler);
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
};
