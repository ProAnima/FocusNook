import { useEffect, useMemo, useState, type FormEvent } from "react";
import {
  CalendarClock,
  CalendarDays,
  CalendarPlus,
  Check,
  ChevronLeft,
  ChevronRight,
  ListChecks,
  Percent,
  Plus,
  Trash2,
} from "lucide-react";
import { usePlanItems } from "../shared/usePlanItems";
import { commands, type PlanItem } from "../shared/commands";
import { useLocale } from "../shared/useLocale";
import { useReminders } from "../shared/useReminders";
import { useHoldToConfirm } from "../shared/useHoldToConfirm";
import {
  addDays,
  formatDayLabel,
  monthKeyFromDateKey,
  monthRange,
  todayDateKey,
} from "../shared/dateKeys";
import { buildCalendarMarks } from "../shared/calendarMarks";
import { EmptyState } from "./EmptyState";
import { CalendarPopover } from "./CalendarPopover";

interface PlanItemActions {
  onToggleDone: (id: string) => void;
  onCycleProgress: (id: string) => void;
  onToggleDeferred: (id: string) => void;
  onMoveNextDay: (id: string) => void;
  onDelete: (id: string) => void;
}

function useCalendarItems(monthKey: string) {
  const [items, setItems] = useState<PlanItem[]>([]);

  useEffect(() => {
    const { startDate, endDate } = monthRange(monthKey);
    commands.planItems
      .listRange(startDate, endDate)
      .then(setItems)
      .catch(() => setItems([]));
  }, [monthKey]);

  return items;
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
      {item.status !== "partial" && (
        <button
          className="icon-button"
          type="button"
          onClick={() => actions.onCycleProgress(item.id)}
          title={t("day.partial")}
          aria-label={t("day.partial")}
        >
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
      {item.status !== "done" && (
        <button
          className="icon-button"
          type="button"
          onClick={() => actions.onMoveNextDay(item.id)}
          title={t("day.moveNext")}
          aria-label={t("day.moveNext")}
        >
          <CalendarPlus size={13} />
        </button>
      )}
      <button
        className="icon-button hold-delete-button"
        type="button"
        title={t("common.delete")}
        aria-label={t("common.delete")}
        {...deleteButtonProps}
      >
        <Trash2 size={13} />
      </button>
    </div>
  );
}

function PlanItemRow({ item, actions }: { item: PlanItem; actions: PlanItemActions }) {
  const { t } = useLocale();
  const deleteHold = useHoldToConfirm(() => actions.onDelete(item.id));
  return (
    <li className={`plan-item status-${item.status} ${deleteHold.holding ? "is-delete-holding" : ""}`}>
      <button
        className="plan-checkbox"
        type="button"
        onClick={() => actions.onToggleDone(item.id)}
        aria-label={item.status === "done" ? t("day.markUndone") : t("day.markDone")}
      >
        {item.status === "done" && <Check size={12} />}
      </button>
      <span className="plan-title">{item.title}</span>
      {item.status === "partial" && (
        <button
          className="plan-progress"
          type="button"
          onClick={() => actions.onCycleProgress(item.id)}
          title={t("day.changeProgress")}
        >
          {item.progressPercent}%
        </button>
      )}
      <PlanItemActionsRow item={item} actions={actions} deleteButtonProps={deleteHold.buttonProps} />
    </li>
  );
}

function PlanList({ loaded, items, actions }: { loaded: boolean; items: PlanItem[]; actions: PlanItemActions }) {
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

function DayHeader({
  selectedDate,
  doneCount,
  total,
  onChangeDate,
  onOpenCalendar,
}: {
  selectedDate: string;
  doneCount: number;
  total: number;
  onChangeDate: (dateKey: string) => void;
  onOpenCalendar: () => void;
}) {
  const { t, locale } = useLocale();
  const today = todayDateKey();
  return (
    <div className="day-header">
      <div className="day-date-block">
        <span className="day-date">{formatDayLabel(selectedDate, locale)}</span>
        <span className="day-count">
          {doneCount}/{total}
        </span>
      </div>
      <div className="day-nav">
        <button className="icon-button" type="button" onClick={() => onChangeDate(addDays(selectedDate, -1))} title={t("day.previous")} aria-label={t("day.previous")}>
          <ChevronLeft size={14} />
        </button>
        {selectedDate !== today && (
          <button className="day-today-button" type="button" onClick={() => onChangeDate(today)}>
            {t("day.today")}
          </button>
        )}
        <button className="icon-button" type="button" onClick={onOpenCalendar} title={t("day.openCalendar")} aria-label={t("day.openCalendar")}>
          <CalendarDays size={14} />
        </button>
        <button className="icon-button" type="button" onClick={() => onChangeDate(addDays(selectedDate, 1))} title={t("day.next")} aria-label={t("day.next")}>
          <ChevronRight size={14} />
        </button>
      </div>
    </div>
  );
}

export function DayView() {
  const [selectedDate, setSelectedDate] = useState(todayDateKey);
  const [calendarOpen, setCalendarOpen] = useState(false);
  const [calendarMonth, setCalendarMonth] = useState(() => monthKeyFromDateKey(todayDateKey()));
  const plan = usePlanItems(selectedDate, selectedDate === todayDateKey());
  const { reminders } = useReminders();
  const calendarItems = useCalendarItems(calendarMonth);
  const calendarMarks = useMemo(() => buildCalendarMarks(calendarItems, reminders), [calendarItems, reminders]);
  const [draft, setDraft] = useState("");
  const doneCount = plan.items.filter((item) => item.status === "done").length;

  function changeDate(dateKey: string) {
    setSelectedDate(dateKey);
    setCalendarMonth(monthKeyFromDateKey(dateKey));
  }

  const actions: PlanItemActions = {
    onToggleDone: plan.toggleDone,
    onCycleProgress: plan.cycleProgress,
    onToggleDeferred: plan.toggleDeferred,
    onMoveNextDay: (id) => void plan.moveToDate(id, addDays(selectedDate, 1)),
    onDelete: plan.deleteItem,
  };

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const title = draft.trim();
    if (!title) return;
    setDraft("");
    void plan.addItem(title);
  }

  const { t } = useLocale();
  return (
    <div className="tab-view day-shell">
      <DayHeader
        selectedDate={selectedDate}
        doneCount={doneCount}
        total={plan.items.length}
        onChangeDate={changeDate}
        onOpenCalendar={() => setCalendarOpen((value) => !value)}
      />
      {calendarOpen && (
        <CalendarPopover
          monthKey={calendarMonth}
          selectedDate={selectedDate}
          marks={calendarMarks}
          onMonthChange={setCalendarMonth}
          onSelectDate={(dateKey) => {
            changeDate(dateKey);
            setCalendarOpen(false);
          }}
          onClose={() => setCalendarOpen(false)}
        />
      )}
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
