# Update data safety

FocusNook is local-first. Release and support procedures must preserve the local
vault even when server sync is temporarily unavailable.

## Windows

- Install a newer NSIS package over the existing installation.
- Do not manually delete `%APPDATA%\com.proanima.focusnook` or
  `%LOCALAPPDATA%\com.proanima.focusnook`.
- The FocusNook NSIS hook forces the application-data deletion flag off during
  uninstall, so removing program files leaves the encrypted vault available for
  reinstall or support recovery.

## Android

- Install an update over the existing package. For a connected test phone use
  `scripts/android-install-update-windows.ps1`; it calls `adb install -r` and
  never falls back to uninstall or `pm clear`.
- Every distributed update must use the same signing certificate and a higher
  `versionCode`.
- Never tell a user to uninstall or clear storage as a troubleshooting step.
  Android removes app-specific storage on uninstall; the application cannot
  override that operating-system behavior.
- If an in-place update is rejected because the certificate differs, stop. First
  verify that the user's server account contains their operations, then migrate
  them to the permanent release-signed build through an explicit recovery flow.

## Sync recovery

Each client version has a one-time account snapshot seed. It queues the current
tasks, notes, reminders and audio metadata even if an older client incorrectly
marked its operation journal as synced. An operation remains pending until the
server confirms every sent operation as accepted or already present.
