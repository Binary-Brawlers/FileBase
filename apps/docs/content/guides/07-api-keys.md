# API Keys

API keys authenticate server-to-server calls from your backend to FileBase. They are scoped to a single project and are required for creating signed upload sessions, performing direct uploads, and managing files programmatically.

## Key format

FileBase issues keys with one of two prefixes:

| Prefix | Purpose |
| --- | --- |
| `fb_live_` | Production traffic. |
| `fb_test_` | Development and CI. Behaves identically but is intended to be revocable without disturbing production. |

A full key looks like `fb_live_8c4f9a2d3b1e7c5a9d6f0b8e2a4c6d8e`. The string after the prefix is high-entropy random data.

## Storage

FileBase stores only:

- A bcrypt-style hash of the secret part of the key.
- The prefix (`fb_live_` or `fb_test_`).
- A short display fragment (e.g. the last four characters) for UI listing.
- `lastUsedAt` and `revokedAt` timestamps.

The full key is shown to you **once**, immediately after creation. Copy it into your application's secret manager — it cannot be recovered later.

## Creating a key

From the dashboard: **Project → API Keys → Create**. From the API:

```bash
curl -X POST http://localhost:8080/api-keys \
  -H "Authorization: Bearer <admin-session-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "project_id": "prj_123",
    "name": "Production backend"
  }'
```

Response:

```json
{
  "id": "key_abc",
  "name": "Production backend",
  "prefix": "fb_live_",
  "key": "fb_live_8c4f9a2d3b1e7c5a9d6f0b8e2a4c6d8e",
  "createdAt": "2026-05-26T10:00:00Z"
}
```

## Using a key

Pass the key as a bearer token on server-side requests:

```bash
curl -X POST http://localhost:8080/uploads/sign \
  -H "Authorization: Bearer fb_live_..." \
  -H "Content-Type: application/json" \
  -d '{ "preset": "profile_images" }'
```

Never expose `fb_live_` keys to browsers or mobile apps. For client-side uploads, use [Signed Uploads](./09-signed-uploads.md).

## Revoking a key

From the dashboard: **API Keys → Revoke**. From the API:

```bash
curl -X PATCH http://localhost:8080/api-keys/key_abc/revoke \
  -H "Authorization: Bearer <admin-session-token>"
```

Revoked keys are rejected immediately on the next request. Rotation procedure:

1. Create a new key.
2. Deploy it to your application.
3. Verify the new key is being used by checking `lastUsedAt` in the dashboard.
4. Revoke the old key.
