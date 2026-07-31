import { useState, type FormEvent } from "react";
import { LockKeyhole, Plus, UserRound } from "lucide-react";
import type { Profile } from "../shared/commands";
import { useLocale } from "../shared/useLocale";

interface AccountGateProps {
  accounts: Profile[];
  activeAccount: Profile | null;
  setupRequired: boolean;
  onConfigure: (name: string, email: string, password: string) => Promise<void>;
  onCreate: (name: string, email: string, password: string) => Promise<void>;
  onSignIn: (id: string, password: string) => Promise<void>;
}

export function AccountGate({
  accounts,
  activeAccount,
  setupRequired,
  onConfigure,
  onCreate,
  onSignIn,
}: AccountGateProps) {
  const { t } = useLocale();
  const [createMode, setCreateMode] = useState(setupRequired);
  const [selectedId, setSelectedId] = useState(activeAccount?.id ?? accounts[0]?.id ?? "");
  const [name, setName] = useState(activeAccount?.accountConfigured ? "" : activeAccount?.displayName ?? "");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setError(null);
    if (createMode && password !== confirmation) {
      setError(t("account.mismatch"));
      return;
    }
    setBusy(true);
    try {
      if (createMode) {
        const action = setupRequired ? onConfigure : onCreate;
        await action(name.trim(), email.trim(), password);
      } else if (selectedId) {
        await onSignIn(selectedId, password);
      }
    } catch {
      setError(t("account.error"));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="account-gate">
      <div className="account-card">
        <div className="account-mark"><LockKeyhole size={22} /></div>
        <h1>{createMode ? t("account.welcome") : t("account.choose")}</h1>
        {createMode && <p>{t("account.setupHint")}</p>}
        <form onSubmit={(event) => void submit(event)}>
          {!createMode && (
            <div className="account-list" role="radiogroup" aria-label={t("account.choose")}>
              {accounts.filter((account) => account.accountConfigured).map((account) => (
                <button
                  key={account.id}
                  type="button"
                  role="radio"
                  aria-checked={selectedId === account.id}
                  className={`account-option ${selectedId === account.id ? "is-active" : ""}`}
                  onClick={() => setSelectedId(account.id)}
                >
                  <span className="account-avatar" style={{ background: account.avatarColor }}>
                    {account.displayName.charAt(0).toUpperCase()}
                  </span>
                  <span><strong>{account.displayName}</strong><small>{account.email}</small></span>
                </button>
              ))}
            </div>
          )}
          {createMode && (
            <>
              <label>{t("account.name")}<input value={name} onChange={(event) => setName(event.target.value)} autoComplete="name" required /></label>
              <label>{t("account.email")}<input value={email} onChange={(event) => setEmail(event.target.value)} autoComplete="email" inputMode="email" type="email" required /></label>
            </>
          )}
          <label>{t("account.password")}<input value={password} onChange={(event) => setPassword(event.target.value)} autoComplete={createMode ? "new-password" : "current-password"} type="password" minLength={10} required /></label>
          {createMode && (
            <label>{t("account.confirmPassword")}<input value={confirmation} onChange={(event) => setConfirmation(event.target.value)} autoComplete="new-password" type="password" minLength={10} required /><small>{t("account.passwordHint")}</small></label>
          )}
          {error && <span className="account-form-error" role="alert">{error}</span>}
          <button className="account-primary" type="submit" disabled={busy}>
            {createMode ? <UserRound size={16} /> : <LockKeyhole size={16} />}
            {createMode ? t("account.create") : t("account.signIn")}
          </button>
          {!setupRequired && (
            <button className="account-secondary" type="button" onClick={() => { setCreateMode((value) => !value); setError(null); }}>
              <Plus size={15} />{createMode ? t("account.choose") : t("account.add")}
            </button>
          )}
        </form>
      </div>
    </div>
  );
}
