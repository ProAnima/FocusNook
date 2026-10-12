import {
  CalendarClock,
  CalendarPlus,
  CalendarRange,
  Check,
  ListChecks,
  Percent,
  Pencil,
  Trash2,
} from "lucide-react";
import type { PlanItem } from "../shared/commands";
import { useHoldToConfirm } from "../shared/useHoldToConfirm";
import { useLocale } from "../shared/useLocale";
import { usePlanItemLayout } from "../shared/usePlanItemLayout";
import { EmptyState } from "./EmptyState";

export interface PlanItemActions {
  onRename: (item: PlanItem) => void;
  onOpenDetails: (item: PlanItem) => void;
  onToggleDone: (id: string) => void;
  onCycleProgress: (id: string) => void;
  onToggleDeferred: (id: string) => void;
  onToggleLongRunning: (id: string) => void;
  onMoveNextDay: (id: string) => void;
  onDelete: (id: string) => void;
}

function LongRunningButton({ item, actions }: { item: PlanItem; actions: PlanItemActions }) {
  const { t } = useLocale();
  const label = item.isLongRunning ? t("day.longRunningRemove") : t("day.longRunningAdd");
  return (
    <button
      className={`icon-button long-running-action ${item.isLongRunning ? "is-active" : ""}`}
      type="button"
      onClick={() => actions.onToggleLongRunning(item.id)}
      title={label}
      aria-label={label}
      aria-pressed={item.isLongRunning}
    >
      <CalendarRange size={13} />
    </button>
  );
}

function PlanItemActionsRow({
  item,
  actions,
  deleteButtonProps,
}: {
  item: PlanItem;
  actions: PlanItemActions;
  deleteButtonProps: ReturnType<typeof useHoldToConfirm>["buttonProps"];
}) {
  const { t } = useLocale();
  return (
    <div className="plan-item-actions">
      <LongRunningButton item={item} actions={actions} />
      <button className="icon-button" type="button" onClick={() => actions.onRename(item)} title={t("day.rename")} aria-label={t("day.rename")}>
        <Pencil size={13} />
      </button>
      {item.status !== "partial" && (
        <button className="icon-button" type="button" onClick={() => actions.onCycleProgress(item.id)} title={t("day.partial")} aria-label={t("day.partial")}>
          <Percent size={13} />
        </button>
      )}
      <button
        className={`icon-button ${item.status === "deferred" ? "is-active" : ""}`}
        type="button"
        onClick={() => actions.onToggleDeferred(item.id)}
        title={item.status === "deferred" ? t("day.resume") : t("day.defer")}
        aria-label={item.status === "deferred" ? t("day.resume") : t("day.defer")}
      >
        <CalendarClock size={13} />
      </button>
      {item.status !== "done" && !item.isLongRunning && (
        <button className="icon-button" type="button" onClick={() => actions.onMoveNextDay(item.id)} title={t("day.moveNext")} aria-label={t("day.moveNext")}>
          <CalendarPlus size={13} />
        </button>
      )}
      <button className="icon-button hold-delete-button" type="button" title={t("common.delete")} aria-label={t("common.delete")} {...deleteButtonProps}>
        <Trash2 size={13} />
      </button>
    </div>
  );
}

export function PlanItemRow({ item, actions }: { item: PlanItem; actions: PlanItemActions }) {
  const { t } = useLocale();
  const deleteHold = useHoldToConfirm(() => actions.onDelete(item.id));
  const { rowRef, measureRef, stacked } = usePlanItemLayout(item.title, item.status);
  return (
    <li ref={rowRef} className={`plan-item status-${item.status} ${stacked ? "is-actions-stacked" : ""} ${item.isLongRunning ? "is-long-running" : ""} ${deleteHold.holding ? "is-delete-holding" : ""}`}>
      <button className="plan-checkbox" type="button" onClick={() => actions.onToggleDone(item.id)} aria-label={item.status === "done" ? t("day.markUndone") : t("day.markDone")}>
        {item.status === "done" && <Check size={12} />}
      </button>
      <button className="plan-title" type="button" onClick={() => actions.onOpenDetails(item)} aria-label={item.title}>
        <span className="plan-title-measure" ref={measureRef} data-title={item.title} aria-hidden="true" />
        {item.title}
      </button>
      {item.status === "partial" && (
        <button className="plan-progress" type="button" onClick={() => actions.onCycleProgress(item.id)} title={t("day.changeProgress")}>
          {item.progressPercent}%
        </button>
      )}
      <PlanItemActionsRow item={item} actions={actions} deleteButtonProps={deleteHold.buttonProps} />
    </li>
  );
}

function PlanItems({ items, actions }: { items: PlanItem[]; actions: PlanItemActions }) {
  return (
    <ul className="plan-list">
      {items.map((item) => <PlanItemRow key={item.id} item={item} actions={actions} />)}
    </ul>
  );
}

export function PlanItemList({ loaded, items, actions }: { loaded: boolean; items: PlanItem[]; actions: PlanItemActions }) {
  const { t } = useLocale();
  if (loaded && items.length === 0) {
    return <EmptyState icon={ListChecks} text={t("day.empty")} />;
  }
  const longRunning = items.filter((item) => item.isLongRunning);
  const regular = items.filter((item) => !item.isLongRunning);
  return (
    <div className="plan-groups">
      {longRunning.length > 0 && (
        <section className="long-running-group" aria-labelledby="long-running-title">
          <div className="long-running-header">
            <span className="long-running-heading" id="long-running-title"><CalendarRange size={13} />{t("day.longRunning")}</span>
            <span className="long-running-count" aria-label={`${t("day.longRunning")}: ${longRunning.length}`}>{longRunning.length}</span>
          </div>
          <PlanItems items={longRunning} actions={actions} />
        </section>
      )}
      {regular.length > 0 && <PlanItems items={regular} actions={actions} />}
    </div>
  );
}
