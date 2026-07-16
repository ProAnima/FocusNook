# Data safety and Android permission declarations

Use this as the source for both store questionnaires. Recheck it against the
final signed artifact with `android-verify-artifact.ps1`; store forms can change.

## Data processing

| Data | Collected/transmitted | Purpose | Optional | Protection / deletion |
|---|---|---|---|---|
| Email and display name | Only when an account is created | Account and sync | Yes; local mode works without an account | HTTPS; deleted through in-app account deletion |
| Password | Transmitted during registration/login; plaintext is not stored | Authentication | Yes | HTTPS; Argon2id hash on server |
| Device id/name/platform | When sync is enabled | Device linking and token rotation | Yes | Token hashes at rest; removed with account |
| Tasks, notes, reminders | Only when sync is enabled | Cross-device sync | Yes | Client-encrypted payload plus server-side at-rest encryption |
| Voice attachments | Only when sync is enabled | Cross-device attachment sync | Yes | Client-encrypted blob plus server-side at-rest encryption |
| Technical IP/auth-failure events | During server requests | Security, rate limiting, diagnostics | Required for online service | Limited operational use; policy/contact covers access and deletion requests |

No advertising id, contacts, precise location, payment data, health data, or
cross-app tracking is used. Data is not sold and is not used for advertising.
Android system/cloud backup is disabled for the app's local vault; optional
cross-device continuity is provided only through explicit FocusNook sync.

## Permission justification

- `INTERNET`: optional account registration and encrypted cross-device sync.
- `RECORD_AUDIO`: starts only after the user chooses voice-note recording.
- `MODIFY_AUDIO_SETTINGS`: supports the explicit voice recording flow.
- `POST_NOTIFICATIONS`: shows reminders requested by the user on Android 13+.
- `SCHEDULE_EXACT_ALARM`: delivers user-created reminders at the selected time.
- `RECEIVE_BOOT_COMPLETED`: reschedules saved reminders after device reboot.

The release verifier rejects broad package visibility, package installation, and
all-files storage permissions. The default store build excludes Google Play
Services authentication code.
