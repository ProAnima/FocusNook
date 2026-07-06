import { useCallback, useEffect, useState } from "react";
import { commands, type Reminder } from "./commands";

export function useReminders() {
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    commands.reminders
      .list()
      .then(setReminders)
      .catch(() => {
        // Вне Tauri (browser-preview) список недоступен — остаёмся пустыми.
      })
      .finally(() => setLoaded(true));
  }, []);

  const addReminder = useCallback(async (title: string, triggerAtUtc: string) => {
    const created = await commands.reminders.create(title, triggerAtUtc).catch(() => null);
    if (created) {
      setReminders((prev) =>
        [...prev, created].sort((a, b) => a.triggerAtUtc.localeCompare(b.triggerAtUtc)),
      );
    }
  }, []);

  const deleteReminder = useCallback(async (id: string) => {
    const previous = reminders;
    setReminders((prev) => prev.filter((reminder) => reminder.id !== id));
    await commands.reminders.delete(id).catch(() => setReminders(previous));
  }, [reminders]);

  return { reminders, loaded, addReminder, deleteReminder };
}
