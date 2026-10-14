import { useEffect, useState } from "react";
import { commands } from "../../shared/commands";
import { useLocale } from "../../shared/useLocale";

type Props = { refreshKey: number; onConnectionChange: () => void };

type ConfirmProps = {
  busy: boolean;
  onCancel: () => void;
  onConfirm: (password: string, consent: boolean) => void;
};

function SyncConfirmation({ busy, onCancel, onConfirm }: ConfirmProps) {
  const { t } = useLocale();
  const [password, setPassword] = useState("");
  const [consent, setConsent] = useState(false);
  return (
    <div className="account-sync-confirm">
      <input className="server-sync-input" type="password" autoComplete="current-password" value={password} onChange={(event) => setPassword(event.target.value)} placeholder={t("account.syncConfirm")} />
      <label className="server-privacy-consent"><input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} /><span>{t("account.syncConsent")}</span></label>
      <div className="account-sync-actions">
        <button className="account-secondary" type="button" disabled={busy} onClick={onCancel}>{t("common.cancel")}</button>
        <button className="preset-button" type="button" disabled={busy || !password || !consent} onClick={() => onConfirm(password, consent)}>{t("account.syncEnable")}</button>
      </div>
    </div>
  );
}

export function AccountSyncToggle({ refreshKey, onConnectionChange }: Props) {
  const { t } = useLocale();
  const [enabled, setEnabled] = useState(false);
  const [accountEmail, setAccountEmail] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);

  useEffect(() => {
    commands.serverSync.status().then((status) => {
      setEnabled(status.connected);
      setAccountEmail(status.accountEmail);
    }).catch(() => setEnabled(false));
  }, [refreshKey]);

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
      setAccountEmail(null);
      onConnectionChange();
    });

  const enable = (password: string, consent: boolean) =>
    change(async () => {
      const status = await commands.serverSync.setEnabled(true, password, consent);
      setEnabled(status.connected);
      setAccountEmail(status.accountEmail);
      setConfirming(false);
      onConnectionChange();
    });

  const syncNow = () =>
    change(async () => {
      await commands.serverSync.syncNow();
    });

  function cancelConfirmation() {
    setConfirming(false);
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
        <SyncConfirmation busy={busy} onCancel={cancelConfirmation} onConfirm={(password, consent) => void enable(password, consent)} />
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
