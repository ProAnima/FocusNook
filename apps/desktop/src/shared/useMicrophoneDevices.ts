import { useCallback, useEffect, useState } from "react";
import {
  listAudioInputs,
  needsMicrophonePermission,
  stopPermissionProbe,
  type AudioInputDevice,
} from "./microphoneDevices";

export function useMicrophoneDevices() {
  const [devices, setDevices] = useState<AudioInputDevice[]>([]);
  const [loading, setLoading] = useState(false);
  const [permissionNeeded, setPermissionNeeded] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const nextDevices = await listAudioInputs();
      setDevices(nextDevices);
      setPermissionNeeded(needsMicrophonePermission(nextDevices));
    } catch {
      setDevices([]);
      setPermissionNeeded(true);
    } finally {
      setLoading(false);
    }
  }, []);

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

  useEffect(() => {
    const timer = window.setTimeout(() => void refresh(), 0);
    return () => window.clearTimeout(timer);
  }, [refresh]);

  return { devices, loading, permissionNeeded, refresh, requestPermission };
}
