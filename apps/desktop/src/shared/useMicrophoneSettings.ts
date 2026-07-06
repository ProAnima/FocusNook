import { useCallback, useEffect, useRef, useState } from "react";
import { commands } from "./commands";

declare global {
  interface Window {
    webkitAudioContext?: typeof AudioContext;
  }
}

export interface AudioInputDevice {
  deviceId: string;
  label: string;
}

function supportsMediaDevices() {
  return typeof navigator !== "undefined" && Boolean(navigator.mediaDevices?.enumerateDevices);
}

async function stopPermissionProbe() {
  const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
  stream.getTracks().forEach((track) => track.stop());
}

async function listAudioInputs(): Promise<AudioInputDevice[]> {
  if (!supportsMediaDevices()) return [];
  const devices = await navigator.mediaDevices.enumerateDevices();
  return devices
    .filter((device) => device.kind === "audioinput")
    .map((device, index) => ({
      deviceId: device.deviceId,
      label: device.label || `Microphone ${index + 1}`,
    }));
}

export function useMicrophoneSettings() {
  const [devices, setDevices] = useState<AudioInputDevice[]>([]);
  const [selectedDeviceId, setSelectedDeviceIdState] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [permissionNeeded, setPermissionNeeded] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testLevel, setTestLevel] = useState(0);
  const [testFailed, setTestFailed] = useState(false);
  const animationRef = useRef<number | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const nextDevices = await listAudioInputs();
      setDevices(nextDevices);
      setPermissionNeeded(nextDevices.length > 0 && nextDevices.every((device) => device.label.startsWith("Microphone ")));
    } catch {
      setDevices([]);
      setPermissionNeeded(true);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    commands.settings
      .getMicrophoneDeviceId()
      .then(setSelectedDeviceIdState)
      .catch(() => setSelectedDeviceIdState(null));
    const timer = window.setTimeout(() => void refresh(), 0);
    return () => window.clearTimeout(timer);
  }, [refresh]);

  const requestPermission = useCallback(async () => {
    setLoading(true);
    try {
      await stopPermissionProbe();
      await refresh();
      setPermissionNeeded(false);
    } catch {
      setPermissionNeeded(true);
    } finally {
      setLoading(false);
    }
  }, [refresh]);

  const stopMicrophoneTest = useCallback(() => {
    if (animationRef.current !== null) {
      window.cancelAnimationFrame(animationRef.current);
      animationRef.current = null;
    }
    streamRef.current?.getTracks().forEach((track) => track.stop());
    streamRef.current = null;
    void audioContextRef.current?.close();
    audioContextRef.current = null;
    setTesting(false);
    setTestLevel(0);
  }, []);

  const startMicrophoneTest = useCallback(async () => {
    stopMicrophoneTest();
    setTestFailed(false);
    try {
      const audio = selectedDeviceId ? { deviceId: { exact: selectedDeviceId } } : true;
      const stream = await navigator.mediaDevices.getUserMedia({ audio });
      const AudioContextConstructor = window.AudioContext ?? window.webkitAudioContext;
      if (!AudioContextConstructor) throw new Error("AudioContext unavailable");
      const context = new AudioContextConstructor();
      const analyser = context.createAnalyser();
      const source = context.createMediaStreamSource(stream);
      const buffer = new Uint8Array(analyser.fftSize);
      analyser.smoothingTimeConstant = 0.78;
      source.connect(analyser);
      streamRef.current = stream;
      audioContextRef.current = context;
      setTesting(true);

      function tick() {
        analyser.getByteTimeDomainData(buffer);
        let peak = 0;
        for (const value of buffer) {
          peak = Math.max(peak, Math.abs(value - 128));
        }
        setTestLevel(Math.min(1, peak / 72));
        animationRef.current = window.requestAnimationFrame(tick);
      }
      tick();
    } catch {
      setTestFailed(true);
      stopMicrophoneTest();
    }
  }, [selectedDeviceId, stopMicrophoneTest]);

  const toggleMicrophoneTest = useCallback(async () => {
    if (testing) {
      stopMicrophoneTest();
    } else {
      await startMicrophoneTest();
    }
  }, [startMicrophoneTest, stopMicrophoneTest, testing]);

  const setSelectedDeviceId = useCallback(async (deviceId: string | null) => {
    setSelectedDeviceIdState(deviceId);
    try {
      await commands.settings.setMicrophoneDeviceId(deviceId);
    } catch {
      setSelectedDeviceIdState(null);
    }
  }, []);

  useEffect(() => stopMicrophoneTest, [stopMicrophoneTest]);

  return {
    devices,
    selectedDeviceId,
    loading,
    permissionNeeded,
    testing,
    testFailed,
    testLevel,
    refresh,
    requestPermission,
    setSelectedDeviceId,
    toggleMicrophoneTest,
  };
}
