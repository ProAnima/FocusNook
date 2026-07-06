import { useCallback, useEffect, useMemo, useRef, useState, type DragEvent, type FormEvent } from "react";
import {
  Check,
  ChevronLeft,
  ChevronRight,
  Folder,
  FolderOpen,
  GripVertical,
  Inbox,
  Mic,
  NotebookPen,
  Pause,
  Pencil,
  Play,
  Plus,
  Square,
  Trash2,
  X,
} from "lucide-react";
import { commands, type Note, type NoteGroup } from "../shared/commands";
import { useNotes } from "../shared/useNotes";
import { useAudioRecorder } from "../shared/useAudioRecorder";
import { useLocale } from "../shared/useLocale";
import { useMicrophoneSettings } from "../shared/useMicrophoneSettings";
import { useOutsideClick } from "../shared/useOutsideClick";
import { EmptyState } from "./EmptyState";

const NOTE_DRAG_TYPE = "application/x-focusnook-note-id";

function base64ToBlobUrl(base64: string): string {
  const bytes = Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));
  return URL.createObjectURL(new Blob([bytes], { type: "audio/webm" }));
}

function formatAudioTime(seconds: number) {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const minutes = Math.floor(seconds / 60);
  const rest = Math.floor(seconds % 60).toString().padStart(2, "0");
  return `${minutes}:${rest}`;
}

function AudioNotePlayer({ noteId }: { noteId: string }) {
  const audioRef = useRef<HTMLAudioElement>(null);
  const [src, setSrc] = useState<string | null>(null);
  const [playing, setPlaying] = useState(false);
  const [current, setCurrent] = useState(0);
  const [duration, setDuration] = useState(0);
  const { t } = useLocale();

  useEffect(() => {
    let url: string | null = null;
    let cancelled = false;
    commands.notes
      .getAudio(noteId)
      .then((base64) => {
        if (cancelled) return;
        url = base64ToBlobUrl(base64);
        setSrc(url);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      if (url) URL.revokeObjectURL(url);
    };
  }, [noteId]);

  function toggle() {
    const audio = audioRef.current;
    if (!audio) return;
    if (audio.paused) {
      void audio.play();
    } else {
      audio.pause();
    }
  }

  function seek(value: string) {
    const audio = audioRef.current;
    if (!audio) return;
    audio.currentTime = Number(value);
    setCurrent(audio.currentTime);
  }

  return (
    <div className="note-audio-card">
      <button
        className="note-audio-play"
        type="button"
        onClick={toggle}
        disabled={!src}
        title={playing ? t("notes.pauseAudio") : t("notes.playAudio")}
        aria-label={playing ? t("notes.pauseAudio") : t("notes.playAudio")}
      >
        {playing ? <Pause size={13} /> : <Play size={13} />}
      </button>
      <div className="note-audio-main">
        <div className="note-audio-meta">
          <span>{t("notes.audio")}</span>
          <small>{src ? `${formatAudioTime(current)} / ${formatAudioTime(duration)}` : t("notes.audioLoading")}</small>
        </div>
        <input
          className="note-audio-progress"
          type="range"
          min="0"
          max={duration || 0}
          step="0.1"
          value={Math.min(current, duration || current)}
          disabled={!src}
          aria-label={t("notes.audioProgress")}
          onChange={(event) => seek(event.currentTarget.value)}
        />
      </div>
      {src && (
        <audio
          ref={audioRef}
          src={src}
          onPlay={() => setPlaying(true)}
          onPause={() => setPlaying(false)}
          onEnded={() => setPlaying(false)}
          onLoadedMetadata={(event) => setDuration(event.currentTarget.duration || 0)}
          onTimeUpdate={(event) => setCurrent(event.currentTarget.currentTime)}
        />
      )}
    </div>
  );
}

function NoteEditor({
  initialBody,
  onCancel,
  onSave,
}: {
  initialBody: string;
  onCancel: () => void;
  onSave: (body: string) => void;
}) {
  const [value, setValue] = useState(initialBody);
  const { t } = useLocale();
  const canSave = Boolean(value.trim()) && value.trim() !== initialBody;

  function save() {
    const body = value.trim();
    if (body) onSave(body);
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    save();
  }

  return (
    <form className="note-editor" onSubmit={submit}>
      <textarea
        value={value}
        autoFocus
        onChange={(event) => setValue(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") onCancel();
          if ((event.ctrlKey || event.metaKey) && event.key === "Enter") save();
        }}
      />
      <div className="note-editor-actions">
        <button className="icon-button" type="submit" disabled={!canSave} title={t("common.save")} aria-label={t("common.save")}>
          <Check size={13} />
        </button>
        <button className="icon-button" type="button" onClick={onCancel} title={t("common.cancel")} aria-label={t("common.cancel")}>
          <X size={13} />
        </button>
      </div>
    </form>
  );
}

function FolderMoveMenu({
  groups,
  note,
  onMove,
}: {
  groups: NoteGroup[];
  note: Note;
  onMove: (id: string, groupId: string | null) => void;
}) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const { t } = useLocale();
  useOutsideClick(rootRef, () => setOpen(false));
  const options = [{ id: null, name: t("notes.ungrouped") }, ...groups.map((group) => ({ id: group.id, name: group.name }))];

  function move(groupId: string | null) {
    setOpen(false);
    if (groupId !== note.groupId) onMove(note.id, groupId);
  }

  return (
    <div className="note-folder-menu" ref={rootRef}>
      <button
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
      {open && (
        <div className="note-folder-menu-list" role="menu" aria-label={t("notes.moveToFolder")}>
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
        </div>
      )}
    </div>
  );
}

