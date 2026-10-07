import { useCallback, useEffect, useState } from "react";

const AUDIO_LOAD_TIMEOUT_MS = 15_000;

function base64ToBlobUrl(base64: string): string {
  const bytes = Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));
  return URL.createObjectURL(new Blob([bytes], { type: "audio/webm" }));
}

/**
 * Загружает аудио по id в blob-URL с таймаутом. URL освобождается при смене
 * источника, повторе и размонтировании — в одном эффекте, поэтому утечек нет.
 */
export function useAudioSource(audioId: string, loadAudio: (id: string) => Promise<string>) {
  const [attempt, setAttempt] = useState(0);
  const [src, setSrc] = useState<string | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    let url: string | null = null;
    let cancelled = false;
    const timeout = window.setTimeout(() => {
      if (!cancelled) setError(true);
    }, AUDIO_LOAD_TIMEOUT_MS);
    loadAudio(audioId)
      .then((base64) => {
        if (cancelled) return;
        window.clearTimeout(timeout);
        url = base64ToBlobUrl(base64);
        setError(false);
        setSrc(url);
      })
      .catch(() => {
        if (!cancelled) {
          window.clearTimeout(timeout);
          setError(true);
        }
      });
    return () => {
      cancelled = true;
      window.clearTimeout(timeout);
      if (url) URL.revokeObjectURL(url);
    };
  }, [attempt, audioId, loadAudio]);

  const retry = useCallback(() => {
    setSrc(null);
    setError(false);
    setAttempt((value) => value + 1);
  }, []);

  return { src, error, retry };
}
