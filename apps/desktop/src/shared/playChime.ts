export interface SoundHandle {
  done: Promise<void>;
  stop: () => void;
}

function silentHandle(): SoundHandle {
  return { done: Promise.resolve(), stop: () => undefined };
}

// A reminder should feel noticeable, not like an accidental UI click.
// The handle lets alert actions interrupt both the chime and any voice playback chained after it.
export function playChime(): SoundHandle {
  try {
    const ctx = new AudioContext();
    const oscillator = ctx.createOscillator();
    const gain = ctx.createGain();
    let stopped = false;
    let resolveDone: () => void = () => undefined;
    const done = new Promise<void>((resolve) => {
      resolveDone = resolve;
    });
    const startedAt = ctx.currentTime;
    const duration = 1.15;

    oscillator.type = "sine";
    oscillator.frequency.setValueAtTime(740, startedAt);
    oscillator.frequency.linearRampToValueAtTime(980, startedAt + 0.22);
    oscillator.frequency.setValueAtTime(660, startedAt + 0.42);
    oscillator.frequency.linearRampToValueAtTime(880, startedAt + 0.82);

    gain.gain.setValueAtTime(0.0001, startedAt);
    gain.gain.exponentialRampToValueAtTime(0.18, startedAt + 0.05);
    gain.gain.setValueAtTime(0.16, startedAt + 0.72);
    gain.gain.exponentialRampToValueAtTime(0.0001, startedAt + duration);

    oscillator.connect(gain);
    gain.connect(ctx.destination);
    oscillator.onended = () => {
      if (!stopped) void ctx.close();
      resolveDone();
    };
    oscillator.start(startedAt);
    oscillator.stop(startedAt + duration);

    return {
      done,
      stop: () => {
        if (stopped) return;
        stopped = true;
        try {
          oscillator.stop();
        } catch {
          // The oscillator may already be stopped by the scheduled end.
        }
        void ctx.close();
        resolveDone();
      },
    };
  } catch {
    return silentHandle();
  }
}
