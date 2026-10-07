import { describe, expect, it } from "vitest";
import { needsMicrophonePermission, toAudioInputs } from "./microphoneDevices";

const info = (deviceId: string, kind: MediaDeviceKind, label: string) => ({ deviceId, kind, label });

describe("microphoneDevices", () => {
  it("keeps only audio inputs and numbers unlabeled ones", () => {
    const devices = toAudioInputs([
      info("a", "audioinput", ""),
      info("v", "videoinput", "Camera"),
      info("b", "audioinput", "USB Mic"),
    ]);
    expect(devices).toEqual([
      { deviceId: "a", label: "Microphone 1" },
      { deviceId: "b", label: "USB Mic" },
    ]);
  });

  it("needs permission only when every device lacks a real label", () => {
    expect(needsMicrophonePermission([])).toBe(false);
    expect(needsMicrophonePermission([{ deviceId: "a", label: "Microphone 1" }])).toBe(true);
    expect(
      needsMicrophonePermission([
        { deviceId: "a", label: "Microphone 1" },
        { deviceId: "b", label: "USB Mic" },
      ]),
    ).toBe(false);
  });
});
