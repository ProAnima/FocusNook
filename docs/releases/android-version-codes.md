# Android release ledger

Never reuse a `versionCode` after an artifact has been uploaded to any store,
including draft or rejected submissions. Tauri derives the code from semantic
version as `major * 1_000_000 + minor * 1_000 + patch`.

| Version | versionCode | Status | RuStore | AppGallery | Certificate SHA-256 |
|---|---:|---|---|---|---|
| 0.1.0 | 1000 | reserved for v1 candidate | not uploaded | not uploaded | fill after key creation |
| 0.1.1 | 1001 | mixed-DPI fix candidate | not uploaded | not uploaded | debug build only |
| 0.1.2 | 1002 | WebView zoom reset candidate | not uploaded | not uploaded | debug build only |

Before each upload, record the artifact SHA-256 and store submission id in the
release evidence file created from `docs/releases/release-evidence-template.md`.
