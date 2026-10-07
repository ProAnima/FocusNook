import type { Note, NoteFolderSort, NoteGroup } from "../../shared/commands";

export const NOTE_DRAG_TYPE = "application/x-focusnook-note-id";

export type FolderSelection = string | null | "__all";

export interface FolderOption {
  key: string;
  groupId: FolderSelection;
  icon: "all" | "folder";
  label: string;
  count: number;
}

export function orderGroups(groups: NoteGroup[], notes: Note[], sort: NoteFolderSort): NoteGroup[] {
  if (sort === "name") {
    return [...groups].sort((a, b) => a.name.localeCompare(b.name));
  }
  const latestByGroup = new Map<string, number>();
  notes.forEach((note, index) => {
    if (note.groupId && !latestByGroup.has(note.groupId)) latestByGroup.set(note.groupId, index);
  });
  return [...groups].sort((a, b) => {
    const aIndex = latestByGroup.get(a.id) ?? Number.MAX_SAFE_INTEGER;
    const bIndex = latestByGroup.get(b.id) ?? Number.MAX_SAFE_INTEGER;
    if (aIndex !== bIndex) return aIndex - bIndex;
    return a.name.localeCompare(b.name);
  });
}

export function buildFolderOptions(
  orderedGroups: NoteGroup[],
  notes: Note[],
  labels: { all: string; ungrouped: string },
): FolderOption[] {
  return [
    { key: "__all", groupId: "__all", icon: "all", label: labels.all, count: notes.length },
    {
      key: "__ungrouped",
      groupId: null,
      icon: "folder",
      label: labels.ungrouped,
      count: notes.filter((note) => note.groupId === null).length,
    },
    ...orderedGroups.map((group) => ({
      key: group.id,
      groupId: group.id as FolderSelection,
      icon: "folder" as const,
      label: group.name,
      count: notes.filter((note) => note.groupId === group.id).length,
    })),
  ];
}
