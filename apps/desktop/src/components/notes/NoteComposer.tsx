import { Mic, Plus, Square } from "lucide-react";
import { useAudioRecorder } from "../../shared/useAudioRecorder";
import { useLocale } from "../../shared/useLocale";
import { useSelectedMicrophone } from "../../shared/useSelectedMicrophone";

export function NoteComposer({
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
  const { selectedDeviceId } = useSelectedMicrophone();
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
