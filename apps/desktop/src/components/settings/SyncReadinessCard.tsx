import { Activity, RefreshCw } from "lucide-react";
import { useLocale } from "../../shared/useLocale";
import { useSyncReadiness } from "./useSyncReadiness";

export function SyncReadinessCard() {
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
