import { useState, type FormEvent } from "react";
import { NotebookPen, Plus } from "lucide-react";
import { useNotes } from "../shared/useNotes";
import type { Note } from "../shared/commands";
import { EmptyState } from "./EmptyState";

function NoteRow({ note }: { note: Note }) {
  return (
    <li className="note-item">
      <span className="note-body">{note.body}</span>
    </li>
  );
}

export function NotesView() {
  const { notes, loaded, addNote } = useNotes();
  const [draft, setDraft] = useState("");

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const body = draft.trim();
    if (!body) return;
    setDraft("");
    void addNote(body);
  }

  return (
    <div className="tab-view">
      {loaded && notes.length === 0 ? (
        <EmptyState icon={NotebookPen} text="Пока нет заметок" />
      ) : (
        <ul className="note-list">
          {notes.map((note) => (
            <NoteRow key={note.id} note={note} />
          ))}
        </ul>
      )}

      <form className="quick-add" onSubmit={handleSubmit}>
        <Plus size={14} />
        <input
          placeholder="Новая заметка..."
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
        />
      </form>
    </div>
  );
}
