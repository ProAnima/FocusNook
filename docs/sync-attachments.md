# FocusNook sync attachments

This document describes the production contract for synchronized attachments. Voice recordings,
images, and future small files use the same client queue, encryption, retry, and VDS blob API.

## Goals

- Keep planner data local-first: notes, reminders, and tasks stay usable without the server.
- Sync operation metadata and binary attachments separately.
- Never upload raw media bytes to the VDS.
- Make uploads idempotent, so retrying after a network failure is safe.
- Keep the server content-blind: it routes by user, profile, and blob id, but does not need media
  plaintext.

## Data Model

Client-side attachments are tracked in `sync_blobs`:

- `profile_id`: local profile scope.
- `blob_id`: stable attachment id. Voice recordings currently use the stored audio filename.
- `local_path`: path relative to the local attachment directory.
- `content_type`: MIME type, currently `audio/webm`.
- `sha256`: checksum of the client-encrypted sync payload, not of plaintext.
- `size_bytes`: encrypted payload size accepted by the server.
- `sync_payload_base64`: cached encrypted upload body for deterministic retries.
- `uploaded_at`, `downloaded_at`, `deleted_at`: local transfer state.

The legacy table name is preserved for migration compatibility. New code treats every row as an
attachment. The only type-specific fields are the MIME type and local renderer.

Operations may reference attachments using:

- legacy `audioPath` for existing voice notes and reminders;
- `attachmentIds: string[]` for content-type-neutral references;
- `attachments: { id: string, contentType: string }[]` for images and future files.

## Encryption Layers

There are three distinct layers:

1. Local desktop audio at rest is encrypted with the existing local vault audio key.
2. Sync payloads are encrypted on the client with AES-256-GCM before upload.
3. The server encrypts the already encrypted sync payload again at rest.

The client sync media key is derived from normalized account email and password with Argon2id. The
server does not store this media key. This keeps a VDS database dump from directly exposing media
content, while still allowing a second device to decrypt media after the same account login.

Android currently has the same sync encryption, but local Android audio at rest is still limited by
the existing mobile vault/keystore backlog.

## Sync Order

Upload flow:

1. A note or reminder with `audioPath` is created locally.
2. The local mutation is written to `sync_operations`.
3. The attachment is registered in `sync_blobs`.
4. The attachment outbox attempts every pending upload independently.
5. An operation is publishable only when its own referenced attachments have server acknowledgements.
6. Text-only operations and operations whose attachments are ready are exchanged immediately.

Download flow:

1. Sync pulls remote operations.
2. Remote note/reminder operations are applied locally.
3. The pull cursor is committed after the operation batch is safely applied.
4. Attachment references are registered in the independent attachment inbox.
5. Each available attachment is downloaded, checksum-verified, decrypted, and materialized.
6. Missing attachments remain pending and produce a degraded-media warning without rolling back
   the operation cursor.

## Reliability Invariants

- A broken attachment can block only operations that reference that attachment.
- A missing download can never block later text, tasks, reminders, or other attachments.
- Uploads are retried with the same encrypted payload and stable id.
- The server accepts duplicate uploads and operations idempotently.
- The operation cursor represents applied metadata, never attachment availability.
- Attachment availability is represented only by the attachment outbox/inbox state.
- Integrity or decryption failures never write unverified plaintext to local storage.

## Scaling Rules

- Keep `FOCUSNOOK_MAX_BLOB_BYTES` conservative for mobile sync. The current server default is meant
  for short voice notes and small files.
- Large future files should use chunked blobs with a manifest operation, not a larger single body.
- Content hash and idempotency checks must stay on encrypted bytes.
- The server must never infer attachment content from MIME type beyond validation and quota policy.
- User-facing storage metrics should count encrypted payload bytes, because that is the real VDS
  storage cost.

## Known Follow-Ups

- Add Android local keystore-backed at-rest encryption for downloaded media.
- Add quota UI before enabling image/file uploads by default.
- Add chunk manifests before allowing large binary files.
- Add media-key rotation if the product later supports password changes with preserved old media.
