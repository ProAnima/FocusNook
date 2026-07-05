import { useCallback, useEffect, useState } from "react";
import { commands, type Note } from "./commands";

export function useNotes() {
  const [notes, setNotes] = useState<Note[]>([]);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    commands.notes
      .list()
      .then(setNotes)
      .catch(() => {
        // Вне Tauri (browser-preview) список недоступен — остаёмся пустыми.
      })
      .finally(() => setLoaded(true));
  }, []);

  const addNote = useCallback(async (body: string) => {
    const created = await commands.notes.create(body).catch(() => null);
    if (created) setNotes((prev) => [created, ...prev]);
  }, []);

  return { notes, loaded, addNote };
}
