import { useCallback, useEffect, useState } from "react";
import { commands, type ServerSyncStatus } from "../../shared/commands";

export interface ServerAccountView {
  available: boolean;
  connected: boolean;
  accountEmail: string | null;
  displayName: string | null;
  mediaReady: boolean;
  endpoint: string | null;
}

const DISCONNECTED: ServerAccountView = {
  available: false,
  connected: false,
  accountEmail: null,
  displayName: null,
  mediaReady: false,
  endpoint: null,
};

function toView(status: ServerSyncStatus): ServerAccountView {
  return {
    available: status.available,
    connected: status.connected,
    accountEmail: status.accountEmail,
    displayName: status.displayName,
    mediaReady: status.mediaReady,
    endpoint: status.endpoint,
  };
}

const signedOut = (view: ServerAccountView): ServerAccountView => ({
  ...view,
  connected: false,
  accountEmail: null,
  displayName: null,
  mediaReady: false,
});

/** Загружает статус аккаунта и даёт `run` — обёртку мутаций с единым busy/error. */
function useAccountRunner() {
  const [account, setAccount] = useState<ServerAccountView>(DISCONNECTED);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);

  const refresh = useCallback(() => {
    commands.serverSync
      .status()
      .then((status) => setAccount(toView(status)))
      .catch(() => setAccount(DISCONNECTED));
  }, []);

  useEffect(() => refresh(), [refresh]);

  const run = useCallback(
    async (task: () => Promise<void>): Promise<boolean> => {
      setBusy(true);
      setError(false);
      try {
        await task();
        refresh();
        return true;
      } catch {
        setError(true);
        return false;
      } finally {
        setBusy(false);
      }
    },
    [refresh],
  );

  return { account, setAccount, busy, error, run };
}

export type ServerAccount = ReturnType<typeof useServerAccount>;

/** Состояние аккаунта VDS и действия над ним — компоненты только рисуют. */
export function useServerAccount() {
  const { account, setAccount, busy, error, run } = useAccountRunner();
  const apply = async (request: Promise<ServerSyncStatus>) => setAccount(toView(await request));

  return {
    account,
    busy,
    error,
    signIn: (email: string, password: string) => run(() => apply(commands.serverSync.login(email, password))),
    register: (email: string, password: string, name: string, accepted: boolean) =>
      run(() => apply(commands.serverSync.register(email, password, name, accepted))),
    disconnect: () =>
      run(async () => {
        await commands.serverSync.disconnect();
        setAccount(signedOut);
      }),
    deleteAccount: (password: string) =>
      run(async () => {
        await commands.serverSync.deleteAccount(password);
        setAccount(signedOut);
      }),
  };
}
