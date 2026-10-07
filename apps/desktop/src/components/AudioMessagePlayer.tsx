import { useRef, useState } from "react";
import { Pause, Play, RefreshCw } from "lucide-react";
import { useAudioSource } from "../shared/useAudioSource";
import { useLocale } from "../shared/useLocale";

function formatAudioTime(seconds: number) {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const minutes = Math.floor(seconds / 60);
  const rest = Math.floor(seconds % 60).toString().padStart(2, "0");
  return `${minutes}:${rest}`;
}

function AudioMeta({ status, onOpenDetails }: { status: string; onOpenDetails?: () => void }) {
  const { t } = useLocale();
  return (
    <div className="note-audio-meta">
      {onOpenDetails ? (
        <button className="note-audio-open" type="button" onClick={onOpenDetails}>
          {t("notes.audio")}
        </button>
      ) : (
        <span>{t("notes.audio")}</span>
      )}
      <small>{status}</small>
    </div>
  );
}

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
  const { src, error, retry } = useAudioSource(audioId, loadAudio);
  const [playing, setPlaying] = useState(false);
  const [current, setCurrent] = useState(0);
  const [duration, setDuration] = useState(0);
  const { t } = useLocale();
  const playLabel = error ? t("notes.audioRetry") : playing ? t("notes.pauseAudio") : t("notes.playAudio");

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
        title={playLabel}
        aria-label={playLabel}
      >
        {error ? <RefreshCw size={13} /> : playing ? <Pause size={13} /> : <Play size={13} />}
      </button>
      <div className="note-audio-main">
        <AudioMeta status={error ? t("notes.audioError") : src ? `${formatAudioTime(current)} / ${formatAudioTime(duration)}` : t("notes.audioLoading")} onOpenDetails={onOpenDetails} />
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
