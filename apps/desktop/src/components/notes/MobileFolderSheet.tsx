import type { FormEvent } from "react";
import { Folder, Inbox, Plus, X } from "lucide-react";
import { useLocale } from "../../shared/useLocale";
import { FolderCreateForm } from "./FolderCreateForm";
import type { FolderOption, FolderSelection } from "./folderModel";

function MobileFolderList({
  activeGroupId,
  folderOptions,
  onSelect,
}: {
  activeGroupId: FolderSelection;
  folderOptions: FolderOption[];
  onSelect: (groupId: FolderSelection) => void;
}) {
  return (
    <div className="note-folder-mobile-list">
      {folderOptions.map((option) => (
        <button
          key={option.key}
          className={`note-folder-mobile-item ${activeGroupId === option.groupId ? "is-active" : ""}`}
          type="button"
          onClick={() => onSelect(option.groupId)}
        >
          {option.icon === "all" ? <Inbox size={18} /> : <Folder size={18} />}
          <span>{option.label}</span>
          <small>{option.count}</small>
        </button>
      ))}
    </div>
  );
}

export function MobileFolderSheet({
  activeFolder,
  activeGroupId,
  creating,
  draft,
  folderOptions,
  onCancelCreate,
  onClose,
  onDraftChange,
  onSelect,
  onSubmit,
  onToggleCreate,
}: {
  activeFolder: FolderOption;
  activeGroupId: FolderSelection;
  creating: boolean;
  draft: string;
  folderOptions: FolderOption[];
  onCancelCreate: () => void;
  onClose: () => void;
  onDraftChange: (value: string) => void;
  onSelect: (groupId: FolderSelection) => void;
  onSubmit: (event: FormEvent) => void;
  onToggleCreate: () => void;
}) {
  const { t } = useLocale();
  return (
    <div className="note-folder-mobile-layer" role="presentation" onClick={onClose}>
      <section
        className="note-folder-mobile-sheet"
        role="dialog"
        aria-modal="true"
        aria-label={t("notes.folders")}
        onClick={(event) => event.stopPropagation()}
      >
        <div className="note-folder-mobile-grip" />
        <header className="note-folder-mobile-header">
          <div>
            <strong>{t("notes.folders")}</strong>
            <span>{activeFolder.label}</span>
          </div>
          <div className="note-folder-mobile-actions">
            <button
              className={`icon-button note-folder-add ${creating ? "is-active" : ""}`}
              type="button"
              onClick={onToggleCreate}
              title={t("notes.createFolder")}
              aria-label={t("notes.createFolder")}
            >
              <Plus size={16} />
            </button>
            <button className="icon-button" type="button" onClick={onClose} aria-label={t("header.close")}>
              <X size={17} />
            </button>
          </div>
        </header>
        {creating && (
          <FolderCreateForm
            className="note-folder-create note-folder-create-mobile"
            draft={draft}
            iconSize={15}
            actionIconSize={14}
            onCancel={onCancelCreate}
            onChange={onDraftChange}
            onSubmit={onSubmit}
          />
        )}
        <MobileFolderList activeGroupId={activeGroupId} folderOptions={folderOptions} onSelect={onSelect} />
      </section>
    </div>
  );
}
