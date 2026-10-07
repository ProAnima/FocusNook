import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { openLevelMeter } from "./microphoneLevel";

function fakeStream() {
  const stop = vi.fn();
  return { stream: { getTracks: () => [{ stop }] } as unknown as MediaStream, stop };
}

function stubMediaDevices(stream: MediaStream) {
  const getUserMedia = vi.fn().mockResolvedValue(stream);
  Object.defineProperty(navigator, "mediaDevices", { value: { getUserMedia }, configurable: true });
  return getUserMedia;
}

function stubAudioContext(overrides: Record<string, unknown> = {}) {
  const close = vi.fn().mockResolvedValue(undefined);
  const analyser = { fftSize: 8, smoothingTimeConstant: 0, getByteTimeDomainData: (buffer: Uint8Array) => buffer.fill(128) };
  class FakeContext {
    createAnalyser = () => analyser;
    createMediaStreamSource = () => ({ connect: vi.fn() });
    close = close;
    constructor() {
      Object.assign(this, overrides);
    }
  }
  window.AudioContext = FakeContext as unknown as typeof AudioContext;
  return { close };
}

beforeEach(() => {
  vi.spyOn(window, "requestAnimationFrame").mockReturnValue(7);
  vi.spyOn(window, "cancelAnimationFrame").mockImplementation(() => undefined);
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("openLevelMeter", () => {
  it("requests the selected device and releases stream, context and frame loop", async () => {
    const { stream, stop } = fakeStream();
    const getUserMedia = stubMediaDevices(stream);
    const { close } = stubAudioContext();

    const onLevel = vi.fn();
    const release = await openLevelMeter("mic-1", onLevel);

    expect(getUserMedia).toHaveBeenCalledWith({ audio: { deviceId: { exact: "mic-1" } } });
    expect(onLevel).toHaveBeenCalledWith(0);

    release();
    expect(window.cancelAnimationFrame).toHaveBeenCalledWith(7);
    expect(stop).toHaveBeenCalledOnce();
    expect(close).toHaveBeenCalledOnce();
  });

  it("stops the stream when no AudioContext is available", async () => {
    const { stream, stop } = fakeStream();
    stubMediaDevices(stream);
    window.AudioContext = undefined as unknown as typeof AudioContext;
    window.webkitAudioContext = undefined;

    await expect(openLevelMeter(null, vi.fn())).rejects.toThrow("AudioContext unavailable");
    expect(stop).toHaveBeenCalledOnce();
  });

  it("stops the stream and closes the context when graph setup throws", async () => {
    const { stream, stop } = fakeStream();
    stubMediaDevices(stream);
    const { close } = stubAudioContext({
      createMediaStreamSource: () => {
        throw new Error("graph failed");
      },
    });

    await expect(openLevelMeter(null, vi.fn())).rejects.toThrow("graph failed");
    expect(stop).toHaveBeenCalledOnce();
    expect(close).toHaveBeenCalledOnce();
  });
});
