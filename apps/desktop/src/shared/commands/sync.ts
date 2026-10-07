import { invoke } from "@tauri-apps/api/core";
import type { CloudSyncStatus, ConnectionStatus, SyncProvider, SyncReadinessStatus } from "./types";

export const syncCommands = {
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
  async syncNow(provider: SyncProvider): Promise<CloudSyncStatus> {
    return invoke<CloudSyncStatus>("sync_cloud_now", { provider });
  },
};
