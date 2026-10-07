import { useCallback, useEffect, useRef, useState } from "react";
import { commands, type Reminder } from "./commands";
import { playChime, type SoundHandle } from "./playChime";

/**
 * Состояние окна-алерта: загружает текущее напоминание и проигрывает сигнал, а затем
 * (если есть) голосовую запись. `stopPlayback` останавливает всё и отменяет
 * ещё не начавшееся воспроизведение голоса.
 */
export function useReminderAlert() {
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

  const playVoice = useCallback(async (current: Reminder) => {
    if (cancelledRef.current || !current.audioPath) return;
    const base64 = await commands.reminders.getAudio(current.id).catch(() => null);
    if (cancelledRef.current || !base64) return;
    const audio = new Audio(`data:audio/webm;base64,${base64}`);
    audioRef.current = audio;
    await audio.play().catch(() => undefined);
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
        void chime.done.then(() => playVoice(current));
      })
      .catch(() => {
        // Вне Tauri (browser-preview) текущего алерта нет — окно просто пустое.
      });
    return stopPlayback;
  }, [stopPlayback, playVoice]);

  return { reminder, stopPlayback };
}
