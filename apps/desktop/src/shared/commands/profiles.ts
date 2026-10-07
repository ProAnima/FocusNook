import { invoke } from "@tauri-apps/api/core";
import type { ProfilesResponse } from "./types";

export const profilesCommands = {
  async list(): Promise<ProfilesResponse> {
    return invoke<ProfilesResponse>("list_profiles");
  },
  async create(displayName: string, email: string, password: string): Promise<ProfilesResponse> {
    return invoke<ProfilesResponse>("create_profile", { displayName, email, password });
  },
  async configure(displayName: string, email: string, password: string): Promise<ProfilesResponse> {
    return invoke<ProfilesResponse>("configure_active_account", { displayName, email, password });
  },
  async switchTo(id: string, password: string): Promise<ProfilesResponse> {
    return invoke<ProfilesResponse>("switch_active_profile", { id, password });
  },
  async logout(): Promise<ProfilesResponse> {
    return invoke<ProfilesResponse>("logout_account");
  },
};
