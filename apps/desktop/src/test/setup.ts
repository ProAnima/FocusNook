import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";

// vitest.config.ts не включает test.globals, поэтому RTL не находит глобальный
// afterEach сам и не чистит DOM между тестами — регистрируем явно.
afterEach(() => {
  cleanup();
});
