import { useCallback, useEffect, useRef, useState } from "react";
import { BellRing, Volume2 } from "lucide-react";
import { commands, type Reminder } from "../shared/commands";
import { playChime, type SoundHandle } from "../shared/playChime";
import { useLocale } from "../shared/useLocale";

function AlertActions({
  onAcknowledge,
  onSnooze,
  onSnoozeTomorrow,
}: {
  onAcknowledge: () => void;
  onSnooze: (minutes: number) => void;
  onSnoozeTomorrow: () => void;
}) {
  const { t } = useLocale();
  const primaryRef = useRef<HTMLButtonElement>(null);

  // Раздел 21 ТЗ, screen reader smoke test: это отдельное topmost-окно,
  // которое появляется без действия пользователя — без явного фокуса
  // клавиатурный/screen reader пользователь не имеет стартовой точки на нём.
  useEffect(() => {
    primaryRef.current?.focus();
  }, []);

  return (
    <div className="alert-actions">
      <button ref={primaryRef} className="alert-action alert-action-primary" onClick={onAcknowledge}>
        {t("alert.acknowledge")}
      </button>
      <button className="alert-action" onClick={() => onSnooze(10)}>
        {t("alert.snooze10")}
      </button>
      <button className="alert-action" onClick={() => onSnooze(30)}>
        {t("alert.snooze30")}
      </button>
      <button className="alert-action" onClick={onSnoozeTomorrow}>
        {t("alert.snoozeTomorrow")}
      </button>
    </div>
  );
}

export function ReminderAlert() {
  const [reminder, setReminder] = useState<Reminder | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const chimeRef = useRef<SoundHandle | null>(null);
  const cancelledRef = useRef(false);

  const stopPlayback = useCallback(() => {
    cancelledRef.current = true;
    chimeRef.current?.stop();
    if (!audioRef.current) return;
    audioRef.current.pause();
    audioRef.current.currentTime = 0;
    audioRef.current.src = "";
    audioRef.current = null;
  }, []);

  useEffect(() => {
    cancelledRef.current = false;
    commands.reminders
      .getCurrentAlert()
      .then((current) => {
        setReminder(current);
        if (!current) return;
        const chime = playChime();
        chimeRef.current = chime;
        void chime.done.then(async () => {
          if (cancelledRef.current || !current.audioPath) return;
          const base64 = await commands.reminders.getAudio(current.id).catch(() => null);
          if (cancelledRef.current || !base64) return;
          const audio = new Audio(`data:audio/webm;base64,${base64}`);
          audioRef.current = audio;
          await audio.play().catch(() => undefined);
        });
      })
      .catch(() => {
        // Вне Tauri (browser-preview) текущего алерта нет — окно просто пустое.
      });
    return stopPlayback;
  }, [stopPlayback]);

  if (!reminder) {
    return null;
  }

  const id = reminder.id;

  function snooze(minutes: number) {
    stopPlayback();
    void commands.reminders.snooze(id, new Date(Date.now() + minutes * 60_000).toISOString());
  }

  function snoozeTomorrow() {
    stopPlayback();
    const at = new Date();
    at.setDate(at.getDate() + 1);
    void commands.reminders.snooze(id, at.toISOString());
  }

  return (
    <div className="alert-shell">
      {reminder.audioPath ? <Volume2 size={18} className="alert-icon" /> : <BellRing size={18} className="alert-icon" />}
      <p className="alert-title">{reminder.title}</p>
      <AlertActions
        onAcknowledge={() => {
          stopPlayback();
          void commands.reminders.acknowledge(id);
        }}
        onSnooze={snooze}
        onSnoozeTomorrow={snoozeTomorrow}
      />
    </div>
  );
}
