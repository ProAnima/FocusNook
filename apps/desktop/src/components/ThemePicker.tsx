import { useState, type CSSProperties } from "react";
import {
  Check,
  ChevronDown,
  Flame,
  Leaf,
  Monitor,
  Moon,
  Palette,
  Snowflake,
  Sparkles,
  Sun,
  Waves,
  type LucideIcon,
} from "lucide-react";
import type { ThemeMode } from "../shared/useTheme";
import { THEME_OPTIONS, type ThemeCatalogItem } from "../shared/themeCatalog";
import { useLocale } from "../shared/useLocale";

const THEME_ICONS: Record<ThemeMode, LucideIcon> = {
  system: Monitor,
  light: Sun,
  dark: Moon,
  aurora: Sparkles,
  sunset: Sun,
  ocean: Waves,
  forest: Leaf,
  glacier: Snowflake,
  nebula: Sparkles,
  ember: Flame,
  prism: Palette,
};

type ThemePreviewStyle = CSSProperties & {
  "--theme-preview-a": string;
  "--theme-preview-b": string;
  "--theme-preview-c": string;
};

function themePreviewStyle(option: ThemeCatalogItem): ThemePreviewStyle {
  const [a, b, c] = option.preview;
  return {
    "--theme-preview-a": a,
    "--theme-preview-b": b,
    "--theme-preview-c": c,
  };
}

function themeKindLabel(option: ThemeCatalogItem, t: ReturnType<typeof useLocale>["t"]) {
  if (option.kind === "live") return t("settings.themeKindLive");
  if (option.kind === "adaptive") return t("settings.themeKindAuto");
  return t("settings.themeKindStatic");
}

function ThemePreview({ option }: { option: ThemeCatalogItem }) {
  return (
    <span
      className={`theme-preview ${option.kind === "live" ? "is-live" : ""}`}
      style={themePreviewStyle(option)}
      aria-hidden="true"
    >
      {option.kind === "live" && <span className="theme-preview-shine" />}
    </span>
  );
}

function ThemeCopy({ option }: { option: ThemeCatalogItem }) {
  const { t } = useLocale();
  const Icon = THEME_ICONS[option.mode];

  return (
    <span className="theme-row-copy">
      <span className="theme-row-title">
        <Icon size={14} />
        <span>{t(option.nameKey)}</span>
      </span>
      <span className="theme-row-description">{t(option.descriptionKey)}</span>
    </span>
  );
}

function ThemeRow({
  option,
  isActive,
  onSelect,
}: {
  option: ThemeCatalogItem;
  isActive: boolean;
  onSelect: (mode: ThemeMode) => void;
}) {
  const { t } = useLocale();

  return (
    <button
      className={`theme-row ${isActive ? "is-active" : ""} ${option.kind === "live" ? "is-live" : ""}`}
      onClick={() => onSelect(option.mode)}
      aria-pressed={isActive}
    >
      <ThemePreview option={option} />
      <ThemeCopy option={option} />
      <span className={`theme-row-kind ${option.kind === "live" ? "is-live" : ""}`}>
        {themeKindLabel(option, t)}
      </span>
      <span className="theme-row-check" aria-hidden="true">
        {isActive && <Check size={13} />}
      </span>
    </button>
  );
}

function ThemeSummary({
  option,
  expanded,
  onToggle,
}: {
  option: ThemeCatalogItem;
  expanded: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      className={`theme-summary ${expanded ? "is-expanded" : ""}`}
      onClick={onToggle}
      aria-expanded={expanded}
      aria-controls="theme-options-list"
    >
      <ThemePreview option={option} />
      <ThemeCopy option={option} />
      <ChevronDown className="theme-summary-chevron" size={15} aria-hidden="true" />
    </button>
  );
}

export function ThemePicker({
  mode,
  setMode,
}: {
  mode: ThemeMode;
  setMode: (mode: ThemeMode) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const currentTheme = THEME_OPTIONS.find((option) => option.mode === mode) ?? THEME_OPTIONS[0];

  function selectTheme(next: ThemeMode) {
    setMode(next);
    setExpanded(false);
  }

  return (
    <>
      <ThemeSummary
        option={currentTheme}
        expanded={expanded}
        onToggle={() => setExpanded((value) => !value)}
      />
      {expanded && (
        <div className="theme-list" id="theme-options-list">
          {THEME_OPTIONS.filter((option) => option.mode !== mode).map((option) => (
            <ThemeRow
              key={option.mode}
              option={option}
              isActive={false}
              onSelect={selectTheme}
            />
          ))}
        </div>
      )}
    </>
  );
}
