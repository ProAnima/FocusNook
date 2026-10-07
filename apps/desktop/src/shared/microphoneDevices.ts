export interface AudioInputDevice {
  deviceId: string;
  label: string;
}

const FALLBACK_LABEL_PREFIX = "Microphone ";

function supportsMediaDevices() {
  return typeof navigator !== "undefined" && Boolean(navigator.mediaDevices?.enumerateDevices);
}

export async function stopPermissionProbe() {
  const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
  stream.getTracks().forEach((track) => track.stop());
}

export function toAudioInputs(devices: Pick<MediaDeviceInfo, "deviceId" | "kind" | "label">[]): AudioInputDevice[] {
  return devices
    .filter((device) => device.kind === "audioinput")
    .map((device, index) => ({
      deviceId: device.deviceId,
      label: device.label || `${FALLBACK_LABEL_PREFIX}${index + 1}`,
    }));
}

/** Browsers hide device labels until permission is granted; unlabeled devices mean we still need it. */
export function needsMicrophonePermission(devices: AudioInputDevice[]): boolean {
  return devices.length > 0 && devices.every((device) => device.label.startsWith(FALLBACK_LABEL_PREFIX));
}

export async function listAudioInputs(): Promise<AudioInputDevice[]> {
  if (!supportsMediaDevices()) return [];
  return toAudioInputs(await navigator.mediaDevices.enumerateDevices());
}
