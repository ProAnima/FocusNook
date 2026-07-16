# FocusNook v1 release plan

Scope decision (2026-07-16): the first release ships **Windows desktop and
Android together**, syncing **exclusively through the self-hosted VDS relay**
(`apps/server`). Google Drive, Yandex Disk, the OpenClaw/OpenClawe adapter,
and iOS are explicitly out of scope for v1 — see "Deferred past v1" below.

This file is the working checklist behind the summary in
[README.md#roadmap](../README.md#roadmap). Update it as items close instead of
letting it drift the way the old Iteration 0-3 roadmap did.

## Blockers (must close before v1 ships)

1. **Owner/legal inputs and permanent Android signing identity**:
   - provide the operator name, tax id, legal/actual address, and approved
     support contact listed in `store-listing/owner-inputs.md`;
   - approve the `/privacy` and `/terms` drafts with qualified legal review;
   - create the permanent upload key, make two encrypted backups, and record
     its certificate SHA-256. No release key is stored in this repository.
2. **VDS server production rollout**, prepared but not applied without access:
   - rotate `FOCUSNOOK_ADMIN_TOKEN` after bootstrap;
   - confirm `.env`, DB dumps, and VDS snapshots live outside the repo and
     off-box;
   - add an external uptime monitor for `/healthz` and `/readyz`;
   - run `scripts/deploy-vds.sh --apply`, a real `restore-drill.sh`, and record
     public `/privacy` and `/terms` smoke evidence. The routes currently remain
     absent from the live nginx until this rollout.
3. **One recorded end-to-end desktop + Android sync pass** against the same
   live VDS server. The sync engine is shared and unit-tested on both sides,
   but this audit did not find a recorded manual pass proving a task created
   on desktop reaches Android (and back) through a real deployed server.
4. **Signed-build device and store acceptance**:
   - run the release preflight and test on a Huawei/HMS phone plus a RuStore
     representative phone, including reminder-after-reboot and account deletion;
   - capture the real screenshots in `store-listing/screenshot-plan.md`;
   - complete the age, territory, legal, tax, and content declarations in both
     developer consoles and submit the same verified AAB.

## Housekeeping (do alongside v1, not blocking)

- Document `/v1/sync/events` in `apps/server/README.md` (implemented,
  tested, previously undocumented — now added).
- `disconnect_provider` never revokes the provider-side OAuth token, only the
  local copy. Moot for v1 now that the Google Drive / Yandex Disk rows are
  hidden (see Shipped); revisit only if those adapters ship later.
- Large files past the project's own ~250-300 line budget
  (`NotesView.tsx` 705, `SettingsPanel.tsx` 664, `server_sync.rs` 2048,
  `routes.rs` 1222 lines) — worth splitting for maintainability, not a
  release blocker.

## Explicitly parked for post-v1 (code exists, not on the critical path)

- Google Drive OAuth + direct-to-Drive journal sync (`oauth.rs`, the Google
  branch of `sync.rs`, `cloud_sync.rs`), including the native Android Google
  Sign-In plugin (`plugins/tauri-plugin-google-auth`). The source remains in
  the tree behind the opt-in `cloud-providers` Cargo feature, but the default
  RuStore/AppGallery build does not compile or package it. The Settings UI
  rows for both providers were removed on the same date (see Shipped), so
  neither is reachable in v1.
- Yandex Disk OAuth (connect/disconnect work; no direct-sync adapter was ever
  built for it, unlike the Google Drive journal path).
- OpenClaw / OpenClawe AI adapter, voice-to-text, universal quick capture, AI
  intent confirmation.
- iOS (icon assets exist under `apps/desktop/src-tauri/icons/ios`; no
  generated Xcode project).
- `planner-core` / `planner-sync` / `planner-storage` crate extraction from
  `apps/desktop/src-tauri/src` into standalone crates.

## Shipped (verified 2026-07-16: lint/tsc/vitest/clippy/cargo test all green)

- Desktop overlay shell: transparent window, layer toggle, global shortcut
  with fallback, tray, autostart, multi-monitor window state.
- Android: native alarm/notification plugin with boot rescheduling, mobile
  touch/layout polish, SQLCipher vault encryption and Android Keystore-backed
  encryption keys for the vault and audio files.
- Profiles, Today/Notes/Reminders on local SQLite, reminder alert window,
  local diagnostics, 10-language i18n.
- Operation log with HLC ordering and last-write-wins conflict resolution.
- VDS sync server: email/password and admin-issued accounts, device
  linking, `/v1/sync/exchange`, `/v1/sync/events` long-poll wakeups,
  encrypted attachment (blob) sync, admin monitoring console.
- Google Drive / Yandex Disk rows removed from the Settings UI
  (`SettingsPanel.tsx`) — VDS is now the only connectable sync path in the
  app, matching the scope decision above. Backend code stays parked (see
  above), disabled by default.
- Default Android build no longer contains Google Auth/Play Services, and the
  manifest no longer advertises Android TV support. Cloud adapters are behind
  the opt-in Cargo feature `cloud-providers` for post-v1 work.
- Release Gradle tasks require an explicit signing key; debug builds remain
  available without release secrets. See
  [store-release-checklist.md](store-release-checklist.md).
- Registration requires explicit consent to the versioned privacy policy at
  `/privacy`; users can delete their account and all server-side data from the
  connected-account settings after password confirmation.
- Store release tooling now creates/verifies a permanent signing identity,
  builds arm64 + armv7, rejects debuggable or GMS-contaminated candidates, and
  records version/artifact evidence. RuStore/AppGallery listing drafts,
  permission declarations, store icon, and screenshot plan are prepared under
  `docs/store-listing` and `docs/store-assets`.
- VDS rollout tooling validates required production values, builds the server,
  verifies a pre-deploy database dump, applies the nginx and container changes,
  runs public smoke checks, and restores both layers on failure. An isolated
  Docker rehearsal restored all 9 public tables and exercised consent,
  registration, legal pages, and account deletion.

### Security fixes (external audit, 2026-07-16)

- **Stored XSS in the admin console.** `displayName` was interpolated into
  `innerHTML` unescaped ([admin_web.rs](../apps/server/src/admin_web.rs)); a
  malicious registration could run JS in an admin's session and read the
  session token out of `localStorage`, which wasn't behind nginx Basic Auth
  for the `/v1/admin/monitor` API itself. Fixed with an `esc()` helper on
  every user-controlled field rendered into the table.
- **Rate-limit bypass via spoofed `X-Forwarded-For`.**
  [routes.rs::client_ip](../apps/server/src/routes.rs) trusted the first
  (client-controlled) XFF value; nginx appends the real address rather than
  replacing it (`$proxy_add_x_forwarded_for`), so login/registration
  lockouts were trivially bypassable. Now prefers `X-Real-IP`, which nginx
  always sets to `$remote_addr` and a client cannot override.
- **No E2EE for sync operation payloads.** `payload_ciphertext` was
  literally the plaintext task/note/reminder JSON patch, on both the VDS
  path and the Google Drive journal — the field name implied encryption
  that never happened. Now encrypted with the same AES-256-GCM `media_key`
  primitive already used for blob attachments
  ([blob_crypto.rs](../apps/desktop/src-tauri/src/blob_crypto.rs)), with a
  clean fallback to plaintext for profiles with no `media_key` yet and for
  any already-synced legacy data.
- **Silent data loss on a corrupted Google Drive journal.** A parse failure
  was treated identically to "file doesn't exist," silently replacing the
  journal with an empty one (and a fresh media key) on next save. Now a
  hard error instead.
- **Argon2 blocking the Tokio runtime.** `hash_password`/`verify_password`
  ran synchronously inside async Axum handlers
  ([routes.rs](../apps/server/src/routes.rs)), which could starve all
  request handling under concurrent login/registration load. Now wrapped in
  `tokio::task::spawn_blocking`.
