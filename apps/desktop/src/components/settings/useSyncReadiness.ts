import { useCallback, useEffect, useState } from "react";
import { commands } from "../../shared/commands";

export function useSyncReadiness() {
  const [status, setStatus] = useState<Awaited<ReturnType<typeof commands.sync.readiness>> | null>(null);
  const [failed, setFailed] = useState(false);
  const refresh = useCallback(() => {
    commands.sync
      .readiness()
      .then((nextStatus) => {
        setStatus(nextStatus);
        setFailed(false);
      })
      .catch(() => {
        setStatus(null);
        setFailed(true);
      });
  }, []);
  useEffect(() => refresh(), [refresh]);
  return { status, failed, refresh };
}