function NoteRow({
  groups,
  note,
  onDelete,
  onMove,
  onUpdate,
}: {
  groups: NoteGroup[];
  note: Note;
  onDelete: (id: string) => void;
  onMove: (id: string, groupId: string | null) => void;
  onUpdate: (id: string, body: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const { t } = useLocale();
  const draggable = !editing;
  return (
    <li className={`note-item ${note.kind === "audio" ? "is-audio" : ""} ${editing ? "is-editing" : ""}`}>
      <div
        className="note-drag-handle"
        draggable={draggable}
        title={t("notes.dragHint")}
        aria-label={t("notes.dragHint")}
        onDragStart={(event) => {
          if (!draggable) return;
          event.dataTransfer.setData(NOTE_DRAG_TYPE, note.id);
          event.dataTransfer.setData("text/plain", note.id);
          event.dataTransfer.effectAllowed = "move";
        }}
      >
        <GripVertical size={13} />
      </div>
      <div className="note-content">
        {note.kind === "audio" ? (
          <AudioNotePlayer noteId={note.id} />
        ) : editing ? (
          <NoteEditor
            initialBody={note.body}
            onCancel={() => setEditing(false)}
            onSave={(body) => {
              setEditing(false);
              onUpdate(note.id, body);
            }}
          />
        ) : (
          <span className="note-body">{note.body}</span>
        )}
      </div>
      <div className="note-item-actions">
        {!editing && <FolderMoveMenu groups={groups} note={note} onMove={onMove} />}
        {note.kind !== "audio" && !editing && (
          <button
            className="icon-button"
            type="button"
            onClick={() => setEditing(true)}
            title={t("common.edit")}
            aria-label={t("common.edit")}
          >
            <Pencil size={13} />
          </button>
        )}
        <button
          className="icon-button"
          type="button"
          onClick={() => onDelete(note.id)}
          title={t("common.delete")}
          aria-label={t("common.delete")}
        >
          <Trash2 size={13} />
        </button>
      </div>
    </li>
  );
}

function DropFolderButton({
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

  return (
    <button
      className={`note-folder-chip ${active ? "is-active" : ""} ${dragOver ? "is-drop-target" : ""}`}
      type="button"
      onClick={onClick}
      onDragOver={(event) => {
        if (!onDropNote) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = "move";
        setDragOver(true);
      }}
      onDragLeave={() => setDragOver(false)}
      onDrop={handleDrop}
    >
      {icon === "all" ? <Inbox size={13} /> : <Folder size={13} />}
      <span>{label}</span>
      <small>{count}</small>
    </button>
  );
}

function NoteFolders({
  activeGroupId,
  groups,
  notes,
  onSelect,
  onCreate,
  onMove,
}: {
  activeGroupId: string | null | "__all";
  groups: NoteGroup[];
  notes: Note[];
  onSelect: (groupId: string | null | "__all") => void;
  onCreate: (name: string) => void;
  onMove: (noteId: string, groupId: string | null) => void;
}) {
  const listRef = useRef<HTMLDivElement>(null);
  const [draft, setDraft] = useState("");
  const [creating, setCreating] = useState(false);
  const [canScrollLeft, setCanScrollLeft] = useState(false);
  const [canScrollRight, setCanScrollRight] = useState(false);
  const { t } = useLocale();

  const updateScrollState = useCallback(() => {
    const list = listRef.current;
    if (!list) return;
    setCanScrollLeft(list.scrollLeft > 2);
    setCanScrollRight(list.scrollLeft + list.clientWidth < list.scrollWidth - 2);
  }, []);

  useEffect(() => {
    const timer = window.setTimeout(updateScrollState, 0);
    const list = listRef.current;
    list?.addEventListener("scroll", updateScrollState, { passive: true });
    window.addEventListener("resize", updateScrollState);
    return () => {
      window.clearTimeout(timer);
      list?.removeEventListener("scroll", updateScrollState);
      window.removeEventListener("resize", updateScrollState);
    };
  }, [groups.length, notes.length, updateScrollState]);

  function scrollFolders(direction: -1 | 1) {
    listRef.current?.scrollBy({ left: direction * 118, behavior: "smooth" });
    window.setTimeout(updateScrollState, 180);
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    const name = draft.trim();
    if (!name) return;
    setDraft("");
    setCreating(false);
    onCreate(name);
  }

  function cancelCreate() {
    setDraft("");
    setCreating(false);
  }

  return (
    <div className="note-folders">
      <div className="note-folder-toolbar">
        <button
          className="icon-button note-folder-scroll"
          type="button"
          onClick={() => scrollFolders(-1)}
          disabled={!canScrollLeft}
          title={t("notes.previousFolder")}
          aria-label={t("notes.previousFolder")}
        >
          <ChevronLeft size={13} />
        </button>
        <div className="note-folder-list" ref={listRef} aria-label={t("notes.folders")}>
          <DropFolderButton
            active={activeGroupId === "__all"}
            count={notes.length}
            icon="all"
            label={t("notes.all")}
            onClick={() => onSelect("__all")}
          />
          <DropFolderButton
            active={activeGroupId === null}
            count={notes.filter((note) => note.groupId === null).length}
            icon="folder"
            label={t("notes.ungrouped")}
            onClick={() => onSelect(null)}
            onDropNote={(id) => onMove(id, null)}
          />
          {groups.map((group) => (
            <DropFolderButton
              key={group.id}
              active={activeGroupId === group.id}
              count={notes.filter((note) => note.groupId === group.id).length}
              icon="folder"
              label={group.name}
              onClick={() => onSelect(group.id)}
              onDropNote={(id) => onMove(id, group.id)}
            />
          ))}
        </div>
        <div className="note-folder-controls">
          <button
            className="icon-button note-folder-scroll"
            type="button"
            onClick={() => scrollFolders(1)}
            disabled={!canScrollRight}
            title={t("notes.nextFolder")}
            aria-label={t("notes.nextFolder")}
          >
            <ChevronRight size={13} />
          </button>
          <button
            className={`icon-button note-folder-add ${creating ? "is-active" : ""}`}
            type="button"
            onClick={() => setCreating((value) => !value)}
            title={t("notes.createFolder")}
            aria-label={t("notes.createFolder")}
          >
            <Plus size={13} />
          </button>
        </div>
      </div>
      {creating && (
        <form className="note-folder-create" onSubmit={submit}>
          <Folder size={13} />
          <input
            autoFocus
            placeholder={t("notes.newFolder")}
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") cancelCreate();
            }}
          />
          <div className="note-folder-create-actions">
            <button className="icon-button" type="submit" disabled={!draft.trim()} title={t("common.save")} aria-label={t("common.save")}>
              <Check size={12} />
            </button>
            <button className="icon-button" type="button" onClick={cancelCreate} title={t("common.cancel")} aria-label={t("common.cancel")}>
              <X size={12} />
            </button>
          </div>
        </form>
      )}
    </div>
  );
}

function NoteComposer({
  draft,
  onDraftChange,
  onSubmit,
  onAudioRecorded,
}: {
  draft: string;
  onDraftChange: (value: string) => void;
  onSubmit: () => void;
  onAudioRecorded: (base64: string) => void;
}) {
  const { selectedDeviceId } = useMicrophoneSettings();
  const { recording, error, start, stop } = useAudioRecorder(onAudioRecorded, selectedDeviceId);
  const { t } = useLocale();

  return (
    <>
      <form
        className="quick-add note-composer"
        onSubmit={(event) => {
          event.preventDefault();
          onSubmit();
        }}
      >
        <button className="icon-button note-submit-button" type="submit" title={t("notes.add")} aria-label={t("notes.add")}>
          <Plus size={14} />
        </button>
        <textarea
          placeholder={t("notes.newPlaceholder")}
          value={draft}
          onChange={(event) => onDraftChange(event.target.value)}
          rows={1}
          onKeyDown={(event) => {
            if (event.key !== "Enter") return;
            if (event.shiftKey) return;
            event.preventDefault();
            onSubmit();
          }}
        />
        <button
          type="button"
          className={`icon-button record-button ${recording ? "is-recording" : ""}`}
          onClick={() => (recording ? stop() : void start())}
          title={recording ? t("notes.stopRecording") : t("notes.record")}
          aria-label={recording ? t("notes.stopRecording") : t("notes.record")}
        >
          {recording ? <Square size={13} /> : <Mic size={13} />}
        </button>
      </form>
      {error && <p className="note-error">{error}</p>}
    </>
  );
}

export function NotesView() {
  const { notes, groups, loaded, addGroup, addNote, addAudioNote, moveNoteToGroup, updateNote, deleteNote } = useNotes();
  const [activeGroupId, setActiveGroupId] = useState<string | null | "__all">("__all");
  const [draft, setDraft] = useState("");
  const { t } = useLocale();
  const visibleNotes = useMemo(
    () => (activeGroupId === "__all" ? notes : notes.filter((note) => note.groupId === activeGroupId)),
    [activeGroupId, notes],
  );
  const composerGroupId = activeGroupId === "__all" ? null : activeGroupId;

  function handleSubmit() {
    const body = draft.trim();
    if (!body) return;
    setDraft("");
    void addNote(body, composerGroupId);
  }

  return (
    <div className="tab-view notes-shell">
      <NoteFolders
        activeGroupId={activeGroupId}
        groups={groups}
        notes={notes}
        onSelect={setActiveGroupId}
        onCreate={(name) => void addGroup(name)}
        onMove={(noteId, groupId) => void moveNoteToGroup(noteId, groupId)}
      />

      {loaded && visibleNotes.length === 0 ? (
        <EmptyState icon={NotebookPen} text={t("notes.empty")} />
      ) : (
        <ul className="note-list">
          {visibleNotes.map((note) => (
            <NoteRow
              key={note.id}
              groups={groups}
              note={note}
              onDelete={deleteNote}
              onMove={(id, groupId) => void moveNoteToGroup(id, groupId)}
              onUpdate={(id, body) => void updateNote(id, body)}
            />
          ))}
        </ul>
      )}

      <NoteComposer
        draft={draft}
        onDraftChange={setDraft}
        onSubmit={handleSubmit}
        onAudioRecorded={(base64) => void addAudioNote(base64, composerGroupId)}
      />
    </div>
  );
}
