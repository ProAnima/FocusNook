import { useEffect, useState, type FormEvent } from "react";
import { BellRing, CalendarDays, ChevronDown, ChevronUp, Mic, Plus, Square, Trash2, Volume2, X } from "lucide-react";
import { useReminders } from "../shared/useReminders";
import { getReminderPresets, formatReminderTime } from "../shared/reminderPresets";
import type { Reminder } from "../shared/commands";
import { useAudioRecorder } from "../shared/useAudioRecorder";
import { dateKeyFromDate, formatDayLabel, monthKeyFromDateKey, parseDateKey } from "../shared/dateKeys";
import type { LocaleContextValue } from "../shared/locale-context";
import { useLocale } from "../shared/useLocale";
import { useMicrophoneSettings } from "../shared/useMicrophoneSettings";
import { useHoldToConfirm } from "../shared/useHoldToConfirm";
import { CalendarPopover } from "./CalendarPopover";
import { EmptyState } from "./EmptyState";

function pad(value: number): string {
  return value.toString().padStart(2, "0");
}

function defaultCustomTime() {
  const date = new Date(Date.now() + 15 * 60_000);
  return {
    dateKey: dateKeyFromDate(date),
    hour: date.getHours(),
    minute: date.getMinutes(),
  };
}

function computeValidTriggerIso(dateKey: string, hour: number, minute: number): string | null {
  if (!Number.isInteger(hour) || !Number.isInteger(minute)) return null;
  if (hour < 0 || hour > 23 || minute < 0 || minute > 59) return null;
  const date = parseDateKey(dateKey);
  const triggerAt = new Date(date.getFullYear(), date.getMonth(), date.getDate(), hour, minute, 0, 0);
  if (Number.isNaN(triggerAt.getTime()) || triggerAt.getTime() <= Date.now()) return null;
  return triggerAt.toISOString();
}

function formatCountdown(triggerAtUtc: string, now: number, t: LocaleContextValue["t"]): string {
  if (now <= 0) return "";
  const diffMs = new Date(triggerAtUtc).getTime() - now;
  if (!Number.isFinite(diffMs) || diffMs <= 0) return t("reminders.countdownDue");
  const totalMinutes = Math.max(1, Math.ceil(diffMs / 60_000));
  if (totalMinutes < 60) {
    return t("reminders.countdownMinutes").replace("{minutes}", String(totalMinutes));
  }
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return t("reminders.countdownHours")
    .replace("{hours}", String(hours))
    .replace("{minutes}", String(minutes));
}

function wrapTimeValue(value: number, min: number, max: number): number {
  if (value > max) return min;
  if (value < min) return max;
  return value;
}

function TimeStepper({
  label,
  value,
  min,
  max,
  step = 1,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
}) {
  const upTitle = `${label} +`;
  const downTitle = `${label} -`;
  return (
    <div className="time-stepper" aria-label={label}>
      <button type="button" onClick={() => onChange(wrapTimeValue(value + step, min, max))} title={upTitle} aria-label={upTitle}>
        <ChevronUp size={12} />
      </button>
      <span>{pad(value)}</span>
      <button type="button" onClick={() => onChange(wrapTimeValue(value - step, min, max))} title={downTitle} aria-label={downTitle}>
        <ChevronDown size={12} />
      </button>
    </div>
  );
}

function ReminderRow({ reminder, now, onDelete }: { reminder: Reminder; now: number; onDelete: (id: string) => void }) {
  const { t, locale } = useLocale();
  const deleteHold = useHoldToConfirm(() => onDelete(reminder.id));
  return (
    <li className={`reminder-item ${reminder.audioPath ? "is-audio" : ""} ${deleteHold.holding ? "is-delete-holding" : ""}`}>
      <span className="reminder-kind">{reminder.audioPath ? <Volume2 size={13} /> : <BellRing size={13} />}</span>
      <span className="reminder-title">{reminder.title}</span>
      <span className="reminder-time-block">
        <span className="reminder-time">{formatReminderTime(reminder.triggerAtUtc, locale)}</span>
        <span className="reminder-countdown">{formatCountdown(reminder.triggerAtUtc, now, t)}</span>
      </span>
      <div className="reminder-item-actions">
        <button className="icon-button hold-delete-button" type="button" title={t("common.delete")} aria-label={t("common.delete")} {...deleteHold.buttonProps}>
          <Trash2 size={13} />
        </button>
      </div>
    </li>
  );
}

