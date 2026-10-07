import { invoke } from "@tauri-apps/api/core";

export const diagnosticsCommands = {
  async export(): Promise<string> {
    return invoke<string>("export_diagnostics");
  },
};
