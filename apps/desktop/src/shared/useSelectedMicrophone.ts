import { useCallback, useEffect, useState } from "react";
import { commands } from "./commands";

/** Выбранный микрофон (сохраняется в настройках) без перечисления устройств и теста уровня. */
export function useSelectedMicrophone() {
  const [selectedDeviceId, setSelectedDeviceIdState] = useState<string | null>(null);

  useEffect(() => {
    commands.settings
      .getMicrophoneDeviceId()
      .then(setSelectedDeviceIdState)
      .catch(() => setSelectedDeviceIdState(null));
  }, []);

  const setSelectedDeviceId = useCallback(async (deviceId: string | null) => {
    setSelectedDeviceIdState(deviceId);
    try {
      await commands.settings.setMicrophoneDeviceId(deviceId);
    } catch {
      setSelectedDeviceIdState(null);
    }
  }, []);

  return { selectedDeviceId, setSelectedDeviceId };
}
