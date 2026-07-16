<p align="center">
  <img src="docs/assets/focusnook-readme-header.svg" alt="FocusNook — a quiet always-on-top nook for today's tasks, notes, and reminders." width="100%">
</p>

<p align="center">
  <strong>A calm always-on-top daily planning overlay for tasks, notes, reminders, voice capture, and local-first sync.</strong>
</p>

<p align="center">
  <a href="#status">Status</a> ·
  <a href="#why-focusnook">Why</a> ·
  <a href="#product-scope">Scope</a> ·
  <a href="#architecture">Architecture</a> ·
  <a href="#development">Development</a> ·
  <a href="#license">License</a>
</p>

---

## Status

FocusNook is local-first, with a working sync backbone. The first release
(v1) targets Windows desktop and Android together, syncing exclusively
through a self-hosted VDS relay server (see [apps/server](apps/server)) — no
third-party cloud provider ships or is exposed in v1.

The repository is public so the product direction, technical decisions, and
implementation quality can be reviewed early. The code is not open source and
is not licensed for commercial use, redistribution, or independent forks.

Implemented today:

- Windows desktop overlay shell with Tauri 2: transparent frameless window,
  always-on-top / send-back layer switching, global shortcut with fallback,
  autostart and tray behavior.
- Android companion app built from the same Tauri project: native
  alarm/notification plugin with boot rescheduling, touch-tuned mobile
  layout.
- Per-profile local SQLite vaults, with SQLCipher encryption and
  plaintext-vault migration support on desktop.
- Today, Notes, Reminders, Settings, profile switching, and diagnostics UI,
  shared between desktop and Android.
- Audio note capture with encrypted desktop audio blobs.
- Reminder alert window with snooze and acknowledge actions.
- VDS sync server: accounts, device linking, operation-log exchange,
  encrypted attachment sync, last-write-wins conflict resolution, admin
  monitoring console.
- i18n structure covering 10 languages.

Remaining release operations are tracked in
[docs/v1-release-plan.md](docs/v1-release-plan.md): VDS rollout and restore
evidence, permanent Android signing identity, physical-device acceptance, and
store-console submissions. Android SQLite and audio are encrypted at rest with
an Android Keystore-backed key.

Deferred past v1 — code may exist in the tree, but none of it ships:

- Google Drive and Yandex Disk direct-cloud sync adapters.
- OpenClaw / OpenClawe AI adapter.
- iOS.

## Why FocusNook

Most productivity tools ask the user to open a whole workspace, manage projects,
sort priorities, and keep a system alive.

FocusNook is intentionally smaller:

- it sits quietly near the edge of the screen;
- it keeps today visible without becoming the work itself;
- it captures a task, note, or reminder in seconds;
- it works locally first;
- it can sync later without making cloud access mandatory;
- it leaves room for future AI/OpenClaw routing and service summaries.

The product is not a project management suite. It is a small, reliable daily
nook for the things that must stay close.

## Product Scope

### Desktop

- Compact always-on-top Windows window.
- Transparent rounded edges and custom chrome.
- Remembered screen position and layer mode.
- Global shortcut for toggling front/back.
- Autostart with the system.
- Tray-first lifecycle: closing the window should hide it, not kill reminders.
- Today list with task states:
  - open;
  - done;
  - deferred;
  - partially done with percentage.
- Notes tab.
- Reminders tab.
- Reminder alert window with sound and snooze actions.

### Android

The Android app is a real companion built from the same Tauri project as
desktop (`apps/desktop/src-tauri/gen/android`), not a separate codebase:

- reminders through native alarms and notifications, including boot-time
  rescheduling — shipped;
- shared domain model and sync engine with the desktop app — shipped;
- touch-tuned mobile layout — shipped;
- audio notes use the same recording path as desktop and encrypted-at-rest
  storage backed by Android Keystore;
- microphone permission scoped to explicit voice actions, and voice-to-text
  capture — not started, deferred past v1.

### Sync

FocusNook v1 syncs exclusively through a self-hosted VDS relay server (see
[apps/server](apps/server)): accounts, device linking, an operation-log
exchange, and encrypted attachment sync.

Sync is still treated as a port internally rather than the center of the
product, so additional adapters can be added later without touching planner
logic. Partial adapter code for Google Drive and Yandex Disk already exists in
the tree, but it is intentionally parked — not wired into the v1 UI, and not
on the release critical path.

Attachment sync is documented separately in
[docs/sync-attachments.md](docs/sync-attachments.md): voice recordings are the first supported
binary payload, with the same contract reserved for future images and small text files.

## Architecture

FocusNook follows a progressive local-first architecture:

```text
UI layer
  React screens, compact widgets, overlay shell, view-models

Application layer
  use cases: create task, update progress, schedule reminder, snooze, sync

Domain layer
  entities, value objects, policies, conflict rules, validation

Infrastructure layer
  SQLite, encrypted vault, OS APIs, Tauri plugins, Android services, sync providers

Server layer
  optional VDS sync relay, auth, device registry, encrypted payload storage
```

