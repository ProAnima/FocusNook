import { invoke } from "@tauri-apps/api/core";

export const legalCommands = {
  async openPrivacy() {
    await invoke("open_privacy_policy");
  },
};
