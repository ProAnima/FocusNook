import { invoke } from "@tauri-apps/api/core";
import type { Note, NoteGroup } from "./types";

export const notesCommands = {
  async list(): Promise<Note[]> {
    return invoke<Note[]>("list_notes");
  },
  async listGroups(): Promise<NoteGroup[]> {
    return invoke<NoteGroup[]>("list_note_groups");
  },
  async createGroup(name: string): Promise<NoteGroup> {
    return invoke<NoteGroup>("create_note_group", { name });
  },
  async create(body: string, groupId: string | null): Promise<Note> {
    return invoke<Note>("create_note", { body, groupId });
  },
  async createAudio(audioBase64: string, groupId: string | null): Promise<Note> {
    return invoke<Note>("create_audio_note", { audioBase64, groupId });
  },
  async getAudio(id: string): Promise<string> {
    return invoke<string>("get_note_audio", { id });
  },
  async moveToGroup(id: string, groupId: string | null): Promise<Note> {
    return invoke<Note>("move_note_to_group", { id, groupId });
  },
  async update(id: string, body: string): Promise<Note> {
    return invoke<Note>("update_note", { id, body });
  },
  async delete(id: string) {
    await invoke("delete_note", { id });
  },
};
