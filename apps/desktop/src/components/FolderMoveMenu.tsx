import { createPortal } from "react-dom";
import { Check, FolderOpen } from "lucide-react";
import type { Note, NoteGroup } from "../shared/commands";
import { useAnchoredMenu } from "../shared/useAnchoredMenu";
import { useLocale } from "../shared/useLocale";

export function FolderMoveMenu({
  groups,
  note,
  onMove,
}: {
  groups: NoteGroup[];
  note: Note;
  onMove: (id: string, groupId: string | null) => void;
}) {
  const { open, setOpen, position, triggerRef, menuRef } = useAnchoredMenu(groups.length);
  const { t } = useLocale();
  const options = [{ id: null, name: t("notes.ungrouped") }, ...groups.map((group) => ({ id: group.id, name: group.name }))];

  function move(groupId: string | null) {
    setOpen(false);
    if (groupId !== note.groupId) onMove(note.id, groupId);
  }

  return (
    <div className="note-folder-menu">
      <button
        ref={triggerRef}
        className="icon-button"
        type="button"
        onClick={() => setOpen((value) => !value)}
        title={t("notes.moveToFolder")}
        aria-label={t("notes.moveToFolder")}
        aria-haspopup="menu"
        aria-expanded={open}
      >
        <FolderOpen size={13} />
      </button>
      {open &&
        createPortal(
          <div
            ref={menuRef}
            className="note-folder-menu-list"
            role="menu"
            aria-label={t("notes.moveToFolder")}
            style={{ left: position.left, top: position.top }}
          >
            {options.map((option) => (
              <button
                key={option.id ?? "__ungrouped"}
                className={`note-folder-menu-item ${option.id === note.groupId ? "is-active" : ""}`}
                type="button"
                role="menuitem"
                onClick={() => move(option.id)}
              >
                <span>{option.name}</span>
                {option.id === note.groupId && <Check size={12} />}
              </button>
            ))}
          </div>,
          document.body,
        )}
    </div>
  );
}
