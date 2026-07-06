import { useEffect, useRef, useState } from "react";
import { BellRing } from "lucide-react";
import { commands, type Reminder } from "../shared/commands";
import { playChime } from "../shared/playChime";
import { useLocale } from "../shared/useLocale";

function AlertActions({
  onAcknowledge,
  onSnooze,
  onSnoozeTomorrow,
}: {
  onAcknowledge: () => void;
  onSnooze: (minutes: number) => void;
  onSnoozeTomorrow: () => void;
}) {
  const { t } = useLocale();
  const primaryRef = useRef<HTMLButtonElement>(null);

  // Раздел 21 ТЗ, screen reader smoke test: это отдельное topmost-окно,
  // которое появляется без действия пользователя — без явного фокуса
  // клавиатурный/screen reader пользователь не имеет стартовой точки на нём.
  useEffect(() => {
    primaryRef.current?.focus();
  }, []);

  return (
    <div className="alert-actions">
      <button ref={primaryRef} className="alert-action alert-action-primary" onClick={onAcknowledge}>
        {t("alert.acknowledge")}
      </button>
      <button className="alert-action" onClick={() => onSnooze(5)}>
        {t("alert.snooze5")}
      </button>
      <button className="alert-action" onClick={() => onSnooze(30)}>
        {t("alert.snooze30")}
      </button>
      <button className="alert-action" onClick={onSnoozeTomorrow}>
        {t("alert.snoozeTomorrow")}
      </button>
    </div>
  );
}

export function ReminderAlert() {
  const [reminder, setReminder] = useState<Reminder | null>(null);

  useEffect(() => {
    commands.reminders
      .getCurrentAlert()
      .then((current) => {
        setReminder(current);
        if (current) playChime();
      })
      .catch(() => {
        // Вне Tauri (browser-preview) текущего алерта нет — окно просто пустое.
      });
  }, []);

  if (!reminder) {
    return null;
  }

  const id = reminder.id;

  function snooze(minutes: number) {
    void commands.reminders.snooze(id, new Date(Date.now() + minutes * 60_000).toISOString());
  }

  function snoozeTomorrow() {
    const at = new Date();
    at.setDate(at.getDate() + 1);
    void commands.reminders.snooze(id, at.toISOString());
  }

  return (
    <div className="alert-shell">
      <BellRing size={18} className="alert-icon" />
      <p className="alert-title">{reminder.title}</p>
      <AlertActions
        onAcknowledge={() => void commands.reminders.acknowledge(id)}
        onSnooze={snooze}
        onSnoozeTomorrow={snoozeTomorrow}
      />
    </div>
  );
}
