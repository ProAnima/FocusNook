import { useRef } from "react";
import { ChevronDown, ChevronUp, Plus } from "lucide-react";
import { useLocale } from "../../shared/useLocale";
import { DropFolderButton } from "./DropFolderButton";
import type { FolderOption, FolderSelection } from "./folderModel";

export function DesktopFolderRail({
  activeGroupId,
  creating,
  folderOptions,
  onMove,
  onSelect,
  onToggleCreate,
}: {
  activeGroupId: FolderSelection;
  creating: boolean;
  folderOptions: FolderOption[];
  onMove: (noteId: string, groupId: string | null) => void;
  onSelect: (groupId: FolderSelection) => void;
  onToggleCreate: () => void;
}) {
  const listRef = useRef<HTMLDivElement>(null);
  const { t } = useLocale();

  function scroll(delta: number) {
    listRef.current?.scrollBy({ top: delta, behavior: "smooth" });
  }

  return (
    <>
      <div className="note-folder-rail-header">
        <button
          className={`icon-button note-folder-add ${creating ? "is-active" : ""}`}
          type="button"
          onClick={onToggleCreate}
          title={t("notes.createFolder")}
          aria-label={t("notes.createFolder")}
        >
          <Plus size={13} />
        </button>
      </div>
      <div className="note-folder-scroll-row">
        <button className="icon-button note-folder-scroll" type="button" onClick={() => scroll(-126)} aria-label={t("notes.previousFolder")}>
          <ChevronUp size={13} />
        </button>
      </div>
      <div className="note-folder-list" ref={listRef} aria-label={t("notes.folders")}>
        {folderOptions.map((option) => (
          <DropFolderButton
            key={option.key}
            active={activeGroupId === option.groupId}
            count={option.count}
            icon={option.icon}
            label={option.label}
            onClick={() => onSelect(option.groupId)}
            onDropNote={option.groupId === "__all" ? undefined : (id) => onMove(id, option.groupId)}
          />
        ))}
      </div>
      <div className="note-folder-scroll-row">
        <button className="icon-button note-folder-scroll" type="button" onClick={() => scroll(126)} aria-label={t("notes.nextFolder")}>
          <ChevronDown size={13} />
        </button>
      </div>
    </>
  );
}
