# Troubleshooting

This guide collects common failure modes and how to diagnose them.

## Health endpoints

The API exposes two checks:

```bash
curl http://localhost:8080/health/live   # process is up
curl http://localhost:8080/health/ready  # DB + Redis reachable
```

If `/health/live` is `200` but `/health/ready` is not, a dependency is down. Check `docker compose ps` and `docker compose logs postgres redis`.

## Installer fails

| Symptom | Fix |
| --- | --- |
| `this installer must run as root (use sudo)` | Re-run with `sudo bash`. |
| `FileBase installer only supports Linux` | Use [Manual Docker Compose](./02-docker-compose.md) on macOS/Windows. |
| Docker install fails on an exotic distro | Install Docker manually, then re-run with `FILEBASE_SKIP_DOCKER=1`. |
| `compose up` hangs pulling images | Check outbound network and registry access from the host. |

## Dashboard cannot reach the API

Symptom: the dashboard loads but every request errors.

Causes:

1. `NEXT_PUBLIC_API_URL` points to an address your browser cannot reach. Remember it is a **client-side** URL, not container-internal. Set it to `http://YOUR_SERVER_IP:8080` or a public hostname.
2. CORS blocked the request. The API reads `DASHBOARD_URL` to build the CORS allow-list — make sure it matches the URL you load the dashboard from.

## First-run setup keeps showing

`GET /setup/status` is driven by the database. If it always returns `setup_required: true`:

- The database was wiped or reset.
- The admin user row was deleted manually.

To finish setup, complete the onboarding flow again. To prevent unwanted resets, snapshot the `postgres_data` volume.

## Uploads fail with `mime_not_allowed`

The preset's `allowedMimeTypes` does not include the file's detected MIME type. FileBase validates by sniffed bytes, not just the header — check the actual content type:

```bash
file --mime-type your-file.bin
```

Add the missing type to the preset, or convert the file before uploading.

## Uploads fail with `file_too_large`

Two limits apply:

- The preset's `maxFileSize`.
- The API's `MAX_UPLOAD_SIZE` environment variable (default 10 MB in `.env.example`).

Raise both if you need larger uploads. Reverse proxies (Nginx, Caddy) often enforce their own body-size cap — adjust `client_max_body_size` accordingly.

## Storage test fails

Use `POST /storage-connections/:id/test`. The error message describes which step failed (connect, write, read, delete). See the per-driver guides:

- [Local](./04-local-storage.md)
- [FTP](./05-ftp-storage.md)
- [SFTP](./06-sftp-storage.md)

## Background jobs do not run

The worker reads from the same Redis instance as the API. If image processing or webhook delivery never happens:

```bash
docker compose logs worker
docker compose exec redis redis-cli LLEN filebase:jobs:default
```

If the queue is growing but the worker is idle, restart the worker:

```bash
docker compose restart worker
```

## Useful logs

```bash
docker compose logs -f api
docker compose logs -f worker
docker compose logs -f dashboard
```

All services log JSON with a `request_id` correlation field. Grep for a request ID across services to trace a single upload end to end.

## Getting help

- Open an issue at https://github.com/binary-brawlers/filebase/issues with API and worker logs.
- Redact secrets (`JWT_SECRET`, `ENCRYPTION_KEY`, storage passwords, API keys) before posting.
