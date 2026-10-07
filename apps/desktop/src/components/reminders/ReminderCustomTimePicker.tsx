import { useState, type FormEvent } from "react";
import { CalendarDays, X } from "lucide-react";
import { dateKeyFromDate, formatDayLabel, monthKeyFromDateKey } from "../../shared/dateKeys";
import { useLocale } from "../../shared/useLocale";
import { CalendarPopover } from "../CalendarPopover";
import { TimeStepper } from "./TimeStepper";
import { DEFAULT_CUSTOM_OFFSET_MINUTES, computeValidTriggerIso, defaultCustomTime } from "./reminderTime";

function useCustomTime() {
  const [initial] = useState(defaultCustomTime);
  const [dateKey, setDateKey] = useState(initial.dateKey);
  const [monthKey, setMonthKey] = useState(() => monthKeyFromDateKey(initial.dateKey));
  const [hour, setHour] = useState(initial.hour);
  const [minute, setMinute] = useState(initial.minute);

  function pickDate(nextDateKey: string) {
    setDateKey(nextDateKey);
    setMonthKey(monthKeyFromDateKey(nextDateKey));
  }

  return {
    dateKey,
    monthKey,
    hour,
    minute,
    setMonthKey,
    setHour,
    setMinute,
    pickDate,
    setFromDate(date: Date) {
      pickDate(dateKeyFromDate(date));
      setHour(date.getHours());
      setMinute(date.getMinutes());
    },
    setClock(nextHour: number, nextMinute: number) {
      setHour(nextHour);
      setMinute(nextMinute);
    },
    triggerIso: computeValidTriggerIso(dateKey, hour, minute),
  };
}

function DateField({ time }: { time: ReturnType<typeof useCustomTime> }) {
  const { t, locale } = useLocale();
  const [open, setOpen] = useState(false);
  return (
    <div className="reminder-calendar-field">
      <button
        type="button"
        className="reminder-date-button"
        onClick={() => setOpen((value) => !value)}
        title={t("reminders.dateTimeLabel")}
        aria-label={t("reminders.dateTimeLabel")}
      >
        <CalendarDays size={13} />
        <span>{formatDayLabel(time.dateKey, locale)}</span>
      </button>
      {open && (
        <CalendarPopover
          placement="up"
          monthKey={time.monthKey}
          selectedDate={time.dateKey}
          onMonthChange={time.setMonthKey}
          onSelectDate={(nextDate) => {
            time.pickDate(nextDate);
            setOpen(false);
          }}
          onClose={() => setOpen(false)}
        />
      )}
    </div>
  );
}

function TimeShortcuts({ time }: { time: ReturnType<typeof useCustomTime> }) {
  const { t } = useLocale();
  const inMinutes = (minutes: number) => time.setFromDate(new Date(Date.now() + minutes * 60_000));
  return (
    <div className="reminder-time-shortcuts">
      <button type="button" onClick={() => inMinutes(DEFAULT_CUSTOM_OFFSET_MINUTES)}>
        {t("reminders.plus15")}
      </button>
      <button type="button" onClick={() => inMinutes(30)}>
        {t("reminders.plus30")}
      </button>
      <button type="button" onClick={() => time.setClock(9, 0)}>
        09:00
      </button>
      <button type="button" onClick={() => time.setClock(18, 0)}>
        18:00
      </button>
    </div>
  );
}

interface ReminderCustomTimePickerProps {
  disabled: boolean;
  onSubmit: (triggerAtUtc: string) => void;
  onCancel: () => void;
}

export function ReminderCustomTimePicker({ disabled, onSubmit, onCancel }: ReminderCustomTimePickerProps) {
  const { t } = useLocale();
  const time = useCustomTime();

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (time.triggerIso) onSubmit(time.triggerIso);
  }

  return (
    <form className="reminder-custom-time" onSubmit={handleSubmit}>
      <DateField time={time} />
      <div className="reminder-time-picker" aria-label={t("reminders.timeLabel")}>
        <TimeStepper label={t("reminders.hourLabel")} value={time.hour} min={0} max={23} onChange={time.setHour} />
        <span className="reminder-time-separator">:</span>
        <TimeStepper label={t("reminders.minuteLabel")} value={time.minute} min={0} max={59} step={5} onChange={time.setMinute} />
      </div>
      <button type="submit" className="preset-button" disabled={disabled || !time.triggerIso}>
        {t("reminders.add")}
      </button>
      <button type="button" className="icon-button" onClick={onCancel} title={t("common.cancel")} aria-label={t("common.cancel")}>
        <X size={14} />
      </button>
      <TimeShortcuts time={time} />
    </form>
  );
}
