# Security Recommendations

FileBase ships with safe defaults, but a self-hosted deployment is only as secure as the host running it. This guide collects the recommendations you should review before exposing FileBase to production traffic.

## Secrets

| Secret | Where it lives | Notes |
| --- | --- | --- |
| `JWT_SECRET` | `/opt/filebase/.env` | At least 32 bytes of random data. Rotating invalidates all sessions. |
| `ENCRYPTION_KEY` | `/opt/filebase/.env` | 32 bytes (256 bits). Used to encrypt FTP/SFTP credentials. **Do not rotate without re-encrypting existing rows** — losing this key makes stored credentials unrecoverable. |
| Storage passwords / keys | PostgreSQL, encrypted at rest with AES-256-GCM. | Never returned by API responses. |
| API keys (`fb_live_`, `fb_test_`) | Stored as bcrypt-style hashes plus a prefix. | Full key shown once at creation. |

Generate strong values with `openssl rand -hex 32`. Keep `.env` mode `600` and owned by root.

## Transport

Always terminate TLS in front of FileBase. The shipped Docker Compose listens on plain HTTP — put Caddy, Nginx, or Traefik in front and serve `https://` to clients. Without TLS, signed upload tokens and JWT session cookies travel in cleartext.

## Network exposure

- Do not expose `5432` (PostgreSQL) or `6379` (Redis) to the internet. The default `docker-compose.yml` publishes them for local development convenience; remove the `ports:` entries on production hosts and use the internal Docker network only.
- Restrict the dashboard origin via `DASHBOARD_URL`. The CORS allow-list is derived from it.

## API keys

- Use `fb_test_` keys for development and CI; reserve `fb_live_` for production.
- Issue one key per integration so revocation is targeted.
- Never embed `fb_live_` keys in browser or mobile binaries. Use [Signed Uploads](./09-signed-uploads.md) for client-side flows.

## Signed upload sessions

- Keep TTLs short (a few minutes).
- Tighten preset rules per session when issuing tokens to less-trusted clients.
- Prefer single-use sessions.
- Treat the token returned to the browser as a bearer credential — never log it.

## File validation

FileBase validates uploads by:

1. MIME type sniffed from file bytes.
2. Extension.
3. Magic-byte check where practical.
4. Size against preset and global limits.

Do not loosen these in custom code. Magic-byte checks catch the most common file-spoofing attempts.

## Storage credential hygiene

- Use SFTP with key-based auth in preference to FTP with passwords. See [SFTP Storage](./06-sftp-storage.md).
- Use a dedicated SFTP user whose home directory is the upload base path. Disable shell access (`chsh -s /sbin/nologin`).
- Restrict the SFTP user with `Match User filebase / ChrootDirectory ...` in `sshd_config`.

## Logging and audit

FileBase logs sensitive actions (login, key creation, key revocation, storage credential changes). Logs include a `request_id` correlation field. Do **not** log:

- Plaintext API keys
- Plaintext storage credentials
- Signed upload tokens

If you add custom hooks or webhooks, follow the same rule.

## Webhooks

Outbound webhook payloads are signed with HMAC-SHA256. Verify the signature on the receiving end before trusting the payload. Do not store webhook secrets in client-side code.

## Backups

Back up regularly:

- The PostgreSQL volume (`postgres_data`)
- The uploads volume (`uploads_data`) if you use the local adapter
- `/opt/filebase/.env` — without `ENCRYPTION_KEY`, encrypted credentials in the database are unreadable

Store backups encrypted at rest.

## Reporting vulnerabilities

See [SECURITY.md](https://github.com/binary-brawlers/filebase/blob/main/SECURITY.md) at the repository root for the disclosure policy.
