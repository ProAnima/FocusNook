import { useEffect, useState, type ReactNode } from "react";
import { commands, type ThemeMode } from "./commands";
import { ThemeContext, type ResolvedTheme } from "./theme-context";
import { useSystemPrefersDark } from "./useSystemTheme";

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [mode, setModeState] = useState<ThemeMode>("system");
  const [ready, setReady] = useState(false);
  const systemDark = useSystemPrefersDark();

  useEffect(() => {
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
  }, []);

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
