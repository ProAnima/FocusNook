import { useState } from "react";
import { BellRing, Plus } from "lucide-react";
import { useReminders } from "../shared/useReminders";
import { REMINDER_PRESETS, formatReminderTime } from "../shared/reminderPresets";
import type { Reminder } from "../shared/commands";
import { EmptyState } from "./EmptyState";

function ReminderRow({ reminder }: { reminder: Reminder }) {
  return (
    <li className="reminder-item">
      <span className="reminder-title">{reminder.title}</span>
      <span className="reminder-time">{formatReminderTime(reminder.triggerAtUtc)}</span>
    </li>
  );
}

function ReminderComposer({
  onCreate,
}: {
  onCreate: (title: string, triggerAtUtc: string) => void;
}) {
  const [title, setTitle] = useState("");

  function handlePreset(computeTriggerAtUtc: () => string) {
    const value = title.trim();
    if (!value) return;
    setTitle("");
    onCreate(value, computeTriggerAtUtc());
  }

  return (
    <div className="reminder-composer">
      <div className="quick-add">
        <Plus size={14} />
        <input
          placeholder="Напомнить о..."
          value={title}
          onChange={(event) => setTitle(event.target.value)}
        />
      </div>
      <div className="reminder-presets">
        {REMINDER_PRESETS.map((preset) => (
          <button
            key={preset.key}
            className="preset-button"
            onClick={() => handlePreset(preset.computeTriggerAtUtc)}
            disabled={!title.trim()}
          >
            {preset.label}
          </button>
        ))}
      </div>
    </div>
  );
}

export function RemindersView() {
  const { reminders, loaded, addReminder } = useReminders();

  return (
    <div className="tab-view">
      {loaded && reminders.length === 0 ? (
        <EmptyState icon={BellRing} text="Нет активных напоминаний" />
      ) : (
        <ul className="reminder-list">
          {reminders.map((reminder) => (
            <ReminderRow key={reminder.id} reminder={reminder} />
          ))}
        </ul>
      )}

      <ReminderComposer onCreate={(title, triggerAtUtc) => void addReminder(title, triggerAtUtc)} />
    </div>
  );
}
