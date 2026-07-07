import { useCallback, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Activity, Check, ChevronDown, Cloud, Database, KeyRound, Mic, RefreshCw, Server, ShieldCheck, UserRound, X } from "lucide-react";
import { useTheme, type ThemeMode } from "../shared/useTheme";
import { commands, type Locale, type NoteFolderSort, type SyncProvider } from "../shared/commands";
import { ThemePicker } from "./ThemePicker";
import { LOCALES, LOCALE_LABELS } from "../shared/translations";
import { useLocale } from "../shared/useLocale";
import type { ShortcutInfo } from "../shared/useLayerToggle";
import { useOutsideClick } from "../shared/useOutsideClick";
import { useMicrophoneSettings } from "../shared/useMicrophoneSettings";

const MIN_SERVER_PASSWORD_LENGTH = 7;

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

function NoteFoldersSection() {
  const { t } = useLocale();
  const [sort, setSort] = useState<NoteFolderSort>("recent");

  useEffect(() => {
    commands.settings.getNoteFolderSort().then(setSort).catch(() => setSort("recent"));
  }, []);

  async function select(nextSort: NoteFolderSort) {
    setSort(nextSort);
    await commands.settings.setNoteFolderSort(nextSort).catch(() => setSort(sort));
  }

  return (
    <div className="settings-group">
      <span className="settings-group-label">{t("settings.noteFolders")}</span>
      <div className="settings-choice-grid" role="group" aria-label={t("settings.noteFolderSort")}>
        <button
          className={`preset-button ${sort === "recent" ? "is-active" : ""}`}
          type="button"
          onClick={() => void select("recent")}
        >
          {t("settings.noteFolderSortRecent")}
        </button>
        <button
          className={`preset-button ${sort === "name" ? "is-active" : ""}`}
          type="button"
          onClick={() => void select("name")}
        >
          {t("settings.noteFolderSortName")}
        </button>
      </div>
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

function useSyncReadiness() {
  const [status, setStatus] = useState<Awaited<ReturnType<typeof commands.sync.readiness>> | null>(null);
  const [failed, setFailed] = useState(false);
  const refresh = useCallback(() => {
    commands.sync
      .readiness()
      .then((nextStatus) => {
        setStatus(nextStatus);
        setFailed(false);
      })
      .catch(() => {
        setStatus(null);
        setFailed(true);
      });
  }, []);
  useEffect(() => refresh(), [refresh]);
  return { status, failed, refresh };
}

function SyncReadinessCard() {
  const { t } = useLocale();
  const { status, failed, refresh } = useSyncReadiness();

  return (
    <div className="sync-readiness-card">
      <div className="sync-readiness-title">
        <Activity size={14} />
        <span>{t("settings.syncReadiness")}</span>
        <button
          className="icon-button"
          type="button"
          onClick={() => refresh()}
          title={t("settings.syncReadinessRefresh")}
          aria-label={t("settings.syncReadinessRefresh")}
        >
          <RefreshCw size={12} />
        </button>
      </div>
      <div className="sync-readiness-grid">
        <span>{t("settings.syncReadinessOperations")}</span>
        <strong>{status ? status.operationCount : "..."}</strong>
        <span>{t("settings.syncReadinessDevice")}</span>
        <strong>{status?.deviceIdHash ?? t("settings.syncReadinessNoDevice")}</strong>
        <span>{t("settings.syncReadinessLast")}</span>
        <strong>{status?.lastOperationAt ?? t("settings.syncReadinessNoOps")}</strong>
      </div>
      {failed && <p className="note-error">{t("settings.syncReadinessError")}</p>}
    </div>
  );
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
  const [mode, setMode] = useState<"login" | "register">("login");
  const [available, setAvailable] = useState(false);
  const [connected, setConnected] = useState(false);
  const [accountEmail, setAccountEmail] = useState<string | null>(null);
  const [displayName, setDisplayName] = useState<string | null>(null);
  const [mediaReady, setMediaReady] = useState(false);
  const [savedEndpoint, setSavedEndpoint] = useState<string | null>(null);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [repairPassword, setRepairPassword] = useState("");
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const passwordLength = Array.from(password).length;
  const registerPasswordInvalid = mode === "register" && passwordLength < MIN_SERVER_PASSWORD_LENGTH;
  const registerPasswordTooShort = mode === "register" && passwordLength > 0 && registerPasswordInvalid;

  const refresh = useCallback(() => {
    commands.serverSync
      .status()
      .then((status) => {
        setAvailable(status.available);
        setConnected(status.connected);
        setAccountEmail(status.accountEmail);
        setDisplayName(status.displayName);
        setMediaReady(status.mediaReady);
        setSavedEndpoint(status.endpoint);
      })
      .catch(() => {
        setAvailable(false);
        setConnected(false);
        setAccountEmail(null);
        setDisplayName(null);
        setMediaReady(false);
        setSavedEndpoint(null);
      });
  }, []);

  useEffect(() => refresh(), [refresh]);

  async function handleClick() {
    setError(false);
    if (!connected && registerPasswordInvalid) {
      return;
    }
    setBusy(true);
    try {
      if (connected) {
        await commands.serverSync.disconnect();
        setConnected(false);
        setAccountEmail(null);
        setDisplayName(null);
        setMediaReady(false);
      } else {
        const nextEmail = email.trim();
        const nextName = name.trim();
        const status =
          mode === "register"
            ? await commands.serverSync.register(nextEmail, password, nextName)
            : await commands.serverSync.login(nextEmail, password);
        setAvailable(status.available);
        setConnected(status.connected);
        setAccountEmail(status.accountEmail);
        setDisplayName(status.displayName);
        setMediaReady(status.mediaReady);
        setSavedEndpoint(status.endpoint);
        setPassword("");
      }
      refresh();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }

  async function handleMediaRepair() {
    const nextEmail = accountEmail || email.trim();
    if (!nextEmail || !repairPassword) {
      return;
    }
    setBusy(true);
    setError(false);
    try {
      const status = await commands.serverSync.login(nextEmail, repairPassword);
      setAvailable(status.available);
      setConnected(status.connected);
      setAccountEmail(status.accountEmail);
      setDisplayName(status.displayName);
      setMediaReady(status.mediaReady);
      setSavedEndpoint(status.endpoint);
      setRepairPassword("");
      refresh();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }

  if (connected) {
    return (
      <div className="server-account-card is-connected">
        <div className="server-account-head">
          <div className="sync-provider-icon">
            <UserRound size={15} />
          </div>
          <div className="sync-provider-info">
            <span>{displayName || accountEmail || t("settings.syncServerAccount")}</span>
            <span className="sync-provider-description">{accountEmail}</span>
            <span className="settings-hint">{savedEndpoint}</span>
          </div>
          <button className="preset-button" onClick={() => void handleClick()} disabled={busy}>
            {busy ? t("settings.syncConnecting") : t("settings.syncDisconnect")}
          </button>
        </div>
        {!mediaReady && (
          <div className="server-account-repair">
            <div className="account-sync-summary is-warning">
              <KeyRound size={15} />
              <div>
                <span>{t("settings.syncServerMediaLocked")}</span>
                <p>{t("settings.syncServerMediaHint")}</p>
              </div>
            </div>
            <input
              className="server-sync-input"
              value={repairPassword}
              onChange={(event) => setRepairPassword(event.target.value)}
              placeholder={t("settings.syncServerPassword")}
              autoComplete="current-password"
              type="password"
            />
            <button className="preset-button" onClick={() => void handleMediaRepair()} disabled={busy || !repairPassword}>
              {busy ? t("settings.syncConnecting") : t("settings.syncServerRepairMedia")}
            </button>
            {error && <p className="note-error">{t("settings.syncServerAuthError")}</p>}
          </div>
        )}
      </div>
    );
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
          {available ? t("settings.syncServerReady") : t("settings.syncServerNotConfigured")}
        </span>
      </div>
      <div className="server-account-form">
        <div className="server-account-tabs" role="tablist" aria-label={t("settings.syncServerAccount")}>
          <button className={mode === "login" ? "is-active" : ""} type="button" onClick={() => setMode("login")}>
            {t("settings.syncServerLogin")}
          </button>
          <button className={mode === "register" ? "is-active" : ""} type="button" onClick={() => setMode("register")}>
            {t("settings.syncServerRegister")}
          </button>
        </div>
        {mode === "register" && (
          <input
            className="server-sync-input"
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder={t("settings.syncServerName")}
            autoComplete="name"
          />
        )}
        <input
          className="server-sync-input"
          value={email}
          onChange={(event) => setEmail(event.target.value)}
          placeholder={t("settings.syncServerEmail")}
          autoComplete="email"
          inputMode="email"
        />
        <input
          className="server-sync-input"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          placeholder={t("settings.syncServerPassword")}
          autoComplete={mode === "register" ? "new-password" : "current-password"}
          type="password"
        />
        {mode === "register" && (
          <p className={`settings-hint server-password-hint ${registerPasswordTooShort ? "is-error" : ""}`}>
            {t("settings.syncServerPasswordHint")}
          </p>
        )}
        <button
          className="preset-button"
          onClick={() => void handleClick()}
          disabled={busy || !available || !email.trim() || !password || registerPasswordInvalid}
        >
          {busy ? t("settings.syncConnecting") : mode === "register" ? t("settings.syncServerCreate") : t("settings.syncServerSignIn")}
        </button>
        {error && <p className="note-error">{t("settings.syncServerAuthError")}</p>}
      </div>
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
      <SyncReadinessCard />
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
      {isDesktop && (
        <div className="settings-header">
          <span>{t("settings.title")}</span>
          <button className="icon-button" onClick={onClose} title={t("header.close")} aria-label={t("header.close")}>
            <X size={14} />
          </button>
        </div>
      )}

      <AppearanceSection mode={mode} setMode={setMode} />
      <LanguageSection />
      <NoteFoldersSection />
      <MicrophoneSection />
      {/* "Запускать вместе с Windows" не имеет смысла на телефоне — раздел 11 ТЗ. */}
      {isDesktop && <AutostartSection />}
      {shortcutInfo && <ShortcutSection info={shortcutInfo} />}
      <SyncSection />
      <DiagnosticsSection />
    </div>
  );
}
