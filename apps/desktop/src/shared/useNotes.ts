import { useCallback, useEffect, useState } from "react";
import { commands, type Note, type NoteGroup } from "./commands";

export function useNotes() {
  const [notes, setNotes] = useState<Note[]>([]);
  const [groups, setGroups] = useState<NoteGroup[]>([]);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
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

  const addGroup = useCallback(async (name: string) => {
    const created = await commands.notes.createGroup(name).catch(() => null);
    if (created) setGroups((prev) => [...prev, created]);
    return created;
  }, []);

  const addNote = useCallback(async (body: string, groupId: string | null) => {
    const created = await commands.notes.create(body, groupId).catch(() => null);
    if (created) setNotes((prev) => [created, ...prev]);
  }, []);

  const addAudioNote = useCallback(async (base64: string, groupId: string | null) => {
    const created = await commands.notes.createAudio(base64, groupId).catch(() => null);
    if (created) setNotes((prev) => [created, ...prev]);
  }, []);

  const moveNoteToGroup = useCallback(async (id: string, groupId: string | null) => {
    const previous = notes;
    setNotes((prev) => prev.map((note) => (note.id === id ? { ...note, groupId } : note)));
    const updated = await commands.notes.moveToGroup(id, groupId).catch(() => null);
    if (updated) {
      setNotes((prev) => prev.map((note) => (note.id === id ? updated : note)));
    } else {
      setNotes(previous);
    }
  }, [notes]);

  const updateNote = useCallback(async (id: string, body: string) => {
    const previous = notes;
    setNotes((prev) => prev.map((note) => (note.id === id ? { ...note, body } : note)));
    const updated = await commands.notes.update(id, body).catch(() => null);
    if (updated) {
      setNotes((prev) => prev.map((note) => (note.id === id ? updated : note)));
    } else {
      setNotes(previous);
    }
  }, [notes]);

  const deleteNote = useCallback(async (id: string) => {
    const previous = notes;
    setNotes((prev) => prev.filter((note) => note.id !== id));
    await commands.notes.delete(id).catch(() => setNotes(previous));
  }, [notes]);

  return { notes, groups, loaded, addGroup, addNote, addAudioNote, moveNoteToGroup, updateNote, deleteNote };
}
