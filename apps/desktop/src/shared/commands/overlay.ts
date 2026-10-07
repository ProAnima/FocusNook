import { currentMonitor, cursorPosition, getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { physicalCursorToClient } from "../cursorCoordinates";
import { safeListen } from "./safeListen";
import type { CursorClientPosition, FolderRailSide, ResizeDirection, ShortcutInfo } from "./types";

async function resolveFolderRailSide(positionX?: number): Promise<FolderRailSide> {
  try {
    const win = getCurrentWindow();
    const [position, size, monitor] = await Promise.all([
      positionX === undefined ? win.outerPosition() : Promise.resolve({ x: positionX }),
      win.outerSize(),
      currentMonitor(),
    ]);
    const workArea = monitor?.workArea;
    if (!workArea) return "left";
    const windowCenter = position.x + size.width / 2;
    const monitorCenter = workArea.position.x + workArea.size.width / 2;
    return windowCenter < monitorCenter ? "right" : "left";
  } catch {
    return "left";
  }
}

export const overlayCommands = {
  // Rust хранит front/back как единственный источник правды и сам решает,
  // на что переключиться — и клик, и глобальный хоткей идут сюда же.
  async toggle(): Promise<boolean> {
    return invoke<boolean>("toggle_overlay_layer");
  },
  onLayerChanged(handler: (front: boolean) => void) {
    return safeListen<boolean>("layer-changed", (event) => handler(event.payload));
  },
  async getShortcutStatus(): Promise<ShortcutInfo | null> {
    return invoke<ShortcutInfo | null>("get_shortcut_status");
  },
  async isDesktop(): Promise<boolean> {
    return invoke<boolean>("is_desktop_platform");
  },
  async close() {
    // Прячет в tray (см. lib.rs::CloseRequested) — реально выходит только
    // пункт трея "Выход".
    await getCurrentWindow().close();
  },
  getFolderRailSide: resolveFolderRailSide,
  async onFolderRailSideChanged(handler: (side: FolderRailSide) => void) {
    try {
      return await getCurrentWindow().onMoved(({ payload }) => void resolveFolderRailSide(payload.x).then(handler));
    } catch {
      return () => {};
    }
  },
  async getCursorClientPosition(): Promise<CursorClientPosition> {
    const win = getCurrentWindow();
    const [cursor, position] = await Promise.all([
      cursorPosition(),
      win.outerPosition(),
    ]);
    // devicePixelRatio follows both per-monitor DPI and WebView zoom, while
    // Tauri's scaleFactor only represents the operating-system DPI.
    return physicalCursorToClient(cursor, position, window.devicePixelRatio);
  },
  async setIgnoreCursorEvents(ignore: boolean): Promise<void> {
    await getCurrentWindow().setIgnoreCursorEvents(ignore);
  },
  async startResize(direction: ResizeDirection): Promise<void> {
    await getCurrentWindow().startResizeDragging(direction);
  },
  async resizeBy(widthDelta: number, heightDelta: number): Promise<void> {
    const win = getCurrentWindow();
    const size = await win.innerSize();
    const scale = await win.scaleFactor();
    const logical = size.toLogical(scale);
    const width = Math.min(520, Math.max(280, logical.width + widthDelta));
    const height = Math.min(1200, Math.max(260, logical.height + heightDelta));
    await win.setSize(new LogicalSize(width, height));
  },
};
