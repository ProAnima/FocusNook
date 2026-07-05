# FocusNook Desktop

Tauri 2 + React + TypeScript desktop shell for FocusNook.

This app is the Windows-first overlay prototype:

- frameless transparent window;
- always-on-top layer control;
- global shortcut fallback;
- tray lifecycle;
- local SQLite-backed tasks, notes, and reminders;
- reminder alert window.

## Run

```powershell
npm install
npm run tauri dev
```

## Frontend Preview

```powershell
npm run dev
```

## Checks

```powershell
npm run lint
npx tsc --noEmit
npm test

cd src-tauri
cargo clippy --all-targets
cargo test
```

See the root [README](../../README.md), [LICENSE](../../LICENSE), and
[AGENTS.md](../../AGENTS.md) before making changes.
