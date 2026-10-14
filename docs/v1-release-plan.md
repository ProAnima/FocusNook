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
2. **VDS operational follow-ups** (stability rollout completed 2026-08-21):
   - rotate `FOCUSNOOK_ADMIN_TOKEN` after bootstrap;
   - add an external uptime monitor for `/healthz` and `/readyz`;
   - retain the verified local backup and its manifest according to the
     operator's retention policy. The production `.env` remains outside the
     release tree and no database dump is retained on the VDS.
3. **One recorded end-to-end desktop + Android sync pass** against the same
   live VDS server. The pass must prove tasks (including adding and removing
   the long-running marker) and recorded audio in both directions, including
   playback after a cold restart and a temporary network interruption. The
   encrypted audio transform is covered by an automated two-device round-trip
   test with distinct local vault keys, but there is still no recorded
   physical-device pass through the deployed server.
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
- Code-size refactor (budgets in [AGENTS.md](../AGENTS.md), map in
  [code-structure.md](code-structure.md)). Done: the whole frontend (settings,
  reminders, notes, hooks, `commands`, translations, CSS), the server `routes/`,
  and desktop Rust (`server_sync/`, `commands/` + `shell.rs` out of `lib.rs`,
  `db/`, `profiles/`). Intentionally left: parked `cloud_sync.rs` (post-v1,
  behind the `cloud-providers` feature), `plan_items.rs` (~315 production lines,
  marginally over budget) and test-heavy modules whose production code is within
  budget.
- Quality gates are one command (`scripts/check.ps1`) and CI
  (`.github/workflows/ci.yml`, **not yet run on GitHub** — verify the first run).
  Desktop Rust uses leveled logging to a rotating file; the log file and the
  Android Logcat output have not yet been checked on a running app or device.
- Android-gated Rust code (`#[cfg(target_os = "android")]`) was moved during the
  refactor but cannot be compiled on the developer machine; confirm with a real
  Android build before release (it is part of the signed-candidate acceptance).

## Security design review (2026-10-07) — owner decision needed

- **Sync media key is derivable by the server operator.** The client sends the
  account password to the VDS, and the payload/attachment key is
  `Argon2(password, sha256(email))` (`blob_crypto.rs::derive_media_key`), so
  "client-side encryption" does not protect data from whoever runs the server.
  There is also a silent single-SHA-256 fallback when Argon2 fails, and AES-GCM
  nonces come from UUIDv4 bytes (some bits fixed). Public texts promise
  client-side encryption, not operator blindness, so nothing is falsely
  claimed — but decide before release whether to fix (requires a protocol and
  data migration) or to keep and describe precisely. Card X1 in
  [agent-roadmap.md](agent-roadmap.md).
- Remaining architecture work is planned for a multi-agent team in
  [agent-roadmap.md](agent-roadmap.md) (metrics: `python scripts/metrics.py`).

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

## Implemented, pending physical release acceptance (updated 2026-10-12)

- Zen Mode and task rename (2026-10-12): desktop focus icon beside pin opens a
  one-task view of unfinished items in today's list, including long-running
  items. Next cycles through unfinished tasks without changing their status;
  completed tasks leave the ring, and the view ends only when none remain.
  Zen Mode fits height to the card and Next button within 72–180 px limits, with a single
  exit button at the top right. Normal geometry is restored on exit and is not
  overwritten by compact geometry. Lists and rename dialogs use Settings-style
  thin scrollbars; task titles wrap in narrow windows.
  Inline actions remain beside the title when both fit; otherwise they move above
  it. The renderer restores the native Zen state after reload.
  Next follows the task card with a 4 px gap; unused height below the button is
  removed by native content sizing, while long titles retain a scrollable area.
  Native geometry commands require live-window validation; they have no mock
  WebviewWindow infrastructure and are not covered by Rust unit tests.
  Live Windows checks observed the empty view shrink to 72 px and restore its
  previous 260 px height on exit. A cross-monitor DPI pass remains pending.
  At 280 px width, browser checks observed a short card at 85 px height and a
  long card at 167 px, with Next following the content without trailing space.
  Inline actions and details include Rename on
  desktop and mobile. Renames sync as a `title` update patch that other devices apply.
  Windows UI and browser preview are checked locally; physical Android and
  live two-device VDS acceptance of renaming remain pending.
  Local gates passed: lint, TypeScript, production build, 163 frontend tests,
  desktop fmt/Clippy + 135 tests, and server fmt/Clippy + 23 tests. Desktop test
  linking reports OpenSSL LNK4099 debug-symbol warnings; Clippy is clean.

- Account sync now has one source of truth: the stored per-account server
  session. The redundant `profiles.json` enablement flag was removed because
  migrated accounts could look connected in Settings while background sync
  silently skipped them. Switching accounts invalidates any in-flight response
  before the vault changes and immediately schedules the newly active account;
  manual and background exchanges now share one serialized run gate.
  Settings exposes one account-bound connection flow instead of a second manual
  VDS login that could attach an unrelated server identity. Automated desktop
  tests cover migrated credentials, account-switch invalidation, enablement,
  manual sync, and account deletion; the physical two-account/two-device VDS
  pass remains part of blocker 3 above.
