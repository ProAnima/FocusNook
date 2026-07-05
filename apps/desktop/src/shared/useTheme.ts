import { useContext } from "react";
import { ThemeContext, type ThemeMode } from "./theme-context";

export type { ThemeMode };

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}
