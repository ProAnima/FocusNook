import { BellRing, Trash2, Volume2 } from "lucide-react";
import type { Reminder } from "../../shared/commands";
import { formatReminderTime } from "../../shared/reminderPresets";
import { useHoldToConfirm } from "../../shared/useHoldToConfirm";
import { useLocale } from "../../shared/useLocale";
import { formatCountdown } from "./reminderTime";

interface ReminderRowProps {
  reminder: Reminder;
  now: number;
  onDelete: (id: string) => void;
  onOpenDetails: (reminder: Reminder) => void;
}

export function ReminderRow({ reminder, now, onDelete, onOpenDetails }: ReminderRowProps) {
  const { t, locale } = useLocale();
  const deleteHold = useHoldToConfirm(() => onDelete(reminder.id));
  const classes = ["reminder-item", reminder.audioPath ? "is-audio" : "", deleteHold.holding ? "is-delete-holding" : ""];
  return (
    <li className={classes.filter(Boolean).join(" ")}>
      <span className="reminder-kind">{reminder.audioPath ? <Volume2 size={13} /> : <BellRing size={13} />}</span>
      <button className="reminder-title" type="button" onClick={() => onOpenDetails(reminder)}>
        {reminder.title}
      </button>
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
