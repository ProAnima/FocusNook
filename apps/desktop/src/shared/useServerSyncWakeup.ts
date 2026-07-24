import { useEffect } from "react";
import { commands } from "./commands";

const FOREGROUND_SYNC_INTERVAL_MS = 20_000;

function requestSyncIfVisible() {
  if (document.visibilityState !== "visible") return;
  void commands.serverSync.request().catch(() => {});
}

export function useServerSyncWakeup() {
  useEffect(() => {
    requestSyncIfVisible();
    const intervalId = window.setInterval(requestSyncIfVisible, FOREGROUND_SYNC_INTERVAL_MS);
    window.addEventListener("focus", requestSyncIfVisible);
    window.addEventListener("online", requestSyncIfVisible);
    document.addEventListener("visibilitychange", requestSyncIfVisible);
    return () => {
      window.clearInterval(intervalId);
      window.removeEventListener("focus", requestSyncIfVisible);
      window.removeEventListener("online", requestSyncIfVisible);
      document.removeEventListener("visibilitychange", requestSyncIfVisible);
    };
  }, []);
}
