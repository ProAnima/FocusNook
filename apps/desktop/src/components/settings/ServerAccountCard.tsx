import { useState } from "react";
import { KeyRound, UserRound } from "lucide-react";
import { useLocale } from "../../shared/useLocale";
import type { ServerAccount } from "./useServerAccount";

function PasswordInput({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const { t } = useLocale();
  return (
    <input
      className="server-sync-input"
      value={value}
      onChange={(event) => onChange(event.target.value)}
      placeholder={t("settings.syncServerPassword")}
      autoComplete="current-password"
      type="password"
    />
  );
}

function DeleteAccountBlock({ server }: { server: ServerAccount }) {
  const { t } = useLocale();
  const [open, setOpen] = useState(false);
  const [password, setPassword] = useState("");

  async function confirm() {
    if (!password) return;
    if (await server.deleteAccount(password)) {
      setPassword("");
      setOpen(false);
    }
  }

  if (!open) {
    return (
      <button className="danger-link" type="button" onClick={() => setOpen(true)} disabled={server.busy}>
        {t("settings.syncServerDelete")}
      </button>
    );
  }
  return (
    <>
      <p className="settings-hint">{t("settings.syncServerDeleteHint")}</p>
      <PasswordInput value={password} onChange={setPassword} />
      <div className="server-account-delete-actions">
        <button className="preset-button" type="button" onClick={() => setOpen(false)} disabled={server.busy}>
          {t("common.cancel")}
        </button>
        <button className="danger-button" type="button" onClick={() => void confirm()} disabled={server.busy || !password}>
          {server.busy ? t("settings.syncConnecting") : t("settings.syncServerDeleteConfirm")}
        </button>
      </div>
      {server.error && <p className="note-error">{t("settings.syncServerDeleteError")}</p>}
    </>
  );
}

function MediaRepairBlock({ server }: { server: ServerAccount }) {
  const { t } = useLocale();
  const [password, setPassword] = useState("");
  const email = server.account.accountEmail;

  async function repair() {
    if (!email || !password) return;
    if (await server.signIn(email, password)) setPassword("");
  }

  return (
    <div className="server-account-repair">
      <div className="account-sync-summary is-warning">
        <KeyRound size={15} />
        <div>
          <span>{t("settings.syncServerMediaLocked")}</span>
          <p>{t("settings.syncServerMediaHint")}</p>
        </div>
      </div>
      <PasswordInput value={password} onChange={setPassword} />
      <button className="preset-button" onClick={() => void repair()} disabled={server.busy || !password || !email}>
        {server.busy ? t("settings.syncConnecting") : t("settings.syncServerRepairMedia")}
      </button>
      {server.error && <p className="note-error">{t("settings.syncServerAuthError")}</p>}
    </div>
  );
}

/** Карточка подключённого аккаунта VDS: отключение, удаление, восстановление ключа медиа. */
export function ServerAccountCard({ server }: { server: ServerAccount }) {
  const { t } = useLocale();
  const { accountEmail, displayName, endpoint, mediaReady } = server.account;
  return (
    <div className="server-account-card is-connected">
      <div className="server-account-head">
        <div className="sync-provider-icon">
          <UserRound size={15} />
        </div>
        <div className="sync-provider-info">
          <span>{displayName || accountEmail || t("settings.syncServerAccount")}</span>
          <span className="sync-provider-description">{accountEmail}</span>
          <span className="settings-hint">{endpoint}</span>
        </div>
        <button className="preset-button" onClick={() => void server.disconnect()} disabled={server.busy}>
          {server.busy ? t("settings.syncConnecting") : t("settings.syncDisconnect")}
        </button>
      </div>
      <div className="server-account-delete">
        <DeleteAccountBlock server={server} />
      </div>
      {!mediaReady && <MediaRepairBlock server={server} />}
    </div>
  );
}
