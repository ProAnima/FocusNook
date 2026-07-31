import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Check, FolderOpen } from "lucide-react";
import type { Note, NoteGroup } from "../shared/commands";
import { useLocale } from "../shared/useLocale";

const VIEWPORT_GAP = 8;

// Positioning and dismissal live with the portal trigger so both refs share one lifecycle.
// eslint-disable-next-line max-lines-per-function
export function FolderMoveMenu({
  groups,
  note,
  onMove,
}: {
  groups: NoteGroup[];
  note: Note;
  onMove: (id: string, groupId: string | null) => void;
}) {
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState({ left: 0, top: 0 });
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const { t } = useLocale();
  const options = [{ id: null, name: t("notes.ungrouped") }, ...groups.map((group) => ({ id: group.id, name: group.name }))];

  useLayoutEffect(() => {
    if (!open || !buttonRef.current || !menuRef.current) return;
    const trigger = buttonRef.current.getBoundingClientRect();
    const menu = menuRef.current.getBoundingClientRect();
    const left = Math.min(Math.max(VIEWPORT_GAP, trigger.right - menu.width), window.innerWidth - menu.width - VIEWPORT_GAP);
    const below = trigger.bottom + 5;
    const top = below + menu.height <= window.innerHeight - VIEWPORT_GAP
      ? below
      : Math.max(VIEWPORT_GAP, trigger.top - menu.height - 5);
    setPosition({ left, top });
  }, [open, groups.length]);

  useEffect(() => {
    if (!open) return;
    function closeOutside(event: MouseEvent) {
      const target = event.target as Node;
      if (!buttonRef.current?.contains(target) && !menuRef.current?.contains(target)) setOpen(false);
    }
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }
    function closeOnResize() {
      setOpen(false);
    }
    window.addEventListener("mousedown", closeOutside);
    window.addEventListener("keydown", closeOnEscape);
    window.addEventListener("resize", closeOnResize);
    window.addEventListener("scroll", closeOnResize, true);
    return () => {
      window.removeEventListener("mousedown", closeOutside);
      window.removeEventListener("keydown", closeOnEscape);
      window.removeEventListener("resize", closeOnResize);
      window.removeEventListener("scroll", closeOnResize, true);
    };
  }, [open]);

  function move(groupId: string | null) {
    setOpen(false);
    if (groupId !== note.groupId) onMove(note.id, groupId);
  }

  return (
    <div className="note-folder-menu">
      <button
        ref={buttonRef}
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
