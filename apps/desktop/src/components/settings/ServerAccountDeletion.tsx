import { useCallback, useEffect, useState } from "react";
import { commands } from "../../shared/commands";
import { useLocale } from "../../shared/useLocale";

type Props = { refreshKey: number; onConnectionChange: () => void };

export function ServerAccountDeletion({ refreshKey, onConnectionChange }: Props) {
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
