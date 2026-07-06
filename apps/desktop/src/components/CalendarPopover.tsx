import { useMemo, useRef } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { useLocale } from "../shared/useLocale";
import { INTL_LOCALE_TAG } from "../shared/translations";
import { useOutsideClick } from "../shared/useOutsideClick";
import type { CalendarMarks } from "../shared/calendarMarks";
import {
  addMonths,
  formatDayLabel,
  formatMonthLabel,
  monthGrid,
  parseDateKey,
  todayDateKey,
} from "../shared/dateKeys";

function buildWeekdayLabels(localeTag: string): string[] {
  return Array.from({ length: 7 }, (_, index) =>
    new Intl.DateTimeFormat(localeTag, { weekday: "short" }).format(new Date(2026, 0, 5 + index)),
  );
}

export function CalendarPopover({
  monthKey,
  selectedDate,
  marks = {},
  placement = "down",
  onMonthChange,
  onSelectDate,
  onClose,
}: {
  monthKey: string;
  selectedDate: string;
  marks?: Record<string, CalendarMarks>;
  placement?: "down" | "up";
  onMonthChange: (monthKey: string) => void;
  onSelectDate: (dateKey: string) => void;
  onClose: () => void;
}) {
  const { t, locale } = useLocale();
  const rootRef = useRef<HTMLDivElement>(null);
  const days = useMemo(() => monthGrid(monthKey), [monthKey]);
  const localeTag = INTL_LOCALE_TAG[locale];
  useOutsideClick(rootRef, onClose);

  return (
    <div className={`calendar-popover is-placement-${placement}`} ref={rootRef}>
      <div className="calendar-header">
        <button className="icon-button" type="button" onClick={() => onMonthChange(addMonths(monthKey, -1))} title={t("day.previous")} aria-label={t("day.previous")}>
          <ChevronLeft size={14} />
        </button>
        <span>{formatMonthLabel(monthKey, locale)}</span>
        <button className="icon-button" type="button" onClick={() => onMonthChange(addMonths(monthKey, 1))} title={t("day.next")} aria-label={t("day.next")}>
          <ChevronRight size={14} />
        </button>
      </div>
      <div className="calendar-grid">
        {buildWeekdayLabels(localeTag).map((label) => (
          <span key={label} className="calendar-weekday">
            {label}
          </span>
        ))}
        {days.map((dateKey) => {
          const mark = marks[dateKey];
          const isOtherMonth = !dateKey.startsWith(monthKey);
          const isToday = dateKey === todayDateKey();
          return (
            <button
              key={dateKey}
              className={`calendar-day ${isOtherMonth ? "is-muted" : ""} ${dateKey === selectedDate ? "is-selected" : ""} ${isToday ? "is-today" : ""}`}
              type="button"
              onClick={() => onSelectDate(dateKey)}
              title={`${formatDayLabel(dateKey, locale)}${mark?.tasks ? `, ${t("day.calendarTasks")}: ${mark.tasks}` : ""}${mark?.reminders ? `, ${t("day.calendarReminders")}: ${mark.reminders}` : ""}`}
            >
              <span>{parseDateKey(dateKey).getDate()}</span>
              <span className="calendar-marks">
                {mark?.tasks ? <span className="calendar-dot is-task" /> : null}
                {mark?.reminders ? <span className="calendar-dot is-reminder" /> : null}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
