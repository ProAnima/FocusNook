import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";

// vitest.config.ts не включает test.globals, поэтому RTL не находит глобальный
// afterEach сам и не чистит DOM между тестами — регистрируем явно.
afterEach(() => {
  cleanup();
});

// jsdom не реализует matchMedia — нужен только useSystemTheme (ThemeProvider),
// который до App.test.tsx ни один тест не задевал напрямую (остальные тесты
// рендерят отдельные компоненты, минуя ThemeProvider).
if (!window.matchMedia) {
  window.matchMedia = (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  });
}