The repository has grown past the initial spike; some of this target layout
already exists, some is still aspirational:

```text
apps/
  desktop/        Tauri 2 + React + TypeScript desktop app, including the
                  Android target (apps/desktop/src-tauri/gen/android) — shipped
  server/         self-hosted VDS sync relay — shipped, see apps/server

packages/
  ui/             planned shared UI primitives and design tokens
  i18n/           planned typed dictionaries and locale tests
  contracts/      planned TypeScript/Rust DTO contracts

crates/
  planner-core/   planned domain model and use cases
  planner-sync/   planned operation log and conflict handling
  planner-storage/planned SQLite repositories and migrations
```

## Technology

Desktop stack:

- Tauri 2.
- Rust.
- React.
- TypeScript.
- Vite.
- SQLite through Rust.

Mobile stack:

- Tauri 2 Android target, built from the same `apps/desktop` project —
  shipped.
- Native Android/Kotlin plugin for alarms, notifications, and boot
  rescheduling — shipped.
- Native microphone / speech-to-text plugins — not started, deferred past
  v1.

## Repository Layout

```text
.
├── apps/
│   ├── desktop/
│   │   ├── src/              React application
│   │   └── src-tauri/        Tauri/Rust shell (Windows + Android targets)
│   └── server/                VDS sync relay (Rust)
├── plugins/                    custom Tauri plugins (reminder alarms, ...)
├── docs/
│   ├── assets/                README and brand assets
│   ├── sync-attachments.md    attachment sync contract
│   └── v1-release-plan.md     first-release checklist
├── AGENTS.md                   AI-agent engineering contract
├── LICENSE                     restrictive source-available license
└── README.md
```

## Development

Requirements:

- Node.js.
- npm.
- Rust toolchain.
- Tauri platform prerequisites for Windows.

Install and run the desktop app:

```powershell
cd apps/desktop
npm install
npm run tauri dev
```

Frontend-only preview:

```powershell
cd apps/desktop
npm run dev
```

Production build:

```powershell
cd apps/desktop
npm run build
npm run tauri build
```

## Verification

Before considering a change ready:

```powershell
cd apps/desktop
npm run lint
npx tsc --noEmit
npm test

cd src-tauri
cargo clippy --all-targets
cargo test
```

Native overlay behavior must also be checked manually on Windows:

- transparent frameless window;
- drag region;
- always-on-top toggle;
- global shortcut fallback;
- tray lifecycle;
- autostart;
- multi-monitor positioning.

## Privacy And Security Direction

FocusNook is designed around a few non-negotiables:

- local-first data ownership;
- encrypted profile vaults before production sync;
- encrypted desktop audio note blobs;
- OAuth tokens stored in OS-backed secure storage;
- no raw task/note/reminder content in crash logs;
- no silent telemetry upload;
- AI/OpenClaw adapters behind explicit user consent;
- strict Tauri capabilities per window.

## Roadmap

A detailed, working checklist for the first release lives in
[docs/v1-release-plan.md](docs/v1-release-plan.md). Summary:

### Shipped

- Desktop overlay shell: transparent window, layer toggle, global shortcut,
  tray, autostart, multi-monitor window state.
- Android: native alarm/notification plugin with boot rescheduling,
  touch-tuned mobile layout.
- Profiles, Today/Notes/Reminders on local SQLite, reminder alert window,
  local diagnostics.
- Operation log with HLC ordering and last-write-wins conflict resolution.
- VDS sync server: accounts, device linking, operation exchange, encrypted
  attachment sync, admin monitoring console.
- i18n structure covering 10 languages.

### Required for v1 (first release)

- Create and back up the permanent Android release signing identity.
- Deploy the prepared VDS migration, nginx routes, privacy policy, and user
  agreement with approved operator details.
- VDS production hardening: rotate the bootstrap admin token, verify a real
  off-box backup restore, and add external uptime monitoring.
- One recorded end-to-end desktop + Android sync pass against the same VDS
  server.
- Signed-candidate acceptance on Huawei/HMS and RuStore devices, real store
  screenshots, owner declarations, and moderation approval.

### Deferred past v1

- Google Drive and Yandex Disk direct sync adapters — OAuth and journal code
  exist in the tree but are parked, not wired into the shipped UI.
- OpenClaw / OpenClawe AI adapter, voice-to-text, universal quick capture,
  AI intent confirmation.
- iOS.
- `planner-core` / `planner-sync` / `planner-storage` crate extraction.

## Contributing

Public issues and focused pull requests are welcome at the discretion of
ProAnima Studio.

By submitting a contribution, you agree that ProAnima Studio may use, modify,
publish, distribute, and commercialize that contribution under the terms
described in the license.

Please read [AGENTS.md](AGENTS.md) before making code changes. It defines the
engineering constraints for this repository.

## License

This repository is public but **not open source**.

FocusNook is distributed under the
[FocusNook Source-Available License 1.0](LICENSE).

You may view and privately evaluate the code. You may not use it commercially,
redistribute it, publish independent forks, or create derivative products
without prior written permission from ProAnima Studio.
