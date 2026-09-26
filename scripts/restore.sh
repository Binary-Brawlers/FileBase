#!/usr/bin/env bash
set -euo pipefail

DIR="${FILEBASE_DIR:-/opt/filebase}"
ASSUME_YES="false"

usage() {
  cat <<'USAGE'
Restore a FileBase installation from a backup archive.

Usage:
  restore.sh BACKUP_ARCHIVE [--dir /opt/filebase] [--yes]

Arguments:
  BACKUP_ARCHIVE   Archive created by scripts/backup.sh.

Options:
  --dir DIR        Installation directory containing docker-compose.yml and .env.
                   Default: /opt/filebase (or $FILEBASE_DIR).
  --yes            Skip the interactive confirmation prompt.
  -h, --help       Show this help.

The restore stops the API, worker, and dashboard, replaces the database schema
and local uploads, then starts the stack again. Redis is left untouched.
USAGE
}

ARCHIVE=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dir)
      DIR="${2:?--dir requires a value}"
      shift 2
      ;;
    --yes | -y)
      ASSUME_YES="true"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      if [[ -n "$ARCHIVE" ]]; then
        echo "Unexpected argument: $1" >&2
        usage >&2
        exit 1
      fi
      ARCHIVE="$1"
      shift
      ;;
  esac
done

if [[ -z "$ARCHIVE" ]]; then
  echo "A backup archive path is required." >&2
  usage >&2
  exit 1
fi

COMPOSE_FILE="$DIR/docker-compose.yml"
ENV_FILE="$DIR/.env"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

require_command docker
require_command tar
require_command gzip

if [[ ! -f "$ARCHIVE" ]]; then
  echo "Backup archive not found: $ARCHIVE" >&2
  exit 1
fi

if [[ ! -f "$COMPOSE_FILE" ]]; then
  echo "docker-compose.yml not found in $DIR" >&2
  exit 1
fi

if [[ -f "$ENV_FILE" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$ENV_FILE"
  set +a
fi

POSTGRES_USER="${POSTGRES_USER:-filebase}"
POSTGRES_DB="${POSTGRES_DB:-filebase}"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
tar -C "$WORK_DIR" -xzf "$ARCHIVE" database.sql.gz uploads.tar.gz

if [[ "$ASSUME_YES" != "true" ]]; then
  echo "This will replace the database and local uploads in $DIR."
  echo "Type 'restore' to continue:"
  read -r confirmation
  if [[ "$confirmation" != "restore" ]]; then
    echo "Restore cancelled."
    exit 1
  fi
fi

compose() {
  docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" "$@"
}

echo "Stopping application services..."
compose stop api worker dashboard >/dev/null 2>&1 || true

if ! compose ps --status running --services 2>/dev/null | grep -qx postgres; then
  echo "Starting Postgres..."
  compose up -d postgres
fi

echo "Restoring PostgreSQL database..."
gunzip -c "$WORK_DIR/database.sql.gz" \
  | compose exec -T postgres psql -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB"

echo "Restoring local uploads..."
mkdir -p "$DIR/data"
rm -rf "$DIR/data/uploads"
tar -C "$DIR/data" -xzf "$WORK_DIR/uploads.tar.gz"

echo "Starting application services..."
compose up -d

echo "Restore complete. Verify with: docker compose -f \"$COMPOSE_FILE\" ps"
