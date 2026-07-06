# FocusNook Sync Server

Production-oriented VDS sync relay for FocusNook.

The server is intentionally content-blind:

- users and devices are routed server-side;
- sync operations are scoped by authenticated user and profile;
- client payloads are expected to be encrypted before upload;
- payloads and blobs are encrypted again at rest with AES-GCM;
- access tokens are stored only as HMAC-SHA256 hashes.
- duplicate operation and blob ids are strictly idempotent: the same id with different
  content returns `409 Conflict` instead of being silently accepted.

## Deploy

Standalone server with bundled Caddy:

```bash
cd apps/server
sh scripts/generate-env.sh sync.example.com
docker compose --env-file .env up -d --build
```

The compose file includes Caddy. Point DNS for `FOCUSNOOK_DOMAIN` to the VDS,
open ports `80` and `443`, and Caddy will issue TLS certificates automatically.

Existing VDS with nginx already owning `80`/`443`:

```bash
cd apps/server
docker compose -p focusnook --env-file /opt/focusnook/.env -f compose.vds-nginx.yml up -d --build
```

In this mode the Rust server is bound only to `127.0.0.1:${FOCUSNOOK_HOST_PORT:-18080}`.
Put `nginx/focusnook.conf` into `/etc/nginx/sites-available`, enable it, run `nginx -t`,
then reload nginx. The root web page is protected by nginx Basic Auth; `/v1/*`,
`/healthz`, and `/readyz` remain API endpoints protected by their own bearer-token rules.

The compose files also run a small Postgres backup container. It writes daily custom-format
dumps to `${FOCUSNOOK_BACKUP_DIR:-apps/server/backups}` and keeps the last 14 days. Copy this
folder off the VDS with your normal server backup flow; a local folder on the same disk is a
recovery point, not a disaster-recovery strategy.

## Bootstrap

Create a user with the admin token from `.env`:

```bash
curl -sS https://sync.example.com/v1/admin/users \
  -H "Authorization: Bearer $FOCUSNOOK_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"displayName":"Primary"}'
```

The response contains `userToken` once. For a product-managed desktop build, store it in the
local app-data `sync_providers.json` file outside git:

```json
{
  "server": {
    "endpoint": "https://sync.example.com",
    "userToken": "fnk_user_..."
  }
}
```

The desktop settings UI then shows only a ready/not-ready state and an Enable button. On enable,
the client registers the local device and stores the returned device token in the OS keyring.

Register a device:

```bash
curl -sS https://sync.example.com/v1/devices \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"deviceId":"desktop-device-id","displayName":"Windows desktop","platform":"windows"}'
```

The response contains `deviceToken` once. Sync calls use this token.

Check operational counters:

```bash
curl -sS https://sync.example.com/v1/admin/stats \
  -H "Authorization: Bearer $FOCUSNOOK_ADMIN_TOKEN"
```

## API

- `GET /healthz` - process is alive.
- `GET /readyz` - database is reachable.
- `GET /v1/admin/stats` - operational counters, admin token required.
- `POST /v1/admin/users` - create a sync user, admin token required.
- `POST /v1/devices` - register or rotate a device token, user token required.
- `POST /v1/sync/exchange` - push local operations and pull remote operations, device token required.
- `POST /v1/blobs` - upload an encrypted blob, device token required.
- `GET /v1/blobs/:profileId/:blobId` - download an encrypted blob, device token required.

## Sync contract

`/v1/sync/exchange` accepts opaque encrypted operation payloads. The server validates size,
ownership, uniqueness, and ordering cursor, but never interprets task/note/reminder content.

The server returns every operation after the client's cursor, including operations that came
from the same device. This is intentional: if a desktop or phone loses local state but keeps
its device token, it can rebuild from the relay. Clients must apply operations idempotently.

Domain conflict handling remains a client concern. The server is a durable relay, not the
source of truth for planner state. Server-level conflicts are reserved for broken sync
invariants, for example reusing the same `operationId` or `blobId` with different content.

## Restore

Stop writers before restoring a dump:

```bash
docker compose --env-file .env stop server
docker compose --env-file .env exec -T postgres pg_restore \
  -U "$POSTGRES_USER" \
  -d "$POSTGRES_DB" \
  --clean \
  --if-exists \
  < backups/focusnook-YYYYMMDDTHHMMSSZ.dump
docker compose --env-file .env up -d
```

## Web Admin UI

`GET /` serves a compact monitoring console for operators. It is intentionally separate from
the bearer-token sync API:

- on the nginx VDS profile, the page is first protected by nginx Basic Auth;
- inside the page, a secondary password from `FOCUSNOOK_WEB_SECONDARY_PASSWORD` is required;
- repeated wrong secondary-password attempts are locked out per client IP;
- the web session is in-memory and expires automatically;
- the console shows user count, device count, per-user storage usage, and traffic counters;
- the UI supports dark/light theme and 10 languages.

The console never decrypts or displays planner content. It is operational telemetry only.

## Production Notes

- Rotate `FOCUSNOOK_ADMIN_TOKEN` after bootstrap if it was exposed in shell history.
- Keep `FOCUSNOOK_WEB_SECONDARY_PASSWORD` different from shell, database, and GitHub passwords.
- Store `.env`, database dumps, and VDS snapshots outside the repository.
- Keep TLS termination in Caddy and do not expose the Rust server port publicly.
- For stronger availability than a single VDS, add managed Postgres or streaming replication,
  offsite backups, and an external uptime monitor.
