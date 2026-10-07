import { useState, type DragEvent } from "react";
import { Folder, Inbox } from "lucide-react";
import { NOTE_DRAG_TYPE } from "./folderModel";

export function DropFolderButton({
  active,
  count,
  icon,
  label,
  onClick,
  onDropNote,
}: {
  active: boolean;
  count: number;
  icon: "all" | "folder";
  label: string;
  onClick: () => void;
  onDropNote?: (noteId: string) => void;
}) {
  const [dragOver, setDragOver] = useState(false);

  function handleDrop(event: DragEvent<HTMLButtonElement>) {
    event.preventDefault();
    setDragOver(false);
    if (!onDropNote) return;
    const noteId = event.dataTransfer.getData(NOTE_DRAG_TYPE) || event.dataTransfer.getData("text/plain");
    if (noteId) onDropNote(noteId);
  }

  function handleDragOver(event: DragEvent<HTMLButtonElement>) {
    if (!onDropNote) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    setDragOver(true);
  }

  return (
    <button
      className={`note-folder-chip ${active ? "is-active" : ""} ${dragOver ? "is-drop-target" : ""}`}
      type="button"
      title={label}
      aria-label={label}
      onClick={onClick}
      onDragOver={handleDragOver}
      onDragLeave={() => setDragOver(false)}
      onDrop={handleDrop}
    >
      {icon === "all" ? <Inbox size={13} /> : <Folder size={13} />}
      <span className="note-folder-label">{label}</span>
      <small>{count}</small>
    </button>
  );
}
