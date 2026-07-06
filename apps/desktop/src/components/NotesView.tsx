import { useEffect, useState, type FormEvent } from "react";
import { Mic, NotebookPen, Plus, Square, Trash2 } from "lucide-react";
import { commands, type Note } from "../shared/commands";
import { useNotes } from "../shared/useNotes";
import { useAudioRecorder } from "../shared/useAudioRecorder";
import { useLocale } from "../shared/useLocale";
import { EmptyState } from "./EmptyState";

function base64ToBlobUrl(base64: string): string {
  const bytes = Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));
  return URL.createObjectURL(new Blob([bytes], { type: "audio/webm" }));
}

function AudioNotePlayer({ noteId }: { noteId: string }) {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    let url: string | null = null;
    let cancelled = false;
    commands.notes
      .getAudio(noteId)
      .then((base64) => {
        if (cancelled) return;
        url = base64ToBlobUrl(base64);
        setSrc(url);
      })
      .catch(() => {
        // Файл мог быть удалён вручную вне приложения — просто не показываем плеер.
      });
    return () => {
      cancelled = true;
      if (url) URL.revokeObjectURL(url);
    };
  }, [noteId]);

  return src ? <audio className="note-audio" controls src={src} /> : null;
}

function NoteRow({ note, onDelete }: { note: Note; onDelete: (id: string) => void }) {
  const { t } = useLocale();
  return (
    <li className="note-item">
      {note.kind === "audio" ? (
        <AudioNotePlayer noteId={note.id} />
      ) : (
        <span className="note-body">{note.body}</span>
      )}
      <div className="note-item-actions">
        <button
          className="icon-button"
          onClick={() => onDelete(note.id)}
          title={t("common.delete")}
          aria-label={t("common.delete")}
        >
          <Trash2 size={13} />
        </button>
      </div>
    </li>
  );
}

function NoteComposer({
  draft,
  onDraftChange,
  onSubmit,
  onAudioRecorded,
}: {
  draft: string;
  onDraftChange: (value: string) => void;
  onSubmit: (event: FormEvent) => void;
  onAudioRecorded: (base64: string) => void;
}) {
  const { recording, error, start, stop } = useAudioRecorder(onAudioRecorded);
  const { t } = useLocale();

  return (
    <>
      <form className="quick-add" onSubmit={onSubmit}>
        <Plus size={14} />
        <input
          placeholder={t("notes.newPlaceholder")}
          value={draft}
          onChange={(event) => onDraftChange(event.target.value)}
        />
        <button
          type="button"
          className={`icon-button record-button ${recording ? "is-recording" : ""}`}
          onClick={() => (recording ? stop() : void start())}
          title={recording ? t("notes.stopRecording") : t("notes.record")}
          aria-label={recording ? t("notes.stopRecording") : t("notes.record")}
        >
          {recording ? <Square size={13} /> : <Mic size={13} />}
        </button>
      </form>
      {error && <p className="note-error">{error}</p>}
    </>
  );
}

export function NotesView() {
  const { notes, loaded, addNote, addAudioNote, deleteNote } = useNotes();
  const [draft, setDraft] = useState("");
  const { t } = useLocale();

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
        <EmptyState icon={NotebookPen} text={t("notes.empty")} />
      ) : (
        <ul className="note-list">
          {notes.map((note) => (
            <NoteRow key={note.id} note={note} onDelete={deleteNote} />
          ))}
        </ul>
      )}

      <NoteComposer
        draft={draft}
        onDraftChange={setDraft}
        onSubmit={handleSubmit}
        onAudioRecorded={(base64) => void addAudioNote(base64)}
      />
    </div>
  );
}
