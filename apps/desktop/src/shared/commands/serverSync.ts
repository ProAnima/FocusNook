import { invoke } from "@tauri-apps/api/core";
import { safeListen } from "./safeListen";
import type { ServerSyncStatus } from "./types";

export const serverSyncCommands = {
  onCompleted(handler: () => void) {
    return safeListen("server-sync-completed", handler);
  },
  onFailed(handler: (message: string) => void) {
    return safeListen<string>("server-sync-failed", (event) => handler(event.payload));
  },
  async status(): Promise<ServerSyncStatus> {
    return invoke<ServerSyncStatus>("server_sync_status");
  },
  async syncNow(): Promise<ServerSyncStatus> {
    return invoke<ServerSyncStatus>("sync_server_now");
  },
  async request(): Promise<void> {
    await invoke("request_server_sync");
  },
  async setEnabled(enabled: boolean, password = "", privacyAccepted = false): Promise<ServerSyncStatus> {
    return invoke<ServerSyncStatus>("set_account_sync_enabled", { enabled, password, privacyAccepted });
  },
  async connectDefault(): Promise<ServerSyncStatus> {
    return invoke<ServerSyncStatus>("connect_default_server_sync");
  },
  async register(email: string, password: string, displayName: string, privacyAccepted: boolean): Promise<ServerSyncStatus> {
    return invoke<ServerSyncStatus>("register_server_account", {
      request: { email, password, displayName, privacyAccepted },
    });
  },
  async login(email: string, password: string): Promise<ServerSyncStatus> {
    return invoke<ServerSyncStatus>("login_server_account", { email, password });
  },
  async connect(endpoint: string, token: string): Promise<ServerSyncStatus> {
    return invoke<ServerSyncStatus>("connect_server_sync", { endpoint, token });
  },
  async disconnect() {
    await invoke("disconnect_server_sync");
  },
  async deleteAccount(password: string) {
    await invoke("delete_server_account", { password });
  },
};
