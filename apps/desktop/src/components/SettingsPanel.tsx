import { useCallback, useEffect, useState } from "react";
import { Monitor, Moon, Sun, X } from "lucide-react";
import { useTheme, type ThemeMode } from "../shared/useTheme";
import { commands, type SyncProvider } from "../shared/commands";
import { LOCALES, LOCALE_LABELS } from "../shared/translations";
import { useLocale } from "../shared/useLocale";
import type { ShortcutInfo } from "../shared/useLayerToggle";

function useThemeOptions(): { mode: ThemeMode; label: string; icon: typeof Sun }[] {
  const { t } = useLocale();
  return [
    { mode: "system", label: t("settings.themeSystem"), icon: Monitor },
    { mode: "light", label: t("settings.themeLight"), icon: Sun },
    { mode: "dark", label: t("settings.themeDark"), icon: Moon },
  ];
}

function ThemeSection({
  mode,
  setMode,
}: {
  mode: ThemeMode;
  setMode: (mode: ThemeMode) => void;
}) {
  const { t } = useLocale();
  const themeOptions = useThemeOptions();
  return (
    <div className="settings-group">
      <span className="settings-group-label">{t("settings.theme")}</span>
      <div className="theme-options">
        {themeOptions.map(({ mode: optionMode, label, icon: Icon }) => (
          <button
            key={optionMode}
            className={`theme-option ${mode === optionMode ? "is-active" : ""}`}
            onClick={() => setMode(optionMode)}
          >
            <Icon size={15} />
            <span>{label}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

// Превью-градиенты продублированы из theme.css (--bg-blob-a/b каждой темы) —
// CSS-переменные живой темы недоступны, пока она не активна как data-theme,
// а показать превью нужно именно для НЕ выбранных сейчас тем.
const LIVE_THEME_PREVIEWS: Record<string, [string, string]> = {
  aurora: ["#6d5bff", "#2fd6c0"],
  sunset: ["#ff7a59", "#ffc857"],
  ocean: ["#2ea8c9", "#1fe0a8"],
  forest: ["#4f9e5c", "#c9a24a"],
};

function useLiveThemeOptions(): { mode: ThemeMode; label: string }[] {
  const { t } = useLocale();
  return [
    { mode: "aurora", label: t("settings.themeAurora") },
    { mode: "sunset", label: t("settings.themeSunset") },
    { mode: "ocean", label: t("settings.themeOcean") },
    { mode: "forest", label: t("settings.themeForest") },
  ];
}

// Раздел ...: несколько "живых" тем с ненавязчивым анимированным фоном
// (дрейфующие размытые пятна, лёгкий параллакс за курсором — см. App.css
// .overlay-shell::before/::after и useLiveBackgroundPointer.ts). Отдельная
// группа, а не часть ThemeSection — это не режим "светлая/тёмная", а
// самостоятельный визуальный стиль поверх той же самой data-theme-схемы.
function LiveThemeSection({
  mode,
  setMode,
}: {
  mode: ThemeMode;
  setMode: (mode: ThemeMode) => void;
}) {
  const { t } = useLocale();
  const options = useLiveThemeOptions();
  return (
    <div className="settings-group">
      <span className="settings-group-label">{t("settings.liveThemes")}</span>
      <div className="live-theme-options">
        {options.map(({ mode: optionMode, label }) => {
          const [from, to] = LIVE_THEME_PREVIEWS[optionMode];
          return (
            <button
              key={optionMode}
              className={`live-theme-option ${mode === optionMode ? "is-active" : ""}`}
              onClick={() => setMode(optionMode)}
            >
              <span
                className="live-theme-swatch"
                style={{ backgroundImage: `linear-gradient(135deg, ${from}, ${to})` }}
                aria-hidden="true"
              />
              <span>{label}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

function LanguageSection() {
  const { locale, setLocale, t } = useLocale();
  return (
    <div className="settings-group">
      <span className="settings-group-label">{t("settings.language")}</span>
      <div className="theme-options">
        {LOCALES.map((option) => (
          <button
            key={option}
            className={`theme-option ${locale === option ? "is-active" : ""}`}
            onClick={() => setLocale(option)}
          >
            <span>{LOCALE_LABELS[option]}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

function AutostartSection() {
  const [autostart, setAutostart] = useState(false);
  const { t } = useLocale();

  useEffect(() => {
    commands.settings
      .getAutostart()
      .then(setAutostart)
      .catch(() => {
        // Вне Tauri автостарт недоступен — оставляем выключенным.
      });
  }, []);

  async function toggle() {
    const next = !autostart;
    setAutostart(next);
    try {
      await commands.settings.setAutostart(next);
    } catch {
      setAutostart(!next);
    }
  }

  return (
    <div className="settings-group">
      <span className="settings-group-label">{t("settings.autostart")}</span>
      <button className="toggle-row" onClick={toggle}>
        <span>{t("settings.autostartLabel")}</span>
        <span className={`toggle-switch ${autostart ? "is-on" : ""}`} />
      </button>
    </div>
  );
}

function DiagnosticsSection() {
  const { t } = useLocale();
  const [savedPath, setSavedPath] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  async function handleExport() {
    setFailed(false);
    setSavedPath(null);
    try {
      setSavedPath(await commands.diagnostics.export());
    } catch {
      setFailed(true);
    }
  }

  return (
    <div className="settings-group">
      <span className="settings-group-label">{t("settings.diagnostics")}</span>
      <button className="preset-button" onClick={() => void handleExport()}>
        {t("settings.exportDiagnostics")}
      </button>
      {savedPath && (
        <p className="settings-hint">
          {t("settings.diagnosticsSaved")}: {savedPath}
        </p>
      )}
      {failed && <p className="note-error">{t("settings.diagnosticsError")}</p>}
    </div>
  );
}

function useConnectionStatus(provider: SyncProvider) {
  const [connected, setConnected] = useState(false);
  const refresh = useCallback(() => {
    commands.sync
      .status(provider)
      .then((status) => setConnected(status.connected))
      .catch(() => setConnected(false));
  }, [provider]);
  useEffect(() => refresh(), [refresh]);
  return { connected, refresh };
}

// "Подключено" здесь значит "есть сохранённый refresh-токен" (см.
// sync.rs::connection_status) — не подтверждённая живая проверка. Кнопка не
// делает различий между "провайдер не настроен в sync_providers.json" и
// "OAuth реально не прошёл" — оба случая ведут на один и тот же settings.syncError,
// осознанно минимальный UI для этого шага (только доказать, что флоу работает).
function SyncProviderRow({ provider, label }: { provider: SyncProvider; label: string }) {
  const { t } = useLocale();
  const { connected, refresh } = useConnectionStatus(provider);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);

  async function handleClick() {
    setError(false);
    setBusy(true);
    try {
      if (connected) {
        await commands.sync.disconnect(provider);
      } else {
        await commands.sync.start(provider);
      }
      refresh();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="sync-provider-row">
      <div className="sync-provider-info">
        <span>{label}</span>
        <span className="settings-hint">
          {connected ? t("settings.syncConnected") : t("settings.syncNotConnected")}
        </span>
      </div>
      <button className="preset-button" onClick={() => void handleClick()} disabled={busy}>
        {busy ? t("settings.syncConnecting") : connected ? t("settings.syncDisconnect") : t("settings.syncConnect")}
      </button>
      {error && <p className="note-error">{t("settings.syncError")}</p>}
    </div>
  );
}

function SyncSection() {
  const { t } = useLocale();
  return (
    <div className="settings-group">
      <span className="settings-group-label">{t("settings.sync")}</span>
      <SyncProviderRow provider="google_drive" label={t("settings.syncGoogleDrive")} />
      <SyncProviderRow provider="yandex_disk" label={t("settings.syncYandexDisk")} />
    </div>
  );
}

function ShortcutSection({ info }: { info: ShortcutInfo }) {
  const { t } = useLocale();
  return (
    <div className="settings-group">
      <span className="settings-group-label">{t("settings.shortcutLabel")}</span>
      <p className="settings-hint">
        {info.shortcut.replace(/\+/g, " + ").toUpperCase()}
        {info.isFallback && ` ${t("settings.shortcutFallback")}`}
      </p>
    </div>
  );
}

export function SettingsPanel({
  shortcutInfo,
  onClose,
  isDesktop,
}: {
  shortcutInfo: ShortcutInfo | null;
  onClose: () => void;
  isDesktop: boolean;
}) {
  const { mode, setMode } = useTheme();
  const { t } = useLocale();

  return (
    <div className="settings-panel">
      <div className="settings-header">
        <span>{t("settings.title")}</span>
        <button className="icon-button" onClick={onClose} title={t("header.close")} aria-label={t("header.close")}>
          <X size={14} />
        </button>
      </div>

      <ThemeSection mode={mode} setMode={setMode} />
      <LiveThemeSection mode={mode} setMode={setMode} />
      <LanguageSection />
      {/* "Запускать вместе с Windows" не имеет смысла на телефоне — раздел 11 ТЗ. */}
      {isDesktop && <AutostartSection />}
      {shortcutInfo && <ShortcutSection info={shortcutInfo} />}
      <SyncSection />
      <DiagnosticsSection />
    </div>
  );
}
