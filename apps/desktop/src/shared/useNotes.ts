import { useCallback, useEffect, useState, type Dispatch, type SetStateAction } from "react";
import { commands, type Note, type NoteGroup } from "./commands";
import { useEventSubscription } from "./useEventSubscription";

function useNotesData() {
  const [notes, setNotes] = useState<Note[]>([]);
  const [groups, setGroups] = useState<NoteGroup[]>([]);
  const [loaded, setLoaded] = useState(false);

  const refresh = useCallback(() => {
    Promise.all([commands.notes.list(), commands.notes.listGroups()])
      .then(([nextNotes, nextGroups]) => {
        setNotes(nextNotes);
        setGroups(nextGroups);
      })
      .catch(() => {
        setNotes([]);
        setGroups([]);
      })
      .finally(() => setLoaded(true));
  }, []);

  useEffect(() => refresh(), [refresh]);
  useEventSubscription(commands.serverSync.onCompleted, refresh);

  return { notes, setNotes, groups, setGroups, loaded };
}

/** Оптимистично применяет `patch`, затем подменяет заметку ответом команды или откатывает. */
function useNoteEditor(notes: Note[], setNotes: Dispatch<SetStateAction<Note[]>>) {
  return useCallback(
    async (id: string, patch: Partial<Note>, run: () => Promise<Note>) => {
      const previous = notes;
      setNotes((prev) => prev.map((note) => (note.id === id ? { ...note, ...patch } : note)));
      const updated = await run().catch(() => null);
      if (updated) {
        setNotes((prev) => prev.map((note) => (note.id === id ? updated : note)));
      } else {
        setNotes(previous);
      }
    },
    [notes, setNotes],
  );
}

export function useNotes() {
  const { notes, setNotes, groups, setGroups, loaded } = useNotesData();

  const editNote = useNoteEditor(notes, setNotes);

  const prepend = useCallback((created: Note | null) => {
    if (created) setNotes((prev) => [created, ...prev]);
  }, [setNotes]);

  const addGroup = useCallback(async (name: string) => {
    const created = await commands.notes.createGroup(name).catch(() => null);
    if (created) setGroups((prev) => [...prev, created]);
    return created;
  }, [setGroups]);

  const addNote = useCallback(
    async (body: string, groupId: string | null) => prepend(await commands.notes.create(body, groupId).catch(() => null)),
    [prepend],
  );

  const addAudioNote = useCallback(
    async (base64: string, groupId: string | null) =>
      prepend(await commands.notes.createAudio(base64, groupId).catch(() => null)),
    [prepend],
  );

  const deleteNote = useCallback(async (id: string) => {
    const previous = notes;
    setNotes((prev) => prev.filter((note) => note.id !== id));
    await commands.notes.delete(id).catch(() => setNotes(previous));
  }, [notes, setNotes]);

  return {
    notes,
    groups,
    loaded,
    addGroup,
    addNote,
    addAudioNote,
    deleteNote,
    moveNoteToGroup: (id: string, groupId: string | null) =>
      editNote(id, { groupId }, () => commands.notes.moveToGroup(id, groupId)),
    updateNote: (id: string, body: string) => editNote(id, { body }, () => commands.notes.update(id, body)),
  };
}
