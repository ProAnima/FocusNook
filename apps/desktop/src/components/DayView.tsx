import { useEffect, useMemo, useState, type FormEvent } from "react";
import {
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  Plus,
} from "lucide-react";
import { usePlanItems } from "../shared/usePlanItems";
import { commands, type PlanItem } from "../shared/commands";
import { useLocale } from "../shared/useLocale";
import { useReminders } from "../shared/useReminders";
import {
  addDays,
  formatDayLabel,
  monthKeyFromDateKey,
  monthRange,
  todayDateKey,
} from "../shared/dateKeys";
import { buildCalendarMarks } from "../shared/calendarMarks";
import { CalendarPopover } from "./CalendarPopover";
import { PlanItemDetailsDialog } from "./PlanItemDetailsDialog";
import { PlanItemList, type PlanItemActions } from "./PlanItemList";

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
  const [detailsItem, setDetailsItem] = useState<PlanItem | null>(null);
  const plan = usePlanItems(selectedDate, selectedDate === todayDateKey());
  const { reminders } = useReminders();
  const calendarItems = useCalendarItems(calendarMonth);
  const calendarMarks = useMemo(() => buildCalendarMarks(calendarItems, reminders), [calendarItems, reminders]);
  const [draft, setDraft] = useState("");
  const dailyItems = plan.items.filter((item) => !item.isLongRunning);
  const doneCount = dailyItems.filter((item) => item.status === "done").length;

  function changeDate(dateKey: string) {
    setSelectedDate(dateKey);
    setCalendarMonth(monthKeyFromDateKey(dateKey));
  }

  const actions: PlanItemActions = {
    onOpenDetails: setDetailsItem,
    onToggleDone: plan.toggleDone,
    onCycleProgress: plan.cycleProgress,
    onToggleDeferred: plan.toggleDeferred,
    onToggleLongRunning: plan.toggleLongRunning,
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
        total={dailyItems.length}
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
      <PlanItemList loaded={plan.loaded} items={plan.items} actions={actions} />
      {detailsItem && (
        <PlanItemDetailsDialog
          item={plan.items.find((item) => item.id === detailsItem.id) ?? detailsItem}
          onToggleLongRunning={() => {
            void plan.toggleLongRunning(detailsItem.id).then((updated) => {
              if (updated && !updated.isLongRunning && updated.planDate !== selectedDate) {
                setDetailsItem(null);
              }
            });
          }}
          onCycleProgress={() => void plan.cycleProgress(detailsItem.id)}
          onToggleDeferred={() => void plan.toggleDeferred(detailsItem.id)}
          onDelete={() => {
            void plan.deleteItem(detailsItem.id);
            setDetailsItem(null);
          }}
          onClose={() => setDetailsItem(null)}
        />
      )}

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
