#!/usr/bin/env bash
set -euo pipefail

DIR="${FILEBASE_DIR:-/opt/filebase}"
VERSION=""
SKIP_BACKUP="false"

usage() {
  cat <<'USAGE'
Upgrade a FileBase installation with a pre-flight backup and health check.

Usage:
  upgrade.sh [--dir /opt/filebase] [--version 0.2.0] [--skip-backup]

Options:
  --dir DIR       Installation directory containing docker-compose.yml and .env.
                  Default: /opt/filebase (or $FILEBASE_DIR).
  --version TAG   Image tag to deploy. Defaults to the tag in .env, then latest.
  --skip-backup   Do not create a pre-upgrade backup.
  -h, --help      Show this help.

The script creates a backup, pulls the target images, recreates the stack, waits
for the API health endpoint, and prints rollback instructions.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dir)
      DIR="${2:?--dir requires a value}"
      shift 2
      ;;
    --version)
      VERSION="${2:?--version requires a value}"
      shift 2
      ;;
    --skip-backup)
      SKIP_BACKUP="true"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

COMPOSE_FILE="$DIR/docker-compose.yml"
ENV_FILE="$DIR/.env"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

require_command docker
require_command curl

if [[ ! -f "$COMPOSE_FILE" ]]; then
  echo "docker-compose.yml not found in $DIR" >&2
  exit 1
fi
if [[ ! -f "$ENV_FILE" ]]; then
  echo ".env not found in $DIR" >&2
  exit 1
fi

set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a

HTTP_PORT="${FILEBASE_HTTP_PORT:-8080}"
TARGET_VERSION="${VERSION:-${FILEBASE_VERSION:-latest}}"

compose() {
  docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" "$@"
}

if ! compose ps --status running --services 2>/dev/null | grep -qx api; then
  echo "The FileBase API does not appear to be running. Start the stack before upgrading." >&2
  exit 1
fi

echo "Running upgrade pre-flight check..."
if curl -fsS "http://127.0.0.1:${HTTP_PORT}/health/ready" >/dev/null 2>&1; then
  echo "  API health endpoint is reachable."
else
  echo "  WARNING: API health endpoint is not reachable before the upgrade."
fi

if [[ "$SKIP_BACKUP" != "true" ]]; then
  if [[ -f "$SCRIPT_DIR/backup.sh" ]]; then
    echo "Creating pre-upgrade backup..."
    bash "$SCRIPT_DIR/backup.sh" --dir "$DIR" || {
      echo "Backup failed. Re-run with --skip-backup to continue anyway." >&2
      exit 1
    }
  else
    echo "WARNING: backup.sh not found next to upgrade.sh; skipping backup." >&2
  fi
fi

if [[ "$TARGET_VERSION" != "${FILEBASE_VERSION:-}" ]]; then
  if grep -q '^FILEBASE_VERSION=' "$ENV_FILE"; then
    sed -i.bak "s|^FILEBASE_VERSION=.*|FILEBASE_VERSION=${TARGET_VERSION}|" "$ENV_FILE"
  else
    printf '\nFILEBASE_VERSION=%s\n' "$TARGET_VERSION" >>"$ENV_FILE"
  fi
  echo "Pinned FILEBASE_VERSION=$TARGET_VERSION in $ENV_FILE"
fi

echo "Pulling FileBase images ($TARGET_VERSION)..."
compose pull

echo "Recreating services..."
compose up -d

echo "Waiting for the API to become healthy..."
healthy="false"
for _ in $(seq 1 60); do
  if curl -fsS "http://127.0.0.1:${HTTP_PORT}/health/ready" >/dev/null 2>&1; then
    healthy="true"
    break
  fi
  sleep 2
done

if [[ "$healthy" != "true" ]]; then
  echo "API did not become healthy after the upgrade." >&2
  echo "Inspect logs with: docker compose -f \"$COMPOSE_FILE\" logs api" >&2
  echo "Roll back with:    scripts/restore.sh <backup-archive> --dir \"$DIR\"" >&2
  exit 1
fi

echo "Upgrade complete. API is healthy on version $TARGET_VERSION."
echo "Dashboard: ${DASHBOARD_URL:-http://127.0.0.1:${FILEBASE_DASHBOARD_PORT:-3000}}"
echo "If anything looks wrong, roll back with the backup created before this upgrade:"
echo "  scripts/restore.sh \"<backup-archive>\" --dir \"$DIR\""
