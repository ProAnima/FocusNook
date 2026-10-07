import { useCallback, useState } from "react";
import { ChevronUp, FolderOpen, Inbox } from "lucide-react";
import type { Note, NoteFolderSort, NoteGroup } from "../../shared/commands";
import { useLocale } from "../../shared/useLocale";
import { DesktopFolderRail } from "./DesktopFolderRail";
import { FolderCreateForm } from "./FolderCreateForm";
import { MobileFolderSheet } from "./MobileFolderSheet";
import type { FolderOption, FolderSelection } from "./folderModel";
import { useFolderOptions } from "./useFolderOptions";
import { useMobileSheetLock } from "./useMobileSheetLock";
import { useNoteFolderCreate } from "./useNoteFolderCreate";

function MobileFolderToggle({ folder, onOpen }: { folder: FolderOption; onOpen: () => void }) {
  const { t } = useLocale();
  return (
    <button className="note-folder-mobile-toggle" type="button" onClick={onOpen} aria-label={t("notes.folders")}>
      {folder.icon === "all" ? <Inbox size={17} /> : <FolderOpen size={17} />}
      <span>{folder.label}</span>
      <small>{folder.count}</small>
      <ChevronUp size={16} />
    </button>
  );
}

export function NoteFolders({
  activeGroupId,
  groups,
  isDesktop,
  notes,
  sort,
  onSelect,
  onCreate,
  onMove,
}: {
  activeGroupId: FolderSelection;
  groups: NoteGroup[];
  isDesktop: boolean;
  notes: Note[];
  sort: NoteFolderSort;
  onSelect: (groupId: FolderSelection) => void;
  onCreate: (name: string) => void;
  onMove: (noteId: string, groupId: string | null) => void;
}) {
  const [mobileOpen, setMobileOpen] = useState(false);
  const { draft, setDraft, creating, submit, cancelCreate, toggleCreating } = useNoteFolderCreate(onCreate);
  const folderOptions = useFolderOptions(groups, notes, sort);
  const activeFolder = folderOptions.find((option) => option.groupId === activeGroupId) ?? folderOptions[0];
  const closeMobileSheet = useCallback(() => {
    cancelCreate();
    setMobileOpen(false);
  }, [cancelCreate]);

  useMobileSheetLock(!isDesktop && mobileOpen, closeMobileSheet);

  return (
    <aside className="note-folder-rail">
      {isDesktop ? (
        <DesktopFolderRail
          activeGroupId={activeGroupId}
          creating={creating}
          folderOptions={folderOptions}
          onMove={onMove}
          onSelect={onSelect}
          onToggleCreate={toggleCreating}
        />
      ) : (
        <MobileFolderToggle folder={activeFolder} onOpen={() => setMobileOpen(true)} />
      )}
      {isDesktop && creating && (
        <FolderCreateForm
          className="note-folder-create"
          draft={draft}
          iconSize={13}
          actionIconSize={12}
          onCancel={cancelCreate}
          onChange={setDraft}
          onSubmit={submit}
        />
      )}
      {!isDesktop && mobileOpen && (
        <MobileFolderSheet
          activeFolder={activeFolder}
          activeGroupId={activeGroupId}
          creating={creating}
          draft={draft}
          folderOptions={folderOptions}
          onCancelCreate={cancelCreate}
          onClose={closeMobileSheet}
          onDraftChange={setDraft}
          onSelect={(groupId) => {
            onSelect(groupId);
            closeMobileSheet();
          }}
          onSubmit={submit}
          onToggleCreate={toggleCreating}
        />
      )}
    </aside>
  );
}
