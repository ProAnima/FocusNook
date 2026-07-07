import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import App from "./App";

// Раздел 21 ТЗ: "layout snapshots для compact desktop/mobile" — снимок всей
// разметки (не отдельного компонента), чтобы неожиданные изменения в шапке/
// навигации/вкладках между desktop- и mobile-режимом были видны в одном diff.
const { isDesktop } = vi.hoisted(() => ({ isDesktop: vi.fn() }));

vi.mock("./shared/commands", () => {
  const reject = () => Promise.reject(new Error("недоступно вне Tauri"));
  return {
    commands: {
      overlay: {
        toggle: reject,
        onLayerChanged: () => Promise.resolve(() => {}),
        getShortcutStatus: () => Promise.resolve(null),
        isDesktop,
        close: reject,
        getFolderRailSide: () => Promise.resolve("left"),
        onFolderRailSideChanged: () => Promise.resolve(() => {}),
        getCursorClientPosition: () => Promise.resolve({ x: 0, y: 0 }),
        setIgnoreCursorEvents: () => Promise.resolve(),
      },
      profiles: { list: reject, create: reject, switchTo: reject },
      planItems: {
        list: reject,
        listRange: reject,
        create: reject,
        toggleDone: reject,
        cycleProgress: reject,
        toggleDeferred: reject,
        moveToDate: reject,
        rollOverPending: reject,
        delete: reject,
      },
      notes: {
        list: reject,
        listGroups: reject,
        createGroup: reject,
        create: reject,
        createAudio: reject,
        getAudio: reject,
        moveToGroup: reject,
        update: reject,
        delete: reject,
      },
      reminders: {
        onChanged: () => Promise.resolve(() => {}),
        list: reject,
        create: reject,
        createAudio: reject,
        getCurrentAlert: reject,
        getAudio: reject,
        acknowledge: reject,
        snooze: reject,
        delete: reject,
      },
      settings: {
        getTheme: () => Promise.resolve(null),
        setTheme: reject,
        getAutostart: () => Promise.resolve(false),
        setAutostart: reject,
        getLocale: () => Promise.resolve(null),
        setLocale: reject,
        getMicrophoneDeviceId: () => Promise.resolve(null),
        setMicrophoneDeviceId: reject,
        getNoteFolderSort: () => Promise.resolve("recent"),
        setNoteFolderSort: reject,
      },
      diagnostics: { export: reject },
      serverSync: {
        onCompleted: () => Promise.resolve(() => {}),
        onFailed: () => Promise.resolve(() => {}),
        status: () =>
          Promise.resolve({
            available: true,
            accountEmail: null,
            accountUserId: null,
            connected: false,
            displayName: null,
            endpoint: null,
            message: null,
          }),
        syncNow: reject,
        connectDefault: reject,
        login: reject,
        register: reject,
        disconnect: reject,
      },
    },
    isAlertWindow: () => false,
  };
});

// day-date зависит от реальной сегодняшней даты (Intl.DateTimeFormat от
// new Date()) — заменяем текст на плейсхолдер, а не замораживаем время
// фейковыми таймерами: те мешают опросу findBy* внутри testing-library.
function normalizeDynamicDate(container: HTMLElement) {
  const dateEl = container.querySelector(".day-date");
  if (dateEl) dateEl.textContent = "DATE_PLACEHOLDER";
}

describe("App layout snapshots", () => {
  it("matches the desktop shell layout", async () => {
    isDesktop.mockResolvedValue(true);
    const { container } = render(<App />);

    await screen.findByText("На сегодня пока ничего не запланировано");
    normalizeDynamicDate(container);
    expect(container).toMatchSnapshot();
  });

  it("matches the mobile shell layout", async () => {
    isDesktop.mockResolvedValue(false);
    const { container } = render(<App />);

    await screen.findByText("На сегодня пока ничего не запланировано");
    normalizeDynamicDate(container);
    expect(container).toMatchSnapshot();
  });
});