function ReminderPresetPicker({
  disabled,
  onPreset,
  onOpenCustom,
}: {
  disabled: boolean;
  onPreset: (computeTriggerAtUtc: () => string) => void;
  onOpenCustom: () => void;
}) {
  const { t, locale } = useLocale();
  return (
    <div className="reminder-presets">
      {getReminderPresets(locale).map((preset) => (
        <button key={preset.key} className="preset-button" onClick={() => onPreset(preset.computeTriggerAtUtc)} disabled={disabled}>
          {preset.label}
        </button>
      ))}
      <button className="preset-button" onClick={onOpenCustom} disabled={disabled}>
        {t("reminders.customTime")}
      </button>
    </div>
  );
}

function ReminderCustomTimePicker({
  disabled,
  onSubmit,
  onCancel,
}: {
  disabled: boolean;
  onSubmit: (triggerAtUtc: string) => void;
  onCancel: () => void;
}) {
  const initial = defaultCustomTime();
  const [dateKey, setDateKey] = useState(initial.dateKey);
  const [monthKey, setMonthKey] = useState(() => monthKeyFromDateKey(initial.dateKey));
  const [calendarOpen, setCalendarOpen] = useState(false);
  const [hour, setHour] = useState(initial.hour);
  const [minute, setMinute] = useState(initial.minute);
  const { t, locale } = useLocale();
  const iso = computeValidTriggerIso(dateKey, hour, minute);

  function setFromDate(date: Date) {
    const nextDateKey = dateKeyFromDate(date);
    setDateKey(nextDateKey);
    setMonthKey(monthKeyFromDateKey(nextDateKey));
    setHour(date.getHours());
    setMinute(date.getMinutes());
  }

  function setClock(nextHour: number, nextMinute: number) {
    setHour(nextHour);
    setMinute(nextMinute);
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (iso) onSubmit(iso);
  }

  return (
    <form className="reminder-custom-time" onSubmit={handleSubmit}>
      <div className="reminder-calendar-field">
        <button
          type="button"
          className="reminder-date-button"
          onClick={() => setCalendarOpen((value) => !value)}
          title={t("reminders.dateTimeLabel")}
          aria-label={t("reminders.dateTimeLabel")}
        >
          <CalendarDays size={13} />
          <span>{formatDayLabel(dateKey, locale)}</span>
        </button>
        {calendarOpen && (
          <CalendarPopover
            placement="up"
            monthKey={monthKey}
            selectedDate={dateKey}
            onMonthChange={setMonthKey}
            onSelectDate={(nextDate) => {
              setDateKey(nextDate);
              setMonthKey(monthKeyFromDateKey(nextDate));
              setCalendarOpen(false);
            }}
            onClose={() => setCalendarOpen(false)}
          />
        )}
      </div>
      <div className="reminder-time-picker" aria-label={t("reminders.timeLabel")}>
        <TimeStepper label={t("reminders.hourLabel")} value={hour} min={0} max={23} onChange={setHour} />
        <span className="reminder-time-separator">:</span>
        <TimeStepper label={t("reminders.minuteLabel")} value={minute} min={0} max={59} step={5} onChange={setMinute} />
      </div>
      <button type="submit" className="preset-button" disabled={disabled || !iso}>
        {t("reminders.add")}
      </button>
      <button type="button" className="icon-button" onClick={onCancel} title={t("common.cancel")} aria-label={t("common.cancel")}>
        <X size={14} />
      </button>
      <div className="reminder-time-shortcuts">
        <button type="button" onClick={() => setFromDate(new Date(Date.now() + 15 * 60_000))}>
          {t("reminders.plus15")}
        </button>
        <button type="button" onClick={() => setFromDate(new Date(Date.now() + 30 * 60_000))}>
          {t("reminders.plus30")}
        </button>
        <button type="button" onClick={() => setClock(9, 0)}>
          09:00
        </button>
        <button type="button" onClick={() => setClock(18, 0)}>
          18:00
        </button>
      </div>
    </form>
  );
}

