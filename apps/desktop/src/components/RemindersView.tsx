import { useState, type FormEvent } from "react";
import { BellRing, Plus, Trash2, X } from "lucide-react";
import { useReminders } from "../shared/useReminders";
import { getReminderPresets, formatReminderTime, nowAsDatetimeLocal } from "../shared/reminderPresets";
import type { Reminder } from "../shared/commands";
import { useLocale } from "../shared/useLocale";
import { EmptyState } from "./EmptyState";

function ReminderRow({
  reminder,
  onDelete,
}: {
  reminder: Reminder;
  onDelete: (id: string) => void;
}) {
  const { t, locale } = useLocale();
  return (
    <li className="reminder-item">
      <span className="reminder-title">{reminder.title}</span>
      <span className="reminder-time">{formatReminderTime(reminder.triggerAtUtc, locale)}</span>
      <div className="reminder-item-actions">
        <button
          className="icon-button"
          onClick={() => onDelete(reminder.id)}
          title={t("common.delete")}
          aria-label={t("common.delete")}
        >
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
        <button
          key={preset.key}
          className="preset-button"
          onClick={() => onPreset(preset.computeTriggerAtUtc)}
          disabled={disabled}
        >
          {preset.label}
        </button>
      ))}
      <button className="preset-button" onClick={onOpenCustom} disabled={disabled}>
        {t("reminders.customTime")}
      </button>
    </div>
  );
}

// null, если пусто/невалидно/в прошлом — так handleSubmit ниже остаётся
// в бюджете строк ESLint (max-lines-per-function).
function computeValidTriggerIso(value: string): string | null {
  if (!value) return null;
  const triggerAt = new Date(value);
  if (Number.isNaN(triggerAt.getTime()) || triggerAt.getTime() <= Date.now()) return null;
  return triggerAt.toISOString();
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
  const [value, setValue] = useState("");
  const { t } = useLocale();

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const iso = computeValidTriggerIso(value);
    if (iso) onSubmit(iso);
  }

  return (
    <form className="reminder-custom-time" onSubmit={handleSubmit}>
      <input
        type="datetime-local"
        aria-label={t("reminders.dateTimeLabel")}
        value={value}
        min={nowAsDatetimeLocal()}
        onChange={(event) => setValue(event.target.value)}
      />
      <button type="submit" className="preset-button" disabled={disabled || !value}>
        {t("reminders.add")}
      </button>
      <button
        type="button"
        className="icon-button"
        onClick={onCancel}
        title={t("common.cancel")}
        aria-label={t("common.cancel")}
      >
        <X size={14} />
      </button>
    </form>
  );
}

function ReminderComposer({ onCreate }: { onCreate: (title: string, triggerAtUtc: string) => void }) {
  const [title, setTitle] = useState("");
  const [customOpen, setCustomOpen] = useState(false);
  const { t } = useLocale();
  const hasTitle = Boolean(title.trim());

  function submit(triggerAtUtc: string) {
    const value = title.trim();
    if (!value) return;
    setTitle("");
    setCustomOpen(false);
    onCreate(value, triggerAtUtc);
  }

  return (
    <div className="reminder-composer">
      <div className="quick-add">
        <Plus size={14} />
        <input
          placeholder={t("reminders.addPlaceholder")}
          value={title}
          onChange={(event) => setTitle(event.target.value)}
        />
      </div>
      {customOpen ? (
        <ReminderCustomTimePicker disabled={!hasTitle} onSubmit={submit} onCancel={() => setCustomOpen(false)} />
      ) : (
        <ReminderPresetPicker
          disabled={!hasTitle}
          onPreset={(computeTriggerAtUtc) => submit(computeTriggerAtUtc())}
          onOpenCustom={() => setCustomOpen(true)}
        />
      )}
    </div>
  );
}

export function RemindersView() {
  const { reminders, loaded, addReminder, deleteReminder } = useReminders();
  const { t } = useLocale();

  return (
    <div className="tab-view">
      {loaded && reminders.length === 0 ? (
        <EmptyState icon={BellRing} text={t("reminders.empty")} />
      ) : (
        <ul className="reminder-list">
          {reminders.map((reminder) => (
            <ReminderRow key={reminder.id} reminder={reminder} onDelete={deleteReminder} />
          ))}
        </ul>
      )}

      <ReminderComposer onCreate={(title, triggerAtUtc) => void addReminder(title, triggerAtUtc)} />
    </div>
  );
}
