import { useCallback, useEffect, useRef, useState } from "react";
import { useLocale } from "./useLocale";

// Голосовая заметка, не подкаст — 5 минут с запасом покрывает реальный
// сценарий использования и держит decoded-размер (см. notes.rs::MAX_AUDIO_BYTES)
// в разумных границах без явного ограничения пользователем.
const MAX_RECORDING_MS = 5 * 60 * 1000;

function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onloadend = () => {
      const result = reader.result as string;
      resolve(result.slice(result.indexOf(",") + 1));
    };
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}

function attachHandlers(
  recorder: MediaRecorder,
  stream: MediaStream,
  chunks: Blob[],
  onRecorded: (base64: string) => void,
) {
  recorder.ondataavailable = (event) => {
    if (event.data.size > 0) chunks.push(event.data);
  };
  recorder.onstop = () => {
    stream.getTracks().forEach((track) => track.stop());
    const blob = new Blob(chunks, { type: "audio/webm" });
    void blobToBase64(blob).then(onRecorded);
  };
}

// Раздел 16 ТЗ: явный старт/стоп записи, никакой фоновой или автоматической
// записи — пользователь всегда видит активную запись и сам её завершает.
export function useAudioRecorder(onRecorded: (base64: string) => void, deviceId: string | null = null) {
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const recorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const timerRef = useRef<number | null>(null);
  const { t } = useLocale();

  // Общий путь для ручной остановки, автостопа по таймауту и unmount-очистки
  // — везде одно и то же действие: снять таймер и остановить recorder.
  const stopRecording = useCallback(() => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    recorderRef.current?.stop();
    setRecording(false);
  }, []);

  const start = useCallback(async () => {
    setError(null);
    try {
      const audio = deviceId ? { deviceId: { exact: deviceId } } : true;
      const stream = await navigator.mediaDevices.getUserMedia({ audio });
      const recorder = new MediaRecorder(stream);
      chunksRef.current = [];
      attachHandlers(recorder, stream, chunksRef.current, onRecorded);
      recorder.start();
      recorderRef.current = recorder;
      setRecording(true);
      timerRef.current = window.setTimeout(stopRecording, MAX_RECORDING_MS);
    } catch {
      setError(t("notes.micUnavailable"));
    }
  }, [deviceId, onRecorded, stopRecording, t]);

  // Размонтирование во время активной записи (переключение вкладки/профиля)
  // не должно оставлять висящий MediaRecorder с открытым потоком микрофона.
  useEffect(() => stopRecording, [stopRecording]);

  return { recording, error, start, stop: stopRecording };
}
