import { useCallback, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Check, ChevronDown, Cloud, Database, KeyRound, Mic, RefreshCw, Server, ShieldCheck, X } from "lucide-react";
import { useTheme, type ThemeMode } from "../shared/useTheme";
import { commands, type Locale, type SyncProvider } from "../shared/commands";
import { ThemePicker } from "./ThemePicker";
import { LOCALES, LOCALE_LABELS } from "../shared/translations";
import { useLocale } from "../shared/useLocale";
import type { ShortcutInfo } from "../shared/useLayerToggle";
import { useOutsideClick } from "../shared/useOutsideClick";
import { useMicrophoneSettings } from "../shared/useMicrophoneSettings";

function AppearanceSection({
  mode,
  setMode,
}: {
  mode: ThemeMode;
  setMode: (mode: ThemeMode) => void;
}) {
  const { t } = useLocale();
  return (
    <div className="settings-group">
      <span className="settings-group-label">{t("settings.theme")}</span>
      <ThemePicker mode={mode} setMode={setMode} />
    </div>
  );
}

function LanguageSection() {
  const { locale, setLocale, t } = useLocale();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  useOutsideClick(rootRef, () => setOpen(false));

  function selectLocale(option: Locale) {
    setOpen(false);
    setLocale(option);
  }

  return (
    <div className="settings-group" ref={rootRef}>
      <span className="settings-group-label">{t("settings.language")}</span>
      <button
        className={`settings-select-trigger ${open ? "is-open" : ""}`}
        type="button"
        onClick={() => setOpen((value) => !value)}
        aria-label={t("settings.language")}
        aria-haspopup="listbox"
        aria-expanded={open}
      >
        <span>{LOCALE_LABELS[locale]}</span>
        <ChevronDown size={15} />
      </button>
      {open && (
        <div className="settings-select-menu" role="listbox" aria-label={t("settings.language")}>
          {LOCALES.map((option) => (
            <button
              key={option}
              className={`settings-select-option ${option === locale ? "is-active" : ""}`}
              type="button"
              role="option"
              aria-selected={option === locale}
              onClick={() => selectLocale(option)}
            >
              <span>{LOCALE_LABELS[option]}</span>
              {option === locale && <Check size={13} />}
            </button>
          ))}
        </div>
      )}
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

function MicrophoneSection() {
  const { t } = useLocale();
  const {
    devices,
    selectedDeviceId,
    loading,
    permissionNeeded,
    testing,
    testFailed,
    testLevel,
    requestPermission,
    refresh,
    setSelectedDeviceId,
    toggleMicrophoneTest,
  } = useMicrophoneSettings();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  useOutsideClick(rootRef, () => setOpen(false));
  const selectedLabel = devices.find((device) => device.deviceId === selectedDeviceId)?.label ?? t("settings.microphoneDefault");

  function select(deviceId: string | null) {
    setOpen(false);
    void setSelectedDeviceId(deviceId);
  }

  return (
    <div className="settings-group" ref={rootRef}>
      <span className="settings-group-label">{t("settings.microphone")}</span>
      <div className="settings-inline-row">
        <button
          className={`settings-select-trigger ${open ? "is-open" : ""}`}
          type="button"
          onClick={() => setOpen((value) => !value)}
          aria-label={t("settings.microphone")}
          aria-haspopup="listbox"
          aria-expanded={open}
        >
          <Mic size={14} />
          <span>{selectedLabel}</span>
          <ChevronDown size={15} />
        </button>
        <button
          className="icon-button"
          type="button"
          onClick={() => void refresh()}
          title={t("settings.microphoneRefresh")}
          aria-label={t("settings.microphoneRefresh")}
        >
          <RefreshCw size={13} />
        </button>
      </div>
      {open && (
        <div className="settings-select-menu" role="listbox" aria-label={t("settings.microphone")}>
          <button
            className={`settings-select-option ${selectedDeviceId === null ? "is-active" : ""}`}
            type="button"
            role="option"
            aria-selected={selectedDeviceId === null}
            onClick={() => select(null)}
          >
            <span>{t("settings.microphoneDefault")}</span>
            {selectedDeviceId === null && <Check size={13} />}
          </button>
          {devices.map((device) => (
            <button
              key={device.deviceId}
              className={`settings-select-option ${device.deviceId === selectedDeviceId ? "is-active" : ""}`}
              type="button"
              role="option"
              aria-selected={device.deviceId === selectedDeviceId}
              onClick={() => select(device.deviceId)}
            >
              <span>{device.label}</span>
              {device.deviceId === selectedDeviceId && <Check size={13} />}
            </button>
          ))}
        </div>
      )}
      {permissionNeeded && (
        <button className="preset-button" type="button" onClick={() => void requestPermission()} disabled={loading}>
          {t("settings.microphonePermission")}
        </button>
      )}
      <div className={`microphone-test ${testing ? "is-active" : ""}`}>
        <button className="preset-button" type="button" onClick={() => void toggleMicrophoneTest()}>
          {testing ? t("settings.microphoneTestStop") : t("settings.microphoneTestStart")}
        </button>
        <div className="microphone-meter" aria-label={t("settings.microphoneLevel")}>
          <span style={{ transform: `scaleX(${Math.max(0.03, testLevel)})` }} />
        </div>
      </div>
      {testFailed && <p className="note-error">{t("settings.microphoneTestFailed")}</p>}
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
function SyncProviderRow({
  provider,
  label,
  description,
  icon,
}: {
  provider: SyncProvider;
  label: string;
  description: string;
  icon: ReactNode;
}) {
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
    <div className={`sync-provider-row ${connected ? "is-connected" : ""}`}>
      <div className="sync-provider-icon">{icon}</div>
      <div className="sync-provider-info">
        <span>{label}</span>
        <span className="sync-provider-description">{description}</span>
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

function ServerSyncRow() {
  const { t } = useLocale();
  const [connected, setConnected] = useState(false);
  const [endpoint, setEndpoint] = useState("");
  const [token, setToken] = useState("");
  const [savedEndpoint, setSavedEndpoint] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);

  const refresh = useCallback(() => {
    commands.serverSync
      .status()
      .then((status) => {
        setConnected(status.connected);
        setSavedEndpoint(status.endpoint);
        if (status.endpoint) {
          setEndpoint(status.endpoint);
        }
      })
      .catch(() => {
        setConnected(false);
        setSavedEndpoint(null);
      });
  }, []);

  useEffect(() => refresh(), [refresh]);

  async function handleClick() {
    setError(false);
    setBusy(true);
    try {
      if (connected) {
        await commands.serverSync.disconnect();
        setToken("");
      } else {
        const status = await commands.serverSync.connect(endpoint, token);
        setConnected(status.connected);
        setSavedEndpoint(status.endpoint);
        setToken("");
      }
      refresh();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className={`sync-provider-row sync-provider-server ${connected ? "is-connected" : ""}`}>
      <div className="sync-provider-icon">
        <Server size={15} />
      </div>
      <div className="sync-provider-info">
        <span>{t("settings.syncServer")}</span>
        <span className="sync-provider-description">{t("settings.syncServerDesc")}</span>
        <span className="settings-hint">
          {connected ? `${t("settings.syncConnected")}: ${savedEndpoint}` : t("settings.syncServerNotConfigured")}
        </span>
      </div>
      <div className="server-sync-form">
        <input
          className="server-sync-input"
          value={endpoint}
          onChange={(event) => setEndpoint(event.target.value)}
          placeholder={t("settings.syncServerEndpoint")}
          disabled={busy || connected}
          aria-label={t("settings.syncServerEndpoint")}
        />
        {!connected && (
          <input
            className="server-sync-input"
            value={token}
            onChange={(event) => setToken(event.target.value)}
            placeholder={t("settings.syncServerToken")}
            disabled={busy}
            type="password"
            aria-label={t("settings.syncServerToken")}
          />
        )}
        <button className="preset-button" onClick={() => void handleClick()} disabled={busy}>
          {busy ? t("settings.syncConnecting") : connected ? t("settings.syncDisconnect") : t("settings.syncConnect")}
        </button>
      </div>
      {error && <p className="note-error">{t("settings.syncServerError")}</p>}
    </div>
  );
}

function SyncSection() {
  const { t } = useLocale();
  return (
    <div className="settings-group">
      <span className="settings-group-label">{t("settings.sync")}</span>
      <div className="account-sync-summary">
        <ShieldCheck size={15} />
        <div>
          <span>{t("settings.accountSyncTitle")}</span>
          <p>{t("settings.accountSyncHint")}</p>
        </div>
      </div>
      <SyncProviderRow
        provider="google_drive"
        label={t("settings.syncGoogleDrive")}
        description={t("settings.syncGoogleDriveDesc")}
        icon={<Cloud size={15} />}
      />
      <SyncProviderRow
        provider="yandex_disk"
        label={t("settings.syncYandexDisk")}
        description={t("settings.syncYandexDiskDesc")}
        icon={<Database size={15} />}
      />
      <ServerSyncRow />
      <p className="settings-secure-note">
        <KeyRound size={12} />
        <span>{t("settings.syncSecureNote")}</span>
      </p>
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

      <AppearanceSection mode={mode} setMode={setMode} />
      <LanguageSection />
      <MicrophoneSection />
      {/* "Запускать вместе с Windows" не имеет смысла на телефоне — раздел 11 ТЗ. */}
      {isDesktop && <AutostartSection />}
      {shortcutInfo && <ShortcutSection info={shortcutInfo} />}
      <SyncSection />
      <DiagnosticsSection />
    </div>
  );
}
