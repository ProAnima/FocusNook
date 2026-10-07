import { useCallback, useEffect, useRef, useState } from "react";
import { openLevelMeter } from "./microphoneLevel";

export function useMicrophoneTest(selectedDeviceId: string | null) {
  const [testing, setTesting] = useState(false);
  const [testLevel, setTestLevel] = useState(0);
  const [testFailed, setTestFailed] = useState(false);
  const releaseRef = useRef<(() => void) | null>(null);
  // Каждый stop/unmount увеличивает поколение: ответ на уже неактуальный запрос
  // микрофона сразу освобождается, а не включает тест «в пустоту».
  const generationRef = useRef(0);

  const stopMicrophoneTest = useCallback(() => {
    generationRef.current += 1;
    releaseRef.current?.();
    releaseRef.current = null;
    setTesting(false);
    setTestLevel(0);
  }, []);

  const startMicrophoneTest = useCallback(async () => {
    stopMicrophoneTest();
    const generation = generationRef.current;
    setTestFailed(false);
    try {
      const release = await openLevelMeter(selectedDeviceId, setTestLevel);
      if (generation !== generationRef.current) {
        release();
        return;
      }
      releaseRef.current = release;
      setTesting(true);
    } catch {
      if (generation !== generationRef.current) return;
      setTestFailed(true);
      stopMicrophoneTest();
    }
  }, [selectedDeviceId, stopMicrophoneTest]);

  const toggleMicrophoneTest = useCallback(async () => {
    if (testing) return stopMicrophoneTest();
    await startMicrophoneTest();
  }, [startMicrophoneTest, stopMicrophoneTest, testing]);

  useEffect(() => stopMicrophoneTest, [stopMicrophoneTest]);

  return { testing, testLevel, testFailed, toggleMicrophoneTest };
}
