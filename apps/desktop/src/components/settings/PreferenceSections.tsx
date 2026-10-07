import { useEffect, useState } from "react";
import { Mic, RefreshCw } from "lucide-react";
import { commands, type Locale, type NoteFolderSort } from "../../shared/commands";
import { LOCALES, LOCALE_LABELS } from "../../shared/translations";
import { useLocale } from "../../shared/useLocale";
import { useMicrophoneSettings } from "../../shared/useMicrophoneSettings";
import type { ShortcutInfo } from "../../shared/useLayerToggle";
import type { ThemeMode } from "../../shared/useTheme";
import { ThemePicker } from "../ThemePicker";
import { SelectGroup } from "./SelectGroup";

export function AppearanceSection({ mode, setMode }: { mode: ThemeMode; setMode: (mode: ThemeMode) => void }) {
  const { t } = useLocale();
  return (
    <div className="settings-group">
      <span className="settings-group-label">{t("settings.theme")}</span>
      <ThemePicker mode={mode} setMode={setMode} />
    </div>
  );
}

const LOCALE_OPTIONS = LOCALES.map((value) => ({ value: value as Locale, label: LOCALE_LABELS[value] }));

export function LanguageSection() {
  const { locale, setLocale, t } = useLocale();
  return (
    <SelectGroup
      label={t("settings.language")}
      options={LOCALE_OPTIONS}
      selected={locale}
      selectedLabel={LOCALE_LABELS[locale]}
      onSelect={setLocale}
    />
  );
}

export function AutostartSection() {
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

export function MicrophoneSection() {
  const { t } = useLocale();
  const mic = useMicrophoneSettings();
  const options = [
    { value: null as string | null, label: t("settings.microphoneDefault") },
    ...mic.devices.map((device) => ({ value: device.deviceId as string | null, label: device.label })),
  ];
  const selectedLabel = options.find((option) => option.value === mic.selectedDeviceId)?.label ?? t("settings.microphoneDefault");

  return (
    <SelectGroup
      label={t("settings.microphone")}
      options={options}
      selected={mic.selectedDeviceId}
      selectedLabel={selectedLabel}
      onSelect={(deviceId) => void mic.setSelectedDeviceId(deviceId)}
      leadingIcon={<Mic size={14} />}
      trailing={
        <button
          className="icon-button"
          type="button"
          onClick={() => void mic.refresh()}
          title={t("settings.microphoneRefresh")}
          aria-label={t("settings.microphoneRefresh")}
        >
          <RefreshCw size={13} />
        </button>
      }
    >
      <MicrophoneControls mic={mic} />
    </SelectGroup>
  );
}

function MicrophoneControls({ mic }: { mic: ReturnType<typeof useMicrophoneSettings> }) {
  const { t } = useLocale();
  return (
    <>
      {mic.permissionNeeded && (
        <button className="preset-button" type="button" onClick={() => void mic.requestPermission()} disabled={mic.loading}>
          {t("settings.microphonePermission")}
        </button>
      )}
      <div className={`microphone-test ${mic.testing ? "is-active" : ""}`}>
        <button className="preset-button" type="button" onClick={() => void mic.toggleMicrophoneTest()}>
          {mic.testing ? t("settings.microphoneTestStop") : t("settings.microphoneTestStart")}
        </button>
        <div className="microphone-meter" aria-label={t("settings.microphoneLevel")}>
          <span style={{ transform: `scaleX(${Math.max(0.03, mic.testLevel)})` }} />
        </div>
      </div>
      {mic.testFailed && <p className="note-error">{t("settings.microphoneTestFailed")}</p>}
    </>
  );
}

export function NoteFoldersSection() {
  const { t } = useLocale();
  const [sort, setSort] = useState<NoteFolderSort>("recent");

  useEffect(() => {
    commands.settings.getNoteFolderSort().then(setSort).catch(() => setSort("recent"));
  }, []);

  async function select(nextSort: NoteFolderSort) {
    setSort(nextSort);
    await commands.settings.setNoteFolderSort(nextSort).catch(() => setSort(sort));
  }

  const choices: [NoteFolderSort, string][] = [
    ["recent", t("settings.noteFolderSortRecent")],
    ["name", t("settings.noteFolderSortName")],
  ];

  return (
    <div className="settings-group">
      <span className="settings-group-label">{t("settings.noteFolders")}</span>
      <div className="settings-choice-grid" role="group" aria-label={t("settings.noteFolderSort")}>
        {choices.map(([value, label]) => (
          <button
            key={value}
            className={`preset-button ${sort === value ? "is-active" : ""}`}
            type="button"
            onClick={() => void select(value)}
          >
            {label}
          </button>
        ))}
      </div>
    </div>
  );
}

export function DiagnosticsSection() {
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

export function ShortcutSection({ info }: { info: ShortcutInfo }) {
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
