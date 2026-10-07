import { describe, expect, it } from "vitest";
import type { Note, NoteGroup } from "../../shared/commands";
import { buildFolderOptions, orderGroups } from "./folderModel";

const group = (id: string, name: string) => ({ id, name }) as NoteGroup;
const note = (id: string, groupId: string | null) => ({ id, groupId }) as Note;

describe("folderModel", () => {
  const groups = [group("a", "Beta"), group("b", "Alpha"), group("c", "Gamma")];

  it("sorts by name", () => {
    expect(orderGroups(groups, [], "name").map((g) => g.id)).toEqual(["b", "a", "c"]);
  });

  it("sorts by most recent note, then name", () => {
    const notes = [note("1", "c"), note("2", "a"), note("3", "c")];
    expect(orderGroups(groups, notes, "recent").map((g) => g.id)).toEqual(["c", "a", "b"]);
  });

  it("builds options with counts", () => {
    const notes = [note("1", "a"), note("2", null), note("3", "a")];
    const options = buildFolderOptions([groups[0]], notes, { all: "All", ungrouped: "None" });
    expect(options.map((o) => [o.key, o.label, o.count])).toEqual([
      ["__all", "All", 3],
      ["__ungrouped", "None", 1],
      ["a", "Beta", 2],
    ]);
    expect(options[1].groupId).toBeNull();
  });
});