function ReminderComposer({
  onCreate,
  onCreateAudio,
}: {
  onCreate: (title: string, triggerAtUtc: string) => void;
  onCreateAudio: (title: string, triggerAtUtc: string, audioBase64: string) => void;
}) {
  const [title, setTitle] = useState("");
  const [audioBase64, setAudioBase64] = useState<string | null>(null);
  const [customOpen, setCustomOpen] = useState(false);
  const { selectedDeviceId } = useMicrophoneSettings();
  const { recording, error, start, stop } = useAudioRecorder(setAudioBase64, selectedDeviceId);
  const { t } = useLocale();
  const hasContent = Boolean(title.trim() || audioBase64);
  const disabled = !hasContent || recording;

  function submit(triggerAtUtc: string) {
    const value = title.trim() || t("reminders.voiceDefaultTitle");
    if (audioBase64) {
      onCreateAudio(value, triggerAtUtc, audioBase64);
    } else if (title.trim()) {
      onCreate(value, triggerAtUtc);
    }
    setTitle("");
    setAudioBase64(null);
    setCustomOpen(false);
  }

  return (
    <div className="reminder-composer">
      <div className="quick-add reminder-input-row">
        <Plus size={14} />
        <input placeholder={t("reminders.addPlaceholder")} value={title} onChange={(event) => setTitle(event.target.value)} />
        <button
          type="button"
          className={`icon-button record-button ${recording ? "is-recording" : ""}`}
          onClick={() => (recording ? stop() : void start())}
          title={recording ? t("reminders.stopRecording") : t("reminders.record")}
          aria-label={recording ? t("reminders.stopRecording") : t("reminders.record")}
        >
          {recording ? <Square size={13} /> : <Mic size={13} />}
        </button>
      </div>
      {audioBase64 && (
        <div className="reminder-audio-chip">
          <Volume2 size={13} />
          <span>{t("reminders.voiceReady")}</span>
          <button type="button" className="icon-button" onClick={() => setAudioBase64(null)} title={t("common.delete")} aria-label={t("common.delete")}>
            <X size={12} />
          </button>
        </div>
      )}
      {error && <p className="note-error">{error}</p>}
      {customOpen ? (
        <ReminderCustomTimePicker disabled={disabled} onSubmit={submit} onCancel={() => setCustomOpen(false)} />
      ) : (
        <ReminderPresetPicker
          disabled={disabled}
          onPreset={(computeTriggerAtUtc) => submit(computeTriggerAtUtc())}
          onOpenCustom={() => setCustomOpen(true)}
        />
      )}
    </div>
  );
}

export function RemindersView() {
  const { reminders, loaded, addReminder, addAudioReminder, deleteReminder } = useReminders();
  const [now, setNow] = useState(0);
  const { t } = useLocale();

  useEffect(() => {
    const firstTick = window.setTimeout(() => setNow(Date.now()), 0);
    const timer = window.setInterval(() => setNow(Date.now()), 30_000);
    return () => {
      window.clearTimeout(firstTick);
      window.clearInterval(timer);
    };
  }, []);

  return (
    <div className="tab-view">
      {loaded && reminders.length === 0 ? (
        <EmptyState icon={BellRing} text={t("reminders.empty")} />
      ) : (
        <ul className="reminder-list">
          {reminders.map((reminder) => (
            <ReminderRow key={reminder.id} reminder={reminder} now={now} onDelete={deleteReminder} />
          ))}
        </ul>
      )}

      <ReminderComposer
        onCreate={(title, triggerAtUtc) => void addReminder(title, triggerAtUtc)}
        onCreateAudio={(title, triggerAtUtc, audioBase64) => void addAudioReminder(title, triggerAtUtc, audioBase64)}
      />
    </div>
  );
}
