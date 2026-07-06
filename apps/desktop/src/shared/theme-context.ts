import { createContext } from "react";
import type { ThemeMode } from "./commands";
import { LIVE_THEME_MODES, type LiveThemeMode } from "./themeCatalog";

export type { ThemeMode };

// "system" резолвится в light/dark на основе ОС — во всех остальных случаях
// effective совпадает с mode один-в-один.
export type ResolvedTheme = Exclude<ThemeMode, "system">;

export const LIVE_THEMES: readonly LiveThemeMode[] = LIVE_THEME_MODES;

export function isLiveTheme(theme: ResolvedTheme): theme is LiveThemeMode {
  return (LIVE_THEMES as readonly string[]).includes(theme);
}

export type ThemeContextValue = {
  mode: ThemeMode;
  effective: ResolvedTheme;
  setMode: (mode: ThemeMode) => void;
};

export const ThemeContext = createContext<ThemeContextValue | null>(null);
