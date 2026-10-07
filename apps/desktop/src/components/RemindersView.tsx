import { useEffect, useState } from "react";
import { BellRing } from "lucide-react";
import { useReminders } from "../shared/useReminders";
import { formatReminderTime } from "../shared/reminderPresets";
import { commands, type Reminder } from "../shared/commands";
import { useLocale } from "../shared/useLocale";
import { AudioMessagePlayer } from "./AudioMessagePlayer";
import { EmptyState } from "./EmptyState";
import { ItemDetailsDialog } from "./ItemDetailsDialog";
import { ReminderComposer } from "./reminders/ReminderComposer";
import { ReminderRow } from "./reminders/ReminderRow";

const COUNTDOWN_REFRESH_MS = 30_000;

/** Текущее время, обновляемое раз в полминуты для обратного отсчёта. */
function useNow() {
  const [now, setNow] = useState(0);
  useEffect(() => {
    const firstTick = window.setTimeout(() => setNow(Date.now()), 0);
    const timer = window.setInterval(() => setNow(Date.now()), COUNTDOWN_REFRESH_MS);
    return () => {
      window.clearTimeout(firstTick);
      window.clearInterval(timer);
    };
  }, []);
  return now;
}

function ReminderDetails({ reminder, onClose }: { reminder: Reminder; onClose: () => void }) {
  const { locale } = useLocale();
  return (
    <ItemDetailsDialog ariaLabel={reminder.title} onClose={onClose}>
      <p>{reminder.title}</p>
      <p className="item-details-meta">{formatReminderTime(reminder.triggerAtUtc, locale)}</p>
      {reminder.audioPath && <AudioMessagePlayer audioId={reminder.id} loadAudio={commands.reminders.getAudio} />}
    </ItemDetailsDialog>
  );
}

export function RemindersView() {
  const { reminders, loaded, addReminder, addAudioReminder, deleteReminder } = useReminders();
  const now = useNow();
  const [detailsReminder, setDetailsReminder] = useState<Reminder | null>(null);
  const { t } = useLocale();

  return (
    <div className="tab-view">
      {loaded && reminders.length === 0 ? (
        <EmptyState icon={BellRing} text={t("reminders.empty")} />
      ) : (
        <ul className="reminder-list">
          {reminders.map((reminder) => (
            <ReminderRow key={reminder.id} reminder={reminder} now={now} onDelete={deleteReminder} onOpenDetails={setDetailsReminder} />
          ))}
        </ul>
      )}

      <ReminderComposer
        onCreate={(title, triggerAtUtc) => void addReminder(title, triggerAtUtc)}
        onCreateAudio={(title, triggerAtUtc, audioBase64) => void addAudioReminder(title, triggerAtUtc, audioBase64)}
      />

      {detailsReminder && <ReminderDetails reminder={detailsReminder} onClose={() => setDetailsReminder(null)} />}
    </div>
  );
}
