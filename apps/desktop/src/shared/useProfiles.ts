import { useCallback, useEffect, useState } from "react";
import { commands, type Profile } from "./commands";

export function useProfiles() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [activeProfileId, setActiveProfileId] = useState<string | null>(null);

  useEffect(() => {
    commands.profiles
      .list()
      .then((response) => {
        setProfiles(response.profiles);
        setActiveProfileId(response.activeProfileId);
      })
      .catch(() => {
        // Вне Tauri (browser-preview) профили недоступны — переключатель не показываем.
      });
  }, []);

  const createProfile = useCallback(async (displayName: string) => {
    const response = await commands.profiles.create(displayName).catch(() => null);
    if (response) {
      setProfiles(response.profiles);
      setActiveProfileId(response.activeProfileId);
    }
  }, []);

  const switchProfile = useCallback(async (id: string) => {
    const response = await commands.profiles.switchTo(id).catch(() => null);
    if (response) {
      setProfiles(response.profiles);
      setActiveProfileId(response.activeProfileId);
    }
  }, []);

  return { profiles, activeProfileId, createProfile, switchProfile };
}
