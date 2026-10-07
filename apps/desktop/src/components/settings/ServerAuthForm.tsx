import { useState } from "react";
import { Server } from "lucide-react";
import { commands } from "../../shared/commands";
import { useLocale } from "../../shared/useLocale";
import type { ServerAccount } from "./useServerAccount";
type AuthMode = "login" | "register";

const MIN_SERVER_PASSWORD_LENGTH = 7;

function ModeTabs({ mode, setMode }: { mode: AuthMode; setMode: (mode: AuthMode) => void }) {
  const { t } = useLocale();
  return (
    <div className="server-account-tabs" role="tablist" aria-label={t("settings.syncServerAccount")}>
      <button className={mode === "login" ? "is-active" : ""} type="button" role="tab" aria-selected={mode === "login"} onClick={() => setMode("login")}>
        {t("settings.syncServerLogin")}
      </button>
      <button className={mode === "register" ? "is-active" : ""} type="button" role="tab" aria-selected={mode === "register"} onClick={() => setMode("register")}>
        {t("settings.syncServerRegister")}
      </button>
    </div>
  );
}

function RegisterConsent({
  passwordTooShort,
  accepted,
  setAccepted,
}: {
  passwordTooShort: boolean;
  accepted: boolean;
  setAccepted: (value: boolean) => void;
}) {
  const { t } = useLocale();
  return (
    <>
      <p className={`settings-hint server-password-hint ${passwordTooShort ? "is-error" : ""}`}>
        {t("settings.syncServerPasswordHint")}
      </p>
      <label className="server-privacy-consent">
        <input type="checkbox" checked={accepted} onChange={(event) => setAccepted(event.target.checked)} />
        <span>{t("settings.syncServerPrivacyConsent")}</span>
      </label>
      <button className="privacy-link" type="button" onClick={() => void commands.legal.openPrivacy()}>
        {t("settings.syncServerPrivacyOpen")}
      </button>
    </>
  );
}

/** Форма входа/регистрации на VDS. Пароль очищается только после успешной попытки. */
export function ServerAuthForm({ server }: { server: ServerAccount }) {
  const { t } = useLocale();
  const [mode, setMode] = useState<AuthMode>("login");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [privacyAccepted, setPrivacyAccepted] = useState(false);

  const registering = mode === "register";
  const passwordLength = Array.from(password).length;
  const passwordTooShort = registering && passwordLength < MIN_SERVER_PASSWORD_LENGTH;
  const canSubmit =
    !server.busy && server.account.available && email.trim() && password && !passwordTooShort && (!registering || privacyAccepted);

  async function submit() {
    if (passwordTooShort) return;
    const nextEmail = email.trim();
    const ok = registering
      ? await server.register(nextEmail, password, name.trim(), privacyAccepted)
      : await server.signIn(nextEmail, password);
    if (ok) setPassword("");
  }

  return (
    <div className="sync-provider-row sync-provider-server">
      <div className="sync-provider-icon">
        <Server size={15} />
      </div>
      <div className="sync-provider-info">
        <span>{t("settings.syncServer")}</span>
        <span className="sync-provider-description">{t("settings.syncServerDesc")}</span>
        <span className="settings-hint">
          {server.account.available ? t("settings.syncServerReady") : t("settings.syncServerNotConfigured")}
        </span>
      </div>
      <div className="server-account-form">
        <ModeTabs mode={mode} setMode={setMode} />
        {registering && (
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
          autoComplete={registering ? "new-password" : "current-password"}
          type="password"
        />
        {registering && (
          <RegisterConsent
            passwordTooShort={passwordTooShort && passwordLength > 0}
            accepted={privacyAccepted}
            setAccepted={setPrivacyAccepted}
          />
        )}
        <button className="preset-button" onClick={() => void submit()} disabled={!canSubmit}>
          {server.busy ? t("settings.syncConnecting") : registering ? t("settings.syncServerCreate") : t("settings.syncServerSignIn")}
        </button>
        {server.error && <p className="note-error">{t("settings.syncServerAuthError")}</p>}
      </div>
    </div>
  );
}
