import { useCallback, useEffect, useState } from "react";
import { commands, type Reminder } from "./commands";

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

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    commands.reminders
      .onChanged(() => refresh())
      .then((cleanup) => {
        unlisten = cleanup;
      })
      .catch(() => {});
    return () => unlisten?.();
  }, [refresh]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    commands.serverSync
      .onCompleted(() => refresh())
      .then((cleanup) => {
        unlisten = cleanup;
      })
      .catch(() => {});
    return () => unlisten?.();
  }, [refresh]);

  const addReminder = useCallback(async (title: string, triggerAtUtc: string) => {
    const created = await commands.reminders.create(title, triggerAtUtc).catch(() => null);
    if (created) setReminders((prev) => sortByTrigger([...prev, created]));
  }, []);

  const addAudioReminder = useCallback(async (title: string, triggerAtUtc: string, audioBase64: string) => {
    const created = await commands.reminders.createAudio(title, triggerAtUtc, audioBase64).catch(() => null);
    if (created) setReminders((prev) => sortByTrigger([...prev, created]));
  }, []);

  const deleteReminder = useCallback(async (id: string) => {
    const previous = reminders;
    setReminders((prev) => prev.filter((reminder) => reminder.id !== id));
    await commands.reminders.delete(id).catch(() => setReminders(previous));
  }, [reminders]);

  return { reminders, loaded, addReminder, addAudioReminder, deleteReminder };
}
