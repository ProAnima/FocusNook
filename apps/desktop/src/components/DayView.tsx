import { useState, type FormEvent } from "react";
import { CalendarClock, Check, ListChecks, Percent, Plus, Trash2 } from "lucide-react";
import { usePlanItems } from "../shared/usePlanItems";
import type { PlanItem } from "../shared/commands";
import { INTL_LOCALE_TAG } from "../shared/translations";
import { useLocale } from "../shared/useLocale";
import { EmptyState } from "./EmptyState";

function todayLabel(localeTag: string): string {
  return new Intl.DateTimeFormat(localeTag, {
    weekday: "long",
    day: "numeric",
    month: "long",
  }).format(new Date());
}

interface PlanItemActions {
  onToggleDone: (id: string) => void;
  onCycleProgress: (id: string) => void;
  onToggleDeferred: (id: string) => void;
  onDelete: (id: string) => void;
}

// Раздел 12 ТЗ: "контекстное меню: редактировать, отложить, удалить" —
// реализовано как иконки-действия (см. .plan-item-actions в App.css,
// скрыты до наведения/фокуса), а не выпадающее меню.
function PlanItemActionsRow({ item, actions }: { item: PlanItem; actions: PlanItemActions }) {
  const { t } = useLocale();
  return (
    <div className="plan-item-actions">
      {item.status !== "partial" && (
        <button
          className="icon-button"
          onClick={() => actions.onCycleProgress(item.id)}
          title={t("day.partial")}
          aria-label={t("day.partial")}
        >
          <Percent size={13} />
        </button>
      )}
      <button
        className={`icon-button ${item.status === "deferred" ? "is-active" : ""}`}
        onClick={() => actions.onToggleDeferred(item.id)}
        title={item.status === "deferred" ? t("day.resume") : t("day.defer")}
        aria-label={item.status === "deferred" ? t("day.resume") : t("day.defer")}
      >
        <CalendarClock size={13} />
      </button>
      <button
        className="icon-button"
        onClick={() => actions.onDelete(item.id)}
        title={t("common.delete")}
        aria-label={t("common.delete")}
      >
        <Trash2 size={13} />
      </button>
    </div>
  );
}

function PlanItemRow({ item, actions }: { item: PlanItem; actions: PlanItemActions }) {
  const { t } = useLocale();
  return (
    <li className={`plan-item status-${item.status}`}>
      <button
        className="plan-checkbox"
        onClick={() => actions.onToggleDone(item.id)}
        aria-label={item.status === "done" ? t("day.markUndone") : t("day.markDone")}
      >
        {item.status === "done" && <Check size={12} />}
      </button>
      <span className="plan-title">{item.title}</span>
      {item.status === "partial" && (
        <button
          className="plan-progress"
          onClick={() => actions.onCycleProgress(item.id)}
          title={t("day.changeProgress")}
        >
          {item.progressPercent}%
        </button>
      )}
      <PlanItemActionsRow item={item} actions={actions} />
    </li>
  );
}

function PlanList({
  loaded,
  items,
  actions,
}: {
  loaded: boolean;
  items: PlanItem[];
  actions: PlanItemActions;
}) {
  const { t } = useLocale();
  if (loaded && items.length === 0) {
    return <EmptyState icon={ListChecks} text={t("day.empty")} />;
  }
  return (
    <ul className="plan-list">
      {items.map((item) => (
        <PlanItemRow key={item.id} item={item} actions={actions} />
      ))}
    </ul>
  );
}

function DayHeader({ doneCount, total }: { doneCount: number; total: number }) {
  const { locale } = useLocale();
  return (
    <div className="day-header">
      <span className="day-date">{todayLabel(INTL_LOCALE_TAG[locale])}</span>
      <span className="day-count">
        {doneCount}/{total}
      </span>
    </div>
  );
}

export function DayView() {
  const plan = usePlanItems();
  const [draft, setDraft] = useState("");
  const { t } = useLocale();
  const doneCount = plan.items.filter((item) => item.status === "done").length;
  const actions: PlanItemActions = {
    onToggleDone: plan.toggleDone,
    onCycleProgress: plan.cycleProgress,
    onToggleDeferred: plan.toggleDeferred,
    onDelete: plan.deleteItem,
  };

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const title = draft.trim();
    if (!title) return;
    setDraft("");
    void plan.addItem(title);
  }

  return (
    <div className="tab-view">
      <DayHeader doneCount={doneCount} total={plan.items.length} />
      <PlanList loaded={plan.loaded} items={plan.items} actions={actions} />

      <form className="quick-add" onSubmit={handleSubmit}>
        <Plus size={14} />
        <input
          placeholder={t("day.addPlaceholder")}
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
        />
      </form>
    </div>
  );
}
