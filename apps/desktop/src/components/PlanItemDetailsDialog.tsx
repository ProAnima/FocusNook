import { CalendarClock, CalendarRange, Percent, Trash2 } from "lucide-react";
import type { PlanItem } from "../shared/commands";
import { useHoldToConfirm } from "../shared/useHoldToConfirm";
import { useLocale } from "../shared/useLocale";
import { ItemDetailsDialog } from "./ItemDetailsDialog";

function PlanItemDetailsActions({
  item,
  onCycleProgress,
  onToggleDeferred,
  onDelete,
}: {
  item: PlanItem;
  onCycleProgress: () => void;
  onToggleDeferred: () => void;
  onDelete: () => void;
}) {
  const { t } = useLocale();
  const deleteHold = useHoldToConfirm(onDelete);
  return (
    <div className="plan-details-actions">
      <button type="button" onClick={onCycleProgress}>
        <Percent size={15} />
        {item.status === "partial" ? t("day.changeProgress") : t("day.partial")}
      </button>
      <button type="button" onClick={onToggleDeferred}>
        <CalendarClock size={15} />
        {item.status === "deferred" ? t("day.resume") : t("day.defer")}
      </button>
      <button className="hold-delete-button" type="button" aria-label={t("common.delete")} {...deleteHold.buttonProps}>
        <Trash2 size={15} />
        {t("common.delete")}
      </button>
    </div>
  );
}

export function PlanItemDetailsDialog({
  item,
  onToggleLongRunning,
  onCycleProgress,
  onToggleDeferred,
  onDelete,
  onClose,
}: {
  item: PlanItem;
  onToggleLongRunning: () => void;
  onCycleProgress: () => void;
  onToggleDeferred: () => void;
  onDelete: () => void;
  onClose: () => void;
}) {
  const { t } = useLocale();
  return (
    <ItemDetailsDialog ariaLabel={item.title} onClose={onClose}>
      <p>{item.title}</p>
      <button
        className={`plan-long-running-toggle ${item.isLongRunning ? "is-active" : ""}`}
        type="button"
        onClick={onToggleLongRunning}
        aria-pressed={item.isLongRunning}
      >
        <CalendarRange size={17} />
        <span>
          <strong>{item.isLongRunning ? t("day.longRunningMarked") : t("day.longRunningAdd")}</strong>
          <small>{t("day.longRunningDescription")}</small>
        </span>
      </button>
      <PlanItemDetailsActions
        item={item}
        onCycleProgress={onCycleProgress}
        onToggleDeferred={onToggleDeferred}
        onDelete={onDelete}
      />
    </ItemDetailsDialog>
  );
}
