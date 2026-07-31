import { useEffect, useRef, useState } from "react";
import { Pause, Play, RefreshCw } from "lucide-react";
import { useLocale } from "../shared/useLocale";

const AUDIO_LOAD_TIMEOUT_MS = 15_000;

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

// The player intentionally keeps loading, retry, playback and seeking state together:
// splitting these coupled media events across components makes cleanup of blob URLs unsafe.
// eslint-disable-next-line max-lines-per-function
export function AudioMessagePlayer({
  audioId,
  loadAudio,
  onOpenDetails,
}: {
  audioId: string;
  loadAudio: (id: string) => Promise<string>;
  onOpenDetails?: () => void;
}) {
  const audioRef = useRef<HTMLAudioElement>(null);
  const [attempt, setAttempt] = useState(0);
  const [src, setSrc] = useState<string | null>(null);
  const [error, setError] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [current, setCurrent] = useState(0);
  const [duration, setDuration] = useState(0);
  const { t } = useLocale();

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

  function retry() {
    setSrc(null);
    setError(false);
    setAttempt((value) => value + 1);
  }

  function toggle() {
    const audio = audioRef.current;
    if (!audio) return;
    if (audio.paused) void audio.play();
    else audio.pause();
  }

  return (
    <div className={`note-audio-card ${error ? "is-error" : ""}`}>
      <button
        className="note-audio-play"
        type="button"
        onClick={error ? retry : toggle}
        disabled={!src && !error}
        title={error ? t("notes.audioRetry") : playing ? t("notes.pauseAudio") : t("notes.playAudio")}
        aria-label={error ? t("notes.audioRetry") : playing ? t("notes.pauseAudio") : t("notes.playAudio")}
      >
        {error ? <RefreshCw size={13} /> : playing ? <Pause size={13} /> : <Play size={13} />}
      </button>
      <div className="note-audio-main">
        <div className="note-audio-meta">
          {onOpenDetails ? (
            <button className="note-audio-open" type="button" onClick={onOpenDetails}>
              {t("notes.audio")}
            </button>
          ) : (
            <span>{t("notes.audio")}</span>
          )}
          <small>
            {error
              ? t("notes.audioError")
              : src
                ? `${formatAudioTime(current)} / ${formatAudioTime(duration)}`
                : t("notes.audioLoading")}
          </small>
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
          onChange={(event) => {
            const audio = audioRef.current;
            if (!audio) return;
            audio.currentTime = Number(event.currentTarget.value);
            setCurrent(audio.currentTime);
          }}
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
