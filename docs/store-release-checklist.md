# Android store release checklist

The v1 Android package is GMS-free and targets ordinary phones/tablets. Use one
signed AAB for both RuStore and Huawei AppGallery unless a store explicitly
requests APK.

## One-time setup

- Run `apps/desktop/scripts/android-keystore-init.ps1` to create the permanent
  upload identity; keep two encrypted backups and test one restore.
- Record package id `com.proanima.focusnook`, support email
  `info@proanima.net`, privacy URL `https://focus.proanima.net/privacy`, and
  agreement URL `https://focus.proanima.net/terms` in both consoles.
- Deploy the server migration and public legal routes before enabling account
  registration in a store build.

## Every candidate

1. Increase Android `versionCode`; never reuse a code uploaded to either store.
2. From `apps/desktop`, run
   `powershell -ExecutionPolicy Bypass -File scripts/android-release-preflight.ps1`.
   It runs all repository gates, builds one arm64 + armv7 AAB, verifies the
   signature, package/version, forbidden permissions, and absence of GMS auth.
3. Copy `docs/releases/release-evidence-template.md`, then record the artifact
   and certificate SHA-256 values printed by the preflight.
4. Install a derived APK on a Huawei device without GMS and a representative
   RuStore phone. Verify registration consent, desktop <-> Android sync,
   reminders after reboot, audio attachments, offline recovery, logout, and
   account deletion.
5. Confirm `https://focus.proanima.net/healthz`, `/readyz`, `/privacy`, and
   `/terms` are public and healthy. Record VDS backup/restore and smoke evidence.

## Prepared store material

- Draft listings and release notes: `docs/store-listing/`.
- Permission/data-safety source: `docs/store-listing/data-safety-and-permissions.md`.
- Screenshot shot list: `docs/store-listing/screenshot-plan.md`.
- Store icon: `docs/store-assets/icon-512.png` (512 x 512 PNG).
- Remaining owner/account inputs: `docs/store-listing/owner-inputs.md`.

RuStore accepts APK or AAB and requires a 512 x 512 icon plus at least one
actual phone screenshot. Huawei AppGallery Connect accepts APK/App Bundle and
requires localized app details, screenshots, category, regions, and a privacy
URL. Reconfirm live console fields at submission time because store forms and
declarations can change.

Do not upload if `keystore.properties` is absent: Gradle intentionally fails
release tasks instead of emitting an unsigned artifact.
