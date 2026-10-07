import { useMemo } from "react";
import type { Note, NoteFolderSort, NoteGroup } from "../../shared/commands";
import { useLocale } from "../../shared/useLocale";
import { buildFolderOptions, orderGroups, type FolderOption } from "./folderModel";

export function useFolderOptions(groups: NoteGroup[], notes: Note[], sort: NoteFolderSort): FolderOption[] {
  const { t } = useLocale();
  const all = t("notes.all");
  const ungrouped = t("notes.ungrouped");
  return useMemo(
    () => buildFolderOptions(orderGroups(groups, notes, sort), notes, { all, ungrouped }),
    [all, groups, notes, sort, ungrouped],
  );
}
