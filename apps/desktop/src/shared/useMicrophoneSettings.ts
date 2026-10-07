import { useMicrophoneDevices } from "./useMicrophoneDevices";
import { useMicrophoneTest } from "./useMicrophoneTest";
import { useSelectedMicrophone } from "./useSelectedMicrophone";

/** Полные настройки микрофона для экрана настроек: выбор, список устройств, разрешение, тест уровня. */
export function useMicrophoneSettings() {
  const { selectedDeviceId, setSelectedDeviceId } = useSelectedMicrophone();
  const { devices, loading, permissionNeeded, refresh, requestPermission } = useMicrophoneDevices();
  const { testing, testFailed, testLevel, toggleMicrophoneTest } = useMicrophoneTest(selectedDeviceId);

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
