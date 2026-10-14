import { useCallback, useEffect, useRef, useState } from "react";
import { Activity, Check, ChevronDown, KeyRound, Mic, RefreshCw, ShieldCheck, X } from "lucide-react";
import { useTheme, type ThemeMode } from "../shared/useTheme";
import { commands, type Locale, type NoteFolderSort } from "../shared/commands";
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

function AccountSyncToggle({ refreshKey, onConnectionChange }: { refreshKey: number; onConnectionChange: () => void }) {
  const { t } = useLocale();
  const [enabled, setEnabled] = useState(false);
  const [accountEmail, setAccountEmail] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [password, setPassword] = useState("");
  const [consent, setConsent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);

  useEffect(() => {
    commands.serverSync.status().then((status) => {
      setEnabled(status.connected);
      setAccountEmail(status.accountEmail);
    }).catch(() => setEnabled(false));
  }, [refreshKey]);

  async function disable() {
    setBusy(true);
    setError(false);
    try {
      await commands.serverSync.setEnabled(false);
      setEnabled(false);
      setAccountEmail(null);
      onConnectionChange();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }

  async function enable() {
    setBusy(true);
    setError(false);
    try {
      const status = await commands.serverSync.setEnabled(true, password, consent);
      setEnabled(status.connected);
      setAccountEmail(status.accountEmail);
      setConfirming(false);
      setPassword("");
      onConnectionChange();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }

  async function syncNow() {
    setBusy(true);
    setError(false);
    try {
      await commands.serverSync.syncNow();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }

  function cancelConfirmation() {
    setConfirming(false);
    setPassword("");
    setConsent(false);
    setError(false);
  }

  return (
    <div className="account-sync-toggle-card">
      <label className="account-sync-toggle-row">
        <span><strong>{t("account.sync")}</strong><small>{t("account.syncHint")}</small></span>
        <input
          type="checkbox"
          role="switch"
          checked={enabled}
          disabled={busy}
          onChange={(event) => event.target.checked ? setConfirming(true) : void disable()}
        />
      </label>
      {confirming && !enabled && (
        <div className="account-sync-confirm">
          <input className="server-sync-input" type="password" autoComplete="current-password" value={password} onChange={(event) => setPassword(event.target.value)} placeholder={t("account.syncConfirm")} />
          <label className="server-privacy-consent"><input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} /><span>{t("account.syncConsent")}</span></label>
          <div className="account-sync-actions">
            <button className="account-secondary" type="button" disabled={busy} onClick={cancelConfirmation}>{t("common.cancel")}</button>
            <button className="preset-button" type="button" disabled={busy || !password || !consent} onClick={() => void enable()}>{t("account.syncEnable")}</button>
          </div>
        </div>
      )}
      {enabled && (
        <div className="account-sync-connected">
          <span>{accountEmail}</span>
          <button className="preset-button" type="button" disabled={busy} onClick={() => void syncNow()}>
            {t("settings.syncNow")}
          </button>
        </div>
      )}
      {error && <p className="note-error">{t("settings.syncServerAuthError")}</p>}
    </div>
  );
}

function ServerAccountDeletion({ refreshKey, onConnectionChange }: { refreshKey: number; onConnectionChange: () => void }) {
  const { t } = useLocale();
  const [available, setAvailable] = useState(false);
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);

  const refresh = useCallback(() => {
    commands.serverSync.status()
      .then((status) => setAvailable(status.connected && status.accountUserId !== null))
      .catch(() => setAvailable(false));
  }, []);

  useEffect(() => {
    refresh();
    let unlisten = () => {};
    void commands.serverSync.onCompleted(refresh).then((cleanup) => { unlisten = cleanup; });
    return () => unlisten();
  }, [refresh, refreshKey]);

  async function deleteAccount() {
    setBusy(true);
    setError(false);
    try {
      await commands.serverSync.deleteAccount(password);
      setAvailable(false);
      setPassword("");
      onConnectionChange();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }

  if (!available) return null;
  return (
    <details className="server-sync-advanced">
      <summary>{t("settings.syncServerDelete")}</summary>
      <div className="server-account-delete">
        <p className="settings-hint">{t("settings.syncServerDeleteHint")}</p>
        <input className="server-sync-input" value={password} onChange={(event) => setPassword(event.target.value)} placeholder={t("settings.syncServerPassword")} autoComplete="current-password" type="password" />
        <button className="danger-button" type="button" onClick={() => void deleteAccount()} disabled={busy || !password}>
          {busy ? t("settings.syncConnecting") : t("settings.syncServerDeleteConfirm")}
        </button>
        {error && <p className="note-error">{t("settings.syncServerDeleteError")}</p>}
      </div>
    </details>
  );
}

function SyncSection() {
  const { t } = useLocale();
  const [refreshKey, setRefreshKey] = useState(0);
  const refreshConnection = useCallback(() => setRefreshKey((value) => value + 1), []);
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
      <AccountSyncToggle refreshKey={refreshKey} onConnectionChange={refreshConnection} />
      <ServerAccountDeletion refreshKey={refreshKey} onConnectionChange={refreshConnection} />
      {/* Google Drive/Yandex Disk адаптеры отключены от UI на v1 (VDS-only,
          см. docs/v1-release-plan.md) — Rust/plugin-код остаётся в дереве
          нетронутым для пост-v1, но провайдеры больше не показываются и не
          подключаемы отсюда. */}
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
