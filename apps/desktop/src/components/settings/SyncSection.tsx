import { KeyRound, ShieldCheck } from "lucide-react";
import { useLocale } from "../../shared/useLocale";
import { AccountSyncToggle } from "./AccountSyncToggle";
import { ServerAccountCard } from "./ServerAccountCard";
import { ServerAuthForm } from "./ServerAuthForm";
import { SyncReadinessCard } from "./SyncReadinessCard";
import { useServerAccount } from "./useServerAccount";

function ServerSyncRow() {
  const server = useServerAccount();
  return server.account.connected ? <ServerAccountCard server={server} /> : <ServerAuthForm server={server} />;
}

export function SyncSection() {
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
      <AccountSyncToggle />
      {/* Google Drive/Yandex Disk адаптеры отключены от UI на v1 (VDS-only,
          см. docs/v1-release-plan.md) — Rust/plugin-код остаётся в дереве
          за Cargo-фичей `cloud-providers`, провайдеры отсюда не подключаются. */}
      <details className="server-sync-advanced">
        <summary>{t("account.syncAdvanced")}</summary>
        <ServerSyncRow />
      </details>
      <p className="settings-secure-note">
        <KeyRound size={12} />
        <span>{t("settings.syncSecureNote")}</span>
      </p>
    </div>
  );
}
