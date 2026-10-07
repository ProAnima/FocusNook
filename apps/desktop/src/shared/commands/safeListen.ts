import { listen } from "@tauri-apps/api/event";

export async function safeListen<T>(eventName: string, handler: (event: { payload: T }) => void): Promise<() => void> {
  try {
    return await listen<T>(eventName, handler);
  } catch {
    return () => {};
  }
}
