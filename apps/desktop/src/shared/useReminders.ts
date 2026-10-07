import { useCallback, useEffect, useState } from "react";
import { commands, type Reminder } from "./commands";
import { useEventSubscription } from "./useEventSubscription";

function sortByTrigger(reminders: Reminder[]) {
  return [...reminders].sort((a, b) => a.triggerAtUtc.localeCompare(b.triggerAtUtc));
}

export function useReminders() {
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [loaded, setLoaded] = useState(false);

  const refresh = useCallback(() => {
    commands.reminders
      .list()
      .then((next) => setReminders(sortByTrigger(next)))
      .catch(() => {})
      .finally(() => setLoaded(true));
  }, []);

  useEffect(() => refresh(), [refresh]);
  useEventSubscription(commands.reminders.onChanged, refresh);
  useEventSubscription(commands.serverSync.onCompleted, refresh);

  const insert = useCallback((created: Reminder | null) => {
    if (created) setReminders((prev) => sortByTrigger([...prev, created]));
  }, []);

  const addReminder = useCallback(
    async (title: string, triggerAtUtc: string) =>
      insert(await commands.reminders.create(title, triggerAtUtc).catch(() => null)),
    [insert],
  );

  const addAudioReminder = useCallback(
    async (title: string, triggerAtUtc: string, audioBase64: string) =>
      insert(await commands.reminders.createAudio(title, triggerAtUtc, audioBase64).catch(() => null)),
    [insert],
  );

  const deleteReminder = useCallback(async (id: string) => {
    const previous = reminders;
    setReminders((prev) => prev.filter((reminder) => reminder.id !== id));
    await commands.reminders.delete(id).catch(() => setReminders(previous));
  }, [reminders]);

  return { reminders, loaded, addReminder, addAudioReminder, deleteReminder };
}
