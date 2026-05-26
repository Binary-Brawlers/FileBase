# Manual Docker Compose Install

The one-command installer is the quickest path, but you can also run FileBase with `docker compose` directly. This is useful for development, custom networking, or when the installer cannot run on your host.

## Prerequisites

- Docker 24+ and the `docker compose` plugin
- Git
- Ports `8080` and `3000` available

## Clone the repository

```bash
git clone https://github.com/binary-brawlers/filebase.git
cd filebase
```

## Create `.env`

Copy the example file and edit the values:

```bash
cp .env.example .env
```

Required keys:

```env
APP_URL=http://localhost:8080
DASHBOARD_URL=http://localhost:3000

DATABASE_URL=postgres://filebase:password@postgres:5432/filebase
REDIS_URL=redis://redis:6379

JWT_SECRET=$(openssl rand -hex 32)
ENCRYPTION_KEY=$(openssl rand -hex 32)

STORAGE_DRIVER=local
LOCAL_STORAGE_PATH=/var/lib/filebase/uploads
PUBLIC_BASE_URL=http://localhost:8080/uploads

MAX_UPLOAD_SIZE=10485760
DEFAULT_DUPLICATE_STRATEGY=return_existing
```

`JWT_SECRET` and `ENCRYPTION_KEY` must be at least 32 bytes of high-entropy random data. Generate them with `openssl rand -hex 32` before saving the file.

## Start the stack

```bash
docker compose up -d
```

The first run builds the API, worker, and dashboard images from source. Subsequent runs reuse the cache. Watch the logs while it boots:

```bash
docker compose logs -f
```

## Services

The default `docker-compose.yml` ships five services:

| Service | Image | Purpose | Ports |
| --- | --- | --- | --- |
| `api` | built from `docker/api.Dockerfile` | Axum HTTP API | `8080:8080` |
| `worker` | built from `docker/worker.Dockerfile` | Background jobs | – |
| `dashboard` | built from `docker/dashboard.Dockerfile` | Next.js admin UI | `3000:3000` |
| `postgres` | `postgres:16-alpine` | Metadata store | `5432:5432` |
| `redis` | `redis:7-alpine` | Queue + cache | `6379:6379` |

## Volumes

Two named volumes persist state:

- `postgres_data` — PostgreSQL data directory
- `uploads_data` — local-storage uploads, shared between `api` and `worker`

To back up either volume:

```bash
docker run --rm -v filebase_postgres_data:/data \
  -v "$PWD":/backup alpine \
  tar czf /backup/postgres.tgz -C /data .
```

## Production release compose file

For a pre-built image deployment (no source checkout, no build step) use [`docker-compose.release.yml`](https://github.com/binary-brawlers/filebase/blob/main/docker-compose.release.yml). It pulls tagged images from the registry and is the file used by the one-command installer.

## Stop and clean up

```bash
docker compose down            # stop services, keep volumes
docker compose down --volumes  # also delete postgres + uploads (destructive)
```
