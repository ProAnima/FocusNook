import { useCallback, useEffect, useState } from "react";
import { commands, type Profile, type ProfilesResponse } from "./commands";

function browserPreviewAccounts(): ProfilesResponse {
  const mode = new URLSearchParams(window.location.search).get("accountPreview");
  if (mode === "setup") {
    return {
      profiles: [{ id: "legacy", displayName: "Профиль", avatarColor: "#f2b463", email: null, accountConfigured: false, syncEnabled: false }],
      activeProfileId: "legacy",
      sessionLocked: false,
    };
  }
  if (mode === "locked") {
    return {
      profiles: [
        { id: "one", displayName: "Ян", avatarColor: "#f2b463", email: "i@example.test", accountConfigured: true, syncEnabled: true },
        { id: "two", displayName: "Вита", avatarColor: "#7cb9e8", email: "vita@example.test", accountConfigured: true, syncEnabled: false },
      ],
      activeProfileId: "one",
      sessionLocked: true,
    };
  }
  return {
    profiles: [{
      id: "preview-account",
      displayName: "Preview",
      avatarColor: "#f2b463",
      email: "preview@example.test",
      accountConfigured: true,
      syncEnabled: false,
    }],
    activeProfileId: "preview-account",
    sessionLocked: false,
  };
}

export function useProfiles() {
  const [response, setResponse] = useState<ProfilesResponse | null>(null);

  useEffect(() => {
    commands.profiles.list().then(setResponse).catch(() => setResponse(browserPreviewAccounts()));
  }, []);

  const configureAccount = useCallback(async (displayName: string, email: string, password: string) => {
    setResponse(await commands.profiles.configure(displayName, email, password));
  }, []);

  const createAccount = useCallback(async (displayName: string, email: string, password: string) => {
    setResponse(await commands.profiles.create(displayName, email, password));
  }, []);

  const switchAccount = useCallback(async (id: string, password: string) => {
    setResponse(await commands.profiles.switchTo(id, password));
  }, []);

  const logout = useCallback(async () => {
    setResponse(await commands.profiles.logout());
  }, []);

  const profiles: Profile[] = response?.profiles ?? [];
  return {
    profiles,
    activeProfileId: response?.activeProfileId ?? null,
    activeProfile: profiles.find((profile) => profile.id === response?.activeProfileId) ?? null,
    sessionLocked: response?.sessionLocked ?? false,
    loading: response === null,
    configureAccount,
    createAccount,
    switchAccount,
    logout,
  };
}
