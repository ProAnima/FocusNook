import { useState } from "react";
import { Mic, Plus, Square, Volume2, X } from "lucide-react";
import { getReminderPresets } from "../../shared/reminderPresets";
import { useAudioRecorder } from "../../shared/useAudioRecorder";
import { useLocale } from "../../shared/useLocale";
import { useSelectedMicrophone } from "../../shared/useSelectedMicrophone";
import { ReminderCustomTimePicker } from "./ReminderCustomTimePicker";

function ReminderPresetPicker({
  disabled,
  onPreset,
  onOpenCustom,
}: {
  disabled: boolean;
  onPreset: (computeTriggerAtUtc: () => string) => void;
  onOpenCustom: () => void;
}) {
  const { t, locale } = useLocale();
  return (
    <div className="reminder-presets">
      {getReminderPresets(locale).map((preset) => (
        <button key={preset.key} className="preset-button" onClick={() => onPreset(preset.computeTriggerAtUtc)} disabled={disabled}>
          {preset.label}
        </button>
      ))}
      <button className="preset-button" onClick={onOpenCustom} disabled={disabled}>
        {t("reminders.customTime")}
      </button>
    </div>
  );
}

function RecordButton({ recording, onStart, onStop }: { recording: boolean; onStart: () => void; onStop: () => void }) {
  const { t } = useLocale();
  const label = recording ? t("reminders.stopRecording") : t("reminders.record");
  return (
    <button
      type="button"
      className={`icon-button record-button ${recording ? "is-recording" : ""}`}
      onClick={recording ? onStop : onStart}
      title={label}
      aria-label={label}
    >
      {recording ? <Square size={13} /> : <Mic size={13} />}
    </button>
  );
}

function AudioChip({ onRemove }: { onRemove: () => void }) {
  const { t } = useLocale();
  return (
    <div className="reminder-audio-chip">
      <Volume2 size={13} />
      <span>{t("reminders.voiceReady")}</span>
      <button type="button" className="icon-button" onClick={onRemove} title={t("common.delete")} aria-label={t("common.delete")}>
        <X size={12} />
      </button>
    </div>
  );
}

interface ReminderComposerProps {
  onCreate: (title: string, triggerAtUtc: string) => void;
  onCreateAudio: (title: string, triggerAtUtc: string, audioBase64: string) => void;
}

export function ReminderComposer({ onCreate, onCreateAudio }: ReminderComposerProps) {
  const [title, setTitle] = useState("");
  const [audioBase64, setAudioBase64] = useState<string | null>(null);
  const [customOpen, setCustomOpen] = useState(false);
  const { selectedDeviceId } = useSelectedMicrophone();
  const { recording, error, start, stop } = useAudioRecorder(setAudioBase64, selectedDeviceId);
  const { t } = useLocale();
  const disabled = !(title.trim() || audioBase64) || recording;

  function submit(triggerAtUtc: string) {
    const value = title.trim() || t("reminders.voiceDefaultTitle");
    if (audioBase64) {
      onCreateAudio(value, triggerAtUtc, audioBase64);
    } else if (title.trim()) {
      onCreate(value, triggerAtUtc);
    }
    setTitle("");
    setAudioBase64(null);
    setCustomOpen(false);
  }

  return (
    <div className="reminder-composer">
      <div className="quick-add reminder-input-row">
        <Plus size={14} />
        <input placeholder={t("reminders.addPlaceholder")} value={title} onChange={(event) => setTitle(event.target.value)} />
        <RecordButton recording={recording} onStart={() => void start()} onStop={stop} />
      </div>
      {audioBase64 && <AudioChip onRemove={() => setAudioBase64(null)} />}
      {error && <p className="note-error">{error}</p>}
      {customOpen ? (
        <ReminderCustomTimePicker disabled={disabled} onSubmit={submit} onCancel={() => setCustomOpen(false)} />
      ) : (
        <ReminderPresetPicker
          disabled={disabled}
          onPreset={(computeTriggerAtUtc) => submit(computeTriggerAtUtc())}
          onOpenCustom={() => setCustomOpen(true)}
        />
      )}
    </div>
  );
}
