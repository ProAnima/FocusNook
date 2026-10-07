import { useEffect, useState } from "react";
import { commands } from "../../shared/commands";
import { useLocale } from "../../shared/useLocale";

export function AccountSyncToggle() {
  const { t } = useLocale();
  const [enabled, setEnabled] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [password, setPassword] = useState("");
  const [consent, setConsent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);

  useEffect(() => {
    commands.serverSync.status().then((status) => setEnabled(status.connected)).catch(() => setEnabled(false));
  }, []);

  async function change(task: () => Promise<void>) {
    setBusy(true);
    setError(false);
    try {
      await task();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }

  const disable = () =>
    change(async () => {
      await commands.serverSync.setEnabled(false);
      setEnabled(false);
    });

  const enable = () =>
    change(async () => {
      const status = await commands.serverSync.setEnabled(true, password, consent);
      setEnabled(status.connected);
      setConfirming(false);
      setPassword("");
    });

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
      {error && <p className="note-error">{t("settings.syncServerAuthError")}</p>}
    </div>
  );
}
