import { useEffect, useMemo, useState } from "react";
import { NotebookPen } from "lucide-react";
import { commands, type Note, type NoteFolderSort } from "../shared/commands";
import { useNotes } from "../shared/useNotes";
import { useLocale } from "../shared/useLocale";
import { AudioMessagePlayer } from "./AudioMessagePlayer";
import { EmptyState } from "./EmptyState";
import { ItemDetailsDialog } from "./ItemDetailsDialog";
import { NoteComposer } from "./notes/NoteComposer";
import { NoteFolders } from "./notes/NoteFolders";
import { NoteRow } from "./notes/NoteRow";
import type { FolderSelection } from "./notes/folderModel";

function NoteDetails({ note, onClose }: { note: Note; onClose: () => void }) {
  const { t } = useLocale();
  return (
    <ItemDetailsDialog ariaLabel={note.body || t("notes.audio")} onClose={onClose}>
      {note.body && <p>{note.body}</p>}
      {note.audioPath && <AudioMessagePlayer audioId={note.id} loadAudio={commands.notes.getAudio} />}
    </ItemDetailsDialog>
  );
}

export function NotesView({ isDesktop = true }: { isDesktop?: boolean }) {
  const { notes, groups, loaded, addGroup, addNote, addAudioNote, moveNoteToGroup, updateNote, deleteNote } = useNotes();
  const [activeGroupId, setActiveGroupId] = useState<FolderSelection>("__all");
  const [folderSort, setFolderSort] = useState<NoteFolderSort>("recent");
  const [draft, setDraft] = useState("");
  const [detailsNote, setDetailsNote] = useState<Note | null>(null);
  const { t } = useLocale();
  const visibleNotes = useMemo(
    () => (activeGroupId === "__all" ? notes : notes.filter((note) => note.groupId === activeGroupId)),
    [activeGroupId, notes],
  );
  const composerGroupId = activeGroupId === "__all" ? null : activeGroupId;

  useEffect(() => {
    commands.settings.getNoteFolderSort().then(setFolderSort).catch(() => setFolderSort("recent"));
  }, []);

  function handleSubmit() {
    const body = draft.trim();
    if (!body) return;
    setDraft("");
    void addNote(body, composerGroupId);
  }

  return (
    <div className="tab-view notes-shell">
      <NoteFolders
        activeGroupId={activeGroupId}
        groups={groups}
        isDesktop={isDesktop}
        notes={notes}
        sort={folderSort}
        onSelect={setActiveGroupId}
        onCreate={(name) => void addGroup(name)}
        onMove={(noteId, groupId) => void moveNoteToGroup(noteId, groupId)}
      />

      {loaded && visibleNotes.length === 0 ? (
        <EmptyState icon={NotebookPen} text={t("notes.empty")} />
      ) : (
        <ul className="note-list">
          {visibleNotes.map((note) => (
            <NoteRow
              key={note.id}
              groups={groups}
              isDesktop={isDesktop}
              note={note}
              onDelete={deleteNote}
              onOpenDetails={setDetailsNote}
              onMove={(id, groupId) => void moveNoteToGroup(id, groupId)}
              onUpdate={(id, body) => void updateNote(id, body)}
            />
          ))}
        </ul>
      )}

      <NoteComposer
        draft={draft}
        onDraftChange={setDraft}
        onSubmit={handleSubmit}
        onAudioRecorded={(base64) => void addAudioNote(base64, composerGroupId)}
      />

      {detailsNote && <NoteDetails note={detailsNote} onClose={() => setDetailsNote(null)} />}
    </div>
  );
}
