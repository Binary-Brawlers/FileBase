# Quick Start

This guide installs FileBase on a Linux server with a single command and walks through first-run setup.

## Prerequisites

- A Linux server (Ubuntu 22.04+, Debian 12+, Fedora 39+, or another modern distribution)
- Root or `sudo` access
- Ports `8080` (API) and `3000` (dashboard) reachable from your browser
- Outbound network access to pull Docker images

The installer will install Docker and Docker Compose for you if they are missing.

## Install

```bash
curl -fsSL https://get.filebase.dev/install.sh | sudo bash
```

The installer:

1. Verifies the Linux distribution.
2. Installs Docker and Docker Compose if they are not already present.
3. Creates `/opt/filebase` and `/opt/filebase/data/{postgres,uploads}`.
4. Generates a secure `JWT_SECRET` and `ENCRYPTION_KEY` and writes `/opt/filebase/.env`.
5. Writes the release `docker-compose.yml`.
6. Pulls FileBase images and starts the stack.
7. Prints the first-run setup URL.

## Override defaults

The installer accepts flags or environment variables:

```bash
curl -fsSL https://get.filebase.dev/install.sh | sudo bash -s -- \
  --dir /opt/filebase \
  --http-port 8080 \
  --dashboard-port 3000 \
  --version latest
```

| Flag | Env var | Default |
| --- | --- | --- |
| `--dir` | `FILEBASE_DIR` | `/opt/filebase` |
| `--version` | `FILEBASE_VERSION` | `latest` |
| `--http-port` | `FILEBASE_HTTP_PORT` | `8080` |
| `--dashboard-port` | `FILEBASE_DASHBOARD_PORT` | `3000` |
| `--compose-url` | `FILEBASE_COMPOSE_URL` | release default |

## Open the dashboard

Once the installer finishes, it prints the dashboard URL, for example:

```text
http://203.0.113.10:3000
```

Open it in your browser to begin the first-run onboarding flow. See [First-Run Onboarding](./03-first-run-onboarding.md).

## Verify the stack

From the server you can check the running services:

```bash
cd /opt/filebase
docker compose ps
docker compose logs -f api
```

The API has two health endpoints:

```bash
curl http://localhost:8080/health/live
curl http://localhost:8080/health/ready
```

## Upgrade

```bash
cd /opt/filebase
docker compose pull
docker compose up -d
```

## Uninstall

```bash
cd /opt/filebase
docker compose down
sudo rm -rf /opt/filebase
```

Removing `/opt/filebase` deletes uploaded files and the PostgreSQL volume — back up first if you need to retain data.
