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
      // Функции-компоненты в *.tsx получают повышенный лимит ниже (JSX раздувает строки).
      "max-lines": [
        "warn",
        { max: 300, skipBlankLines: true, skipComments: true },
      ],
      "max-lines-per-function": [
        "warn",
        { max: 40, skipBlankLines: true, skipComments: true },
      ],
      // Граница слоёв из AGENTS.md: Tauri API — только через src/shared/commands/.
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "@tauri-apps/api/core",
              message:
                "Компоненты не вызывают invoke() напрямую — используй объект commands из src/shared/commands.",
            },
            {
              name: "@tauri-apps/plugin-store",
              message:
                "Store используется только внутри src/shared/commands/.",
            },
            {
              name: "@tauri-apps/api/window",
              message:
                "Window API — только внутри src/shared/commands/.",
            },
            {
              name: "@tauri-apps/api/event",
              message:
                "listen() — только внутри src/shared/commands/ (подписки на события — методы on* там же).",
            },
            {
              name: "@tauri-apps/plugin-autostart",
              message:
                "Autostart plugin — только внутри src/shared/commands/.",
            },
          ],
        },
      ],
    },
  },
  {
    // Единственное место, которому разрешено касаться Tauri API напрямую.
    files: ["src/shared/commands/**"],
    rules: { "no-restricted-imports": "off" },
  },
  {
    // Компонент — это в основном разметка: 80 строк на функцию вместо 40,
    // логику при этом выносим в хуки (use*.ts), где действует строгий лимит.
    files: ["src/**/*.tsx"],
    rules: {
      "max-lines-per-function": [
        "warn",
        { max: 80, skipBlankLines: true, skipComments: true },
      ],
    },
  },
  {
    files: ["**/*.test.{ts,tsx}", "src/test/**"],
    rules: { "max-lines-per-function": "off" },
  },
);
