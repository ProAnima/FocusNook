import { useCallback, useState } from "react";
import { KeyRound, ShieldCheck } from "lucide-react";
import { useLocale } from "../../shared/useLocale";
import { AccountSyncToggle } from "./AccountSyncToggle";
import { ServerAccountDeletion } from "./ServerAccountDeletion";
import { SyncReadinessCard } from "./SyncReadinessCard";

export function SyncSection() {
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
          за Cargo-фичей `cloud-providers`, провайдеры отсюда не подключаются. */}
      <p className="settings-secure-note">
        <KeyRound size={12} />
        <span>{t("settings.syncSecureNote")}</span>
      </p>
    </div>
  );
}