- Tasks can be marked as long-running independently of their completion
  status. Marked tasks are global: they appear in a compact group above every
  daily list, do not inflate that day's completion counter, and are skipped by
  daily rollover. Desktop keeps inline actions; Android moves secondary
  actions into the touch-sized details dialog so task titles retain space.
- The SQLite migration uses a safe `false` default for existing rows. The VDS
  operation patch, remote apply path, and full snapshot all carry
  `isLongRunning`; automated migration, rollover, snapshot, and remote-apply
  tests pass.
- Desktop and 390 px mobile React/CSS layouts were inspected in browser
  preview, including accessible region/toggle semantics. A signed Android
  build on a physical phone and the two-device live-VDS pass above remain
  required before this item is considered release-accepted.
- A reversible live-VDS protocol audit with separate desktop/Android device
  tokens proved encrypted operation exchange, long-running marker updates,
  duplicate idempotency, cross-device wakeups, encrypted audio upload/download,
  and exact audio-byte recovery. The temporary account and its data were
  deleted afterwards.
- The 2026-08-21 VDS stability rollout added SIGTERM-aware graceful shutdown,
  bounded request/config limits, Docker health-check start periods and log
  rotation, and nginx throttling scoped to login/registration only. The blob
  request limit now includes JSON/Base64 expansion instead of rejecting valid
  blobs above roughly 12 MiB. A post-deploy Windows-to-Android live probe
  transferred an operation, received the long-poll wakeup, and round-tripped a
  13 MiB opaque blob with an exact SHA-256 match. Cleanup restored the production
  aggregates to 7 users, 20 active devices, 224 operations, and 8 blobs; the
  server remained healthy with 0 restarts and 0 OOM events.
- Production backup evidence (2026-08-21):
  `focusnook-20260821T080037Z.dump`, 1029733 bytes, SHA-256
  `b657e27fe709a5def7108bcb4c56315db9c5d78ac79507033e15c4c69e39e867`.
  The local hash matches its manifest and the temporary VDS dump count is zero.
- The hardened server image was deployed on 2026-07-26 after the verified
  backup and restore drill. Independent live probes now receive `400 Bad
  Request` for both a deliberately wrong blob SHA-256 and an unsafe `../` blob
  id.
- Production backup evidence (2026-07-26): `focusnook-20260726T111800Z.dump`,
  500218 bytes, SHA-256
  `88227e03c893269a9e439e02c767ec033519cadbab0606be12419e9d3ad6ae14`.
  The local hash matches its manifest, the temporary VDS dump count is zero,
  and an isolated PostgreSQL 16 restore recovered all 9 public tables with 20
  validated key constraints, 0 invalid constraints, 171 operation rows, and 5
  blob rows. The disposable restore container was removed.
- Post-backup public smoke passed for health/readiness, legal pages, security
  headers, HTTP-to-HTTPS redirect, nginx config, and both healthy application
  containers. The origin TLS leaf certificate was valid with 70 days remaining.
  After deployment, a repeated reversible two-device probe passed account
  registration/login, encrypted task and long-running-marker transfer,
  idempotency, cross-device wakeup, encrypted audio upload/download, checksum
  verification, and exact decrypted WAV recovery. Both hardened blob
  validations passed; cleanup left 0 audit accounts and 0 orphan
  operation/blob rows.

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
  [routes/helpers.rs::client_ip](../apps/server/src/routes/helpers.rs) trusted the first
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
  ([routes/accounts.rs](../apps/server/src/routes/accounts.rs)), which could starve all
  request handling under concurrent login/registration load. Now wrapped in
  `tokio::task::spawn_blocking`.

### Sync and audio hardening (2026-07-26)

- Attachment integrity fails closed: checksum, decryption, and local
  materialization failures never expose unverified bytes. The original coupling
  between attachment availability and the operation cursor was superseded by
  the independent-delivery design below.
- Downloaded audio is checked for exact size, SHA-256, successful media-key
  decryption, and exact local plaintext before an atomic replacement. A corrupt
  local copy is repaired from the verified server copy.
- Legacy plaintext audio is re-encrypted in place on first read/upload.
  SQLCipher migration now verifies `integrity_check` and removes the plaintext
  database backup after successful conversion.
- The server recomputes uploaded blob SHA-256 instead of trusting the client,
  and both client and server reject blob IDs capable of path traversal.
- VDS requests now have explicit connect/request timeouts; blob transfers use a
  separate three-minute budget suitable for mobile recordings.

### Independent attachment delivery (2026-07-31)

- The operation journal and attachment delivery now advance independently.
  A missing remote attachment cannot hold the operation pull cursor.
- The attachment outbox gates only operations that reference an attachment
  without a server acknowledgement. Independent text and task operations continue.
- Duplicate attachment upload passes were removed. One generic queue now serves
  audio and the future `attachments` contract used by images and files.
- The legacy `sync_blobs` table and VDS `/v1/blobs` API remain unchanged for
  backwards compatibility; the client treats them as generic attachments.
