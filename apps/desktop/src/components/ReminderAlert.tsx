import { useEffect, useRef } from "react";
import { BellRing, Volume2 } from "lucide-react";
import { commands } from "../shared/commands";
import { useReminderAlert } from "../shared/useReminderAlert";
import { useLocale } from "../shared/useLocale";

const MINUTE_MS = 60_000;

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

  // Это отдельное topmost-окно, которое появляется без действия пользователя —
  // без явного фокуса клавиатурный/screen reader пользователь не имеет на нём
  // стартовой точки (a11y).
  useEffect(() => {
    primaryRef.current?.focus();
  }, []);

  return (
    <div className="alert-actions">
      <button ref={primaryRef} className="alert-action alert-action-primary" onClick={onAcknowledge}>
        {t("alert.acknowledge")}
      </button>
      <button className="alert-action" onClick={() => onSnooze(10)}>
        {t("alert.snooze10")}
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
  const { reminder, stopPlayback } = useReminderAlert();

  if (!reminder) {
    return null;
  }

  const id = reminder.id;

  function snoozeUntil(at: Date) {
    stopPlayback();
    void commands.reminders.snooze(id, at.toISOString());
  }

  function snoozeTomorrow() {
    const at = new Date();
    at.setDate(at.getDate() + 1);
    snoozeUntil(at);
  }

  return (
    <div className="alert-shell">
      {reminder.audioPath ? <Volume2 size={18} className="alert-icon" /> : <BellRing size={18} className="alert-icon" />}
      <p className="alert-title">{reminder.title}</p>
      <AlertActions
        onAcknowledge={() => {
          stopPlayback();
          void commands.reminders.acknowledge(id);
        }}
        onSnooze={(minutes) => snoozeUntil(new Date(Date.now() + minutes * MINUTE_MS))}
        onSnoozeTomorrow={snoozeTomorrow}
      />
    </div>
  );
}
