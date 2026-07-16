# Access-day release runbook

This is the exact sequence once the owner supplies legal, VDS, device, and
store access. Stop on the first failed command and attach output to the release
evidence. Do not upload an artifact built from a dirty or unrecorded commit.

## 1. Freeze inputs and signing identity

1. Confirm every item in `docs/store-listing/owner-inputs.md`.
2. Have counsel/owner approve the rendered operator details and legal text.
3. From `apps/desktop`, create the permanent key once:

   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/android-keystore-init.ps1 `
     -DistinguishedName "CN=..., OU=..., O=..., L=..., ST=..., C=RU"
   ```

4. Back up `release-key.jks` and `keystore.properties` twice, then restore and
   inspect one copy with `keytool -list -v`. Never commit either file.
5. Commit/review the intended release and record its hash.

## 2. Build the candidate

From `apps/desktop` on the prepared Windows builder:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/android-release-preflight.ps1
```

Copy `release-evidence-template.md` to a versioned evidence record and fill in
the AAB path, SHA-256, certificate fingerprint, SDK/JDK versions, and gate result.
Do not rebuild after recording the hash; any rebuild creates a new candidate.

## 3. Prepare and update the VDS

1. From the operator computer, run `apps/server/scripts/backup-vds-local.py`,
   verify its local manifest, and record the printed SHA-256. Confirm the current
   service is healthy. Do not retain the dump on the VDS.
2. Add the approved `FOCUSNOOK_LEGAL_*` and `FOCUSNOOK_SUPPORT_EMAIL` values to
   `/opt/focusnook/.env`; confirm no `change-me`/`replace-with` values remain.
3. In the checked-out `apps/server` directory run:

   ```sh
   sh scripts/predeploy-check.sh
   FOCUSNOOK_LOCAL_BACKUP_SHA256=<verified-sha256> sh scripts/deploy-vds.sh --apply
   ```

4. Record the local backup path, SHA-256, and smoke output. Verify `/privacy`
   and `/terms` display the approved identity. Deployment-time rollback is
   automatic; after a successful deploy no old FocusNook image or rollback file
   is retained on the VDS.

## 4. Signed-build acceptance

Install a derived signed APK on one Huawei/HMS device and one representative
RuStore device. Record device/OS versions and exercise the full checklist in
`store-release-checklist.md`, including rebooted reminders, offline recovery,
both sync directions, consent, and account deletion. Capture the six real
screens from `store-listing/screenshot-plan.md` only after this pass.

## 5. Submit

Use `store-listing/rustore-ru.md`, `store-listing/appgallery-en.md`, the data
safety/permission declaration, `store-assets/icon-512.png`, and captured images.
Reconfirm current console requirements, finish owner-only declarations, upload
the exact AAB hash from evidence to each store, and record both submission ids.

Release is complete only after moderation approval, public install on each
store's device, first-launch smoke, and VDS monitoring remain healthy.
