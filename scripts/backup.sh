#!/usr/bin/env bash
set -euo pipefail

DIR="${FILEBASE_DIR:-/opt/filebase}"
OUTPUT_DIR="${FILEBASE_BACKUP_DIR:-}"

usage() {
  cat <<'USAGE'
Back up a FileBase installation (PostgreSQL database + local uploads).

Usage:
  backup.sh [--dir /opt/filebase] [--output /path/to/backups]

Options:
  --dir DIR      Installation directory containing docker-compose.yml and .env.
                 Default: /opt/filebase (or $FILEBASE_DIR).
  --output DIR   Directory for backup archives.
                 Default: <dir>/backups (or $FILEBASE_BACKUP_DIR).
  -h, --help     Show this help.

The archive contains database.sql.gz, uploads.tar.gz, and manifest.json.
Use scripts/restore.sh to restore it.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dir)
      DIR="${2:?--dir requires a value}"
      shift 2
      ;;
    --output)
      OUTPUT_DIR="${2:?--output requires a value}"
      shift 2
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

OUTPUT_DIR="${OUTPUT_DIR:-$DIR/backups}"
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
VERSION="${FILEBASE_VERSION:-unknown}"

compose() {
  docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" "$@"
}

if ! compose ps --status running --services 2>/dev/null | grep -qx postgres; then
  echo "Postgres service is not running in $DIR" >&2
  exit 1
fi

TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

echo "Dumping PostgreSQL database..."
compose exec -T postgres pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" --no-owner --clean --if-exists \
  | gzip -9 > "$WORK_DIR/database.sql.gz"

UPLOADS_DIR="$DIR/data/uploads"
if [[ -d "$UPLOADS_DIR" ]]; then
  echo "Archiving local uploads..."
  tar -C "$(dirname "$UPLOADS_DIR")" -czf "$WORK_DIR/uploads.tar.gz" "$(basename "$UPLOADS_DIR")"
else
  echo "No local uploads directory found; recording an empty archive."
  tar -C "$WORK_DIR" -czf "$WORK_DIR/uploads.tar.gz" --files-from /dev/null
fi

cat > "$WORK_DIR/manifest.json" <<EOF
{
  "createdAt": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "hostname": "$(hostname)",
  "filebaseDir": "$DIR",
  "version": "$VERSION"
}
EOF

mkdir -p "$OUTPUT_DIR"
ARCHIVE="$OUTPUT_DIR/filebase-backup-$TIMESTAMP.tar.gz"
tar -C "$WORK_DIR" -czf "$ARCHIVE" database.sql.gz uploads.tar.gz manifest.json

SIZE="$(du -h "$ARCHIVE" | cut -f1)"
echo "Backup complete: $ARCHIVE ($SIZE)"
echo "Restore with: scripts/restore.sh \"$ARCHIVE\" --dir \"$DIR\""
