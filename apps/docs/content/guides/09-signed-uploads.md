# Signed Uploads

Signed upload sessions let browsers and mobile apps upload directly to FileBase without ever seeing your `fb_live_` API key or your storage credentials.

## Flow

```text
Browser
  │ 1. ask app backend for a signed session
  ▼
App Backend (has fb_live_ key)
  │ 2. POST /uploads/sign
  ▼
FileBase
  │ 3. returns { uploadUrl, token, expiresAt }
  ▼
Browser
  │ 4. POST file to uploadUrl with token
  ▼
FileBase
  │ 5. validates, stores, returns final file URL
```

The token is scoped to a single project and preset, has a short TTL, and is stored as a hash on the server.

## Server: request a session

From your application backend:

```ts
const res = await fetch("http://localhost:8080/uploads/sign", {
  method: "POST",
  headers: {
    Authorization: `Bearer ${process.env.FILEBASE_API_KEY}`,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    preset: "profile_images",
    folder: "users/123",      // optional override
    maxFileSize: 5_000_000,   // optional override (must be ≤ preset max)
  }),
});

const session = await res.json();
// {
//   uploadUrl: "http://localhost:8080/uploads/sess_abc",
//   token: "tmp_signed_token...",
//   expiresAt: "2026-05-26T12:30:00Z"
// }
```

Return `session` to your client. The token itself is never stored in the database — only its hash is — so it cannot be replayed once expired or used.

## Client: upload the file

```ts
const form = new FormData();
form.append("file", file);

const res = await fetch(session.uploadUrl, {
  method: "POST",
  headers: { Authorization: `Bearer ${session.token}` },
  body: form,
});

const result = await res.json();
console.log(result.url);
```

Or use the [`@binary-brawlers/filebase-client`](./10-sdks.md) SDK which handles the two-step flow for you.

## Token rules

| Rule | Default |
| --- | --- |
| Expiration | A few minutes after creation. Set per session via `expiresIn`. |
| Scope | One project, one preset. |
| Use count | Single-use where the preset allows; multi-use sessions can be enabled per request. |
| MIME / size limits | Inherited from the preset, optionally tightened per session. Never loosened. |
| Storage | Token is hashed (no plaintext retained). |

If a token expires, is reused after consumption, or violates its limits, the upload request returns `401` or `400` and the file is not stored.

## When not to use signed uploads

If the upload originates from a trusted server you control, you can `POST /uploads` directly with your `fb_live_` key and skip the session step. Signed sessions exist specifically for untrusted clients.
