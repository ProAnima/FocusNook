import js from "@eslint/js";
import globals from "globals";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist", "src-tauri/target", "node_modules"] },
  {
    files: ["src/**/*.{ts,tsx}"],
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.browser,
    },
    plugins: {
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": [
        "warn",
        { allowConstantExport: true },
      ],
      // Бюджеты кода из AGENTS.md — ориентир, а не догма, поэтому warn, не error.
      "max-lines": [
        "warn",
        { max: 300, skipBlankLines: true, skipComments: true },
      ],
      "max-lines-per-function": [
        "warn",
        { max: 40, skipBlankLines: true, skipComments: true },
      ],
      // Граница слоёв из AGENTS.md: Tauri API — только через src/shared/commands.ts.
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "@tauri-apps/api/core",
              message:
                "Компоненты не вызывают invoke() напрямую — используй commandClient из src/shared/commands.ts.",
            },
            {
              name: "@tauri-apps/plugin-store",
              message:
                "Store используется только внутри src/shared/commands.ts.",
            },
            {
              name: "@tauri-apps/api/window",
              message:
                "Window API — только внутри src/shared/commands.ts.",
            },
            {
              name: "@tauri-apps/api/event",
              message:
                "listen() — только внутри src/shared/commands.ts (или shared/use*-хуков).",
            },
            {
              name: "@tauri-apps/plugin-autostart",
              message:
                "Autostart plugin — только внутри src/shared/commands.ts.",
            },
          ],
        },
      ],
    },
  },
  {
    // Единственное место, которому разрешено касаться Tauri API напрямую.
    files: ["src/shared/commands.ts"],
    rules: { "no-restricted-imports": "off" },
  },
  {
    files: ["**/*.test.{ts,tsx}", "src/test/**"],
    rules: { "max-lines-per-function": "off" },
  },
);
