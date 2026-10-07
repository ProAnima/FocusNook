import { useState, type DragEvent } from "react";
import { GripVertical, Pencil, Trash2 } from "lucide-react";
import { commands, type Note, type NoteGroup } from "../../shared/commands";
import { useHoldToConfirm } from "../../shared/useHoldToConfirm";
import { useLocale } from "../../shared/useLocale";
import { AudioMessagePlayer } from "../AudioMessagePlayer";
import { FolderMoveMenu } from "../FolderMoveMenu";
import { NoteEditor } from "./NoteEditor";
import { NOTE_DRAG_TYPE } from "./folderModel";

interface NoteRowProps {
  groups: NoteGroup[];
  isDesktop: boolean;
  note: Note;
  onDelete: (id: string) => void;
  onOpenDetails: (note: Note) => void;
  onMove: (id: string, groupId: string | null) => void;
  onUpdate: (id: string, body: string) => void;
}

function NoteContent({
  editing,
  note,
  onCancelEdit,
  onOpenDetails,
  onSave,
}: {
  editing: boolean;
  note: Note;
  onCancelEdit: () => void;
  onOpenDetails: (note: Note) => void;
  onSave: (body: string) => void;
}) {
  if (note.kind === "audio") {
    return <AudioMessagePlayer audioId={note.id} loadAudio={commands.notes.getAudio} onOpenDetails={() => onOpenDetails(note)} />;
  }
  if (editing) {
    return <NoteEditor initialBody={note.body} onCancel={onCancelEdit} onSave={onSave} />;
  }
  return (
    <button className="note-body" type="button" onClick={() => onOpenDetails(note)}>
      {note.body}
    </button>
  );
}

export function NoteRow({ groups, isDesktop, note, onDelete, onOpenDetails, onMove, onUpdate }: NoteRowProps) {
  const [editing, setEditing] = useState(false);
  const { t } = useLocale();
  const draggable = isDesktop && !editing;
  const deleteHold = useHoldToConfirm(() => onDelete(note.id));
  function startDrag(event: DragEvent<HTMLElement>) {
    if (!draggable) return;
    event.dataTransfer.setData(NOTE_DRAG_TYPE, note.id);
    event.dataTransfer.setData("text/plain", note.id);
    event.dataTransfer.effectAllowed = "move";
  }

  return (
    <li
      className={`note-item ${note.kind === "audio" ? "is-audio" : ""} ${editing ? "is-editing" : ""} ${deleteHold.holding ? "is-delete-holding" : ""}`}
    >
      {isDesktop && (
        <button
          className="note-drag-handle"
          type="button"
          draggable={draggable}
          disabled={!draggable}
          onDragStart={startDrag}
          title={t("notes.dragHint")}
          aria-label={t("notes.dragHint")}
        >
          <GripVertical size={13} />
        </button>
      )}
      <div className="note-content">
        <NoteContent
          editing={editing}
          note={note}
          onCancelEdit={() => setEditing(false)}
          onOpenDetails={onOpenDetails}
          onSave={(body) => {
            setEditing(false);
            onUpdate(note.id, body);
          }}
        />
      </div>
      <div className="note-item-actions">
        {!editing && <FolderMoveMenu groups={groups} note={note} onMove={onMove} />}
        {note.kind !== "audio" && !editing && (
          <button
            className="icon-button"
            type="button"
            onClick={() => setEditing(true)}
            title={t("common.edit")}
            aria-label={t("common.edit")}
          >
            <Pencil size={13} />
          </button>
        )}
        <button
          className="icon-button hold-delete-button"
          type="button"
          title={t("common.delete")}
          aria-label={t("common.delete")}
          {...deleteHold.buttonProps}
        >
          <Trash2 size={13} />
        </button>
      </div>
    </li>
  );
}
