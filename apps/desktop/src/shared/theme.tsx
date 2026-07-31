import { useEffect, useState, type ReactNode } from "react";
import { commands, type ThemeMode } from "./commands";
import { ThemeContext, type ResolvedTheme } from "./theme-context";
import { useSystemPrefersDark } from "./useSystemTheme";

const PREVIEW_THEMES: readonly ThemeMode[] = ["light", "dark", "aurora", "sunset", "ocean", "forest", "glacier", "nebula", "ember", "prism"];

function previewTheme(): ThemeMode | null {
  if (!import.meta.env.DEV) return null;
  const value = new URLSearchParams(window.location.search).get("themePreview");
  return PREVIEW_THEMES.find((theme) => theme === value) ?? null;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const preview = previewTheme();
  const [mode, setModeState] = useState<ThemeMode>(preview ?? "system");
  const [ready, setReady] = useState(Boolean(preview));
  const systemDark = useSystemPrefersDark();

  useEffect(() => {
    if (preview) return;
    let cancelled = false;
    commands.settings
      .getTheme()
      .then((saved) => {
        if (cancelled) return;
        if (saved) setModeState(saved);
      })
      .catch(() => {
        // Нет доступа к Tauri store (например, окно открыто вне Tauri) — остаёмся на "system".
      })
      .finally(() => {
        if (!cancelled) setReady(true);
      });
    return () => {
      cancelled = true;
    };
  }, [preview]);

  const effective: ResolvedTheme = mode === "system" ? (systemDark ? "dark" : "light") : mode;

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", effective);
  }, [effective]);

  function setMode(next: ThemeMode) {
    setModeState(next);
    void commands.settings.setTheme(next);
  }

  // Ждём сохранённую тему перед первым рендером, чтобы не мигнуть чужой темой.
  if (!ready) return null;

  return (
    <ThemeContext.Provider value={{ mode, effective, setMode }}>
      {children}
    </ThemeContext.Provider>
  );
}
