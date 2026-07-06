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
      },
      profiles: { list: reject, create: reject, switchTo: reject },
      planItems: {
        list: reject,
        create: reject,
        toggleDone: reject,
        cycleProgress: reject,
        toggleDeferred: reject,
        delete: reject,
      },
      notes: { list: reject, create: reject, createAudio: reject, getAudio: reject, delete: reject },
      reminders: {
        list: reject,
        create: reject,
        getCurrentAlert: reject,
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
      },
      diagnostics: { export: reject },
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
