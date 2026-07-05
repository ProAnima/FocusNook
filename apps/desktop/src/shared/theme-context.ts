import { createContext } from "react";
import type { ThemeMode } from "./commands";

export type { ThemeMode };

export type ThemeContextValue = {
  mode: ThemeMode;
  effective: "light" | "dark";
  setMode: (mode: ThemeMode) => void;
};

export const ThemeContext = createContext<ThemeContextValue | null>(null);
