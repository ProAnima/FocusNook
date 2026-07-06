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

  const addAudioNote = useCallback(async (base64: string) => {
    const created = await commands.notes.createAudio(base64).catch(() => null);
    if (created) setNotes((prev) => [created, ...prev]);
  }, []);

  const deleteNote = useCallback(async (id: string) => {
    const previous = notes;
    setNotes((prev) => prev.filter((note) => note.id !== id));
    await commands.notes.delete(id).catch(() => setNotes(previous));
  }, [notes]);

  return { notes, loaded, addNote, addAudioNote, deleteNote };
}
