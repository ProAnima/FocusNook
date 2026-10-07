declare global {
  interface Window {
    webkitAudioContext?: typeof AudioContext;
  }
}

/** Normalised 0..1 loudness from an 8-bit time-domain buffer (silence sits at 128). */
export function peakLevel(buffer: Uint8Array): number {
  let peak = 0;
  for (const value of buffer) {
    peak = Math.max(peak, Math.abs(value - 128));
  }
  return Math.min(1, peak / 72);
}

/**
 * Opens the microphone and reports its level on every animation frame.
 * Resolves to a function that releases the stream, audio context and frame loop.
 */
export async function openLevelMeter(deviceId: string | null, onLevel: (level: number) => void): Promise<() => void> {
  const audio = deviceId ? { deviceId: { exact: deviceId } } : true;
  const stream = await navigator.mediaDevices.getUserMedia({ audio });
  let context: AudioContext | null = null;
  try {
    const AudioContextConstructor = window.AudioContext ?? window.webkitAudioContext;
    if (!AudioContextConstructor) throw new Error("AudioContext unavailable");
    context = new AudioContextConstructor();
    return startMeter(stream, context, onLevel);
  } catch (error) {
    // Любой сбой после получения потока не должен оставлять микрофон включённым.
    stream.getTracks().forEach((track) => track.stop());
    void context?.close();
    throw error;
  }
}

function startMeter(stream: MediaStream, context: AudioContext, onLevel: (level: number) => void): () => void {
  const analyser = context.createAnalyser();
  const buffer = new Uint8Array(analyser.fftSize);
  analyser.smoothingTimeConstant = 0.78;
  context.createMediaStreamSource(stream).connect(analyser);
  let frame: number | null = null;

  function tick() {
    analyser.getByteTimeDomainData(buffer);
    onLevel(peakLevel(buffer));
    frame = window.requestAnimationFrame(tick);
  }
  tick();

  return () => {
    if (frame !== null) window.cancelAnimationFrame(frame);
    stream.getTracks().forEach((track) => track.stop());
    void context.close();
  };
}
