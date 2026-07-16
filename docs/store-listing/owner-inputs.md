# Owner inputs required before rollout and submission

Do not guess these values. They are the remaining human/account inputs.

## Public legal identity

- Full registered publisher/operator name.
- Legal form and tax id (INN or applicable identifier).
- Legal and actual/postal address required by the publisher agreement.
- Confirmed support and personal-data contact email.
- Final legal review/approval of `/privacy` and `/terms` text.

## Android signing

- Distinguished name for the permanent release certificate.
- A release-key password stored in the approved password manager.
- Two encrypted backups of `release-key.jks` and `keystore.properties`, with one
  restore test and named release owners.

## VDS access window

- SSH access with Docker and nginx reload permissions.
- Confirmed `/opt/focusnook/.env`, compose/repository paths, and off-box backup target.
- Permission to update nginx and run a disposable Postgres restore drill.
- External uptime-monitor destination/contact.

## Store accounts and acceptance devices

- RuStore developer account and accepted publisher agreement.
- Huawei developer/AppGallery Connect access and verified developer identity.
- Distribution countries, age questionnaire, support site, and final commercial declarations.
- Approved privacy-policy/user-agreement translations for every non-Russian
  region selected in AppGallery; the prepared public legal pages are Russian.
- One Huawei/HMS phone and one representative RuStore phone for signed-build acceptance.
