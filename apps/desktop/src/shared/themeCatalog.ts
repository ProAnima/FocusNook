import type { ThemeMode } from "./commands";
import type { TranslationKey } from "./translations";

export type ThemeKind = "adaptive" | "static" | "live";

export interface ThemeCatalogItem {
  mode: ThemeMode;
  kind: ThemeKind;
  nameKey: TranslationKey;
  descriptionKey: TranslationKey;
  preview: readonly [string, string, string];
}

export const BASE_THEME_OPTIONS = [
  {
    mode: "system",
    kind: "adaptive",
    nameKey: "settings.themeSystem",
    descriptionKey: "settings.themeSystemDesc",
    preview: ["#f7f8fc", "#171821", "#6f82ff"],
  },
  {
    mode: "light",
    kind: "static",
    nameKey: "settings.themeLight",
    descriptionKey: "settings.themeLightDesc",
    preview: ["#ffffff", "#edf0f7", "#4d65df"],
  },
  {
    mode: "dark",
    kind: "static",
    nameKey: "settings.themeDark",
    descriptionKey: "settings.themeDarkDesc",
    preview: ["#222434", "#10121a", "#8ca0ff"],
  },
] as const satisfies readonly ThemeCatalogItem[];

export const LIVE_THEME_OPTIONS = [
  {
    mode: "aurora",
    kind: "live",
    nameKey: "settings.themeAurora",
    descriptionKey: "settings.themeAuroraDesc",
    preview: ["#8b7dff", "#34e0ba", "#151633"],
  },
  {
    mode: "sunset",
    kind: "live",
    nameKey: "settings.themeSunset",
    descriptionKey: "settings.themeSunsetDesc",
    preview: ["#ff7b6f", "#ffd166", "#221526"],
  },
  {
    mode: "ocean",
    kind: "live",
    nameKey: "settings.themeOcean",
    descriptionKey: "settings.themeOceanDesc",
    preview: ["#31b6d6", "#27e2aa", "#0b1e2d"],
  },
  {
    mode: "forest",
    kind: "live",
    nameKey: "settings.themeForest",
    descriptionKey: "settings.themeForestDesc",
    preview: ["#67c97a", "#d7bb61", "#101b13"],
  },
  {
    mode: "glacier",
    kind: "live",
    nameKey: "settings.themeGlacier",
    descriptionKey: "settings.themeGlacierDesc",
    preview: ["#a8e7ff", "#74a8ff", "#101c2a"],
  },
  {
    mode: "nebula",
    kind: "live",
    nameKey: "settings.themeNebula",
    descriptionKey: "settings.themeNebulaDesc",
    preview: ["#c277ff", "#5fd0ff", "#17142d"],
  },
  {
    mode: "ember",
    kind: "live",
    nameKey: "settings.themeEmber",
    descriptionKey: "settings.themeEmberDesc",
    preview: ["#ff735f", "#ffc247", "#241414"],
  },
  {
    mode: "prism",
    kind: "live",
    nameKey: "settings.themePrism",
    descriptionKey: "settings.themePrismDesc",
    preview: ["#66f0d1", "#ff78c8", "#172034"],
  },
] as const satisfies readonly ThemeCatalogItem[];

export const THEME_OPTIONS = [
  ...BASE_THEME_OPTIONS,
  ...LIVE_THEME_OPTIONS,
] as const satisfies readonly ThemeCatalogItem[];

export type LiveThemeMode = (typeof LIVE_THEME_OPTIONS)[number]["mode"];

export interface LiveThemeShaderConfig {
  colors: readonly [string, string, string];
  speed: number;
  intensity: number;
}

export const LIVE_THEME_MODES = LIVE_THEME_OPTIONS.map((theme) => theme.mode) as readonly LiveThemeMode[];

export const LIVE_THEME_SHADER_CONFIG: Record<LiveThemeMode, LiveThemeShaderConfig> = {
  aurora: {
    colors: ["#8b7dff", "#34e0ba", "#2746c7"],
    speed: 0.74,
    intensity: 0.68,
  },
  sunset: {
    colors: ["#ff7b6f", "#ffd166", "#7f5cff"],
    speed: 0.62,
    intensity: 0.6,
  },
  ocean: {
    colors: ["#31b6d6", "#27e2aa", "#3f73ff"],
    speed: 0.7,
    intensity: 0.62,
  },
  forest: {
    colors: ["#67c97a", "#d7bb61", "#4da3ff"],
    speed: 0.56,
    intensity: 0.58,
  },
  glacier: {
    colors: ["#a8e7ff", "#74a8ff", "#d8fbff"],
    speed: 0.48,
    intensity: 0.55,
  },
  nebula: {
    colors: ["#c277ff", "#5fd0ff", "#ff6fb1"],
    speed: 0.68,
    intensity: 0.64,
  },
  ember: {
    colors: ["#ff735f", "#ffc247", "#9a4dff"],
    speed: 0.58,
    intensity: 0.58,
  },
  prism: {
    colors: ["#66f0d1", "#ff78c8", "#7f8cff"],
    speed: 0.78,
    intensity: 0.6,
  },
};

export function getLiveThemeShader(theme: ThemeMode): LiveThemeShaderConfig | null {
  if (!(LIVE_THEME_MODES as readonly string[]).includes(theme)) return null;
  return LIVE_THEME_SHADER_CONFIG[theme as LiveThemeMode];
}
