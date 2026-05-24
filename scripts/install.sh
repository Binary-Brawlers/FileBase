#!/usr/bin/env bash
# FileBase one-command installer.
#
# Usage:
#   curl -fsSL https://get.filebase.dev/install.sh | sudo bash
#   curl -fsSL https://get.filebase.dev/install.sh | sudo bash -s -- --dir /opt/filebase
#
# Environment overrides:
#   FILEBASE_DIR        Install directory (default: /opt/filebase)
#   FILEBASE_VERSION    Image tag to pull (default: latest)
#   FILEBASE_HTTP_PORT  API port (default: 8080)
#   FILEBASE_DASHBOARD_PORT  Dashboard port (default: 3000)
#   FILEBASE_COMPOSE_URL     Override release docker-compose.yml URL

set -euo pipefail

FILEBASE_DIR="${FILEBASE_DIR:-/opt/filebase}"
FILEBASE_VERSION="${FILEBASE_VERSION:-latest}"
FILEBASE_HTTP_PORT="${FILEBASE_HTTP_PORT:-8080}"
FILEBASE_DASHBOARD_PORT="${FILEBASE_DASHBOARD_PORT:-3000}"
FILEBASE_COMPOSE_URL="${FILEBASE_COMPOSE_URL:-}"

# ---- arg parsing ------------------------------------------------------------
while [ $# -gt 0 ]; do
    case "$1" in
        --dir) FILEBASE_DIR="$2"; shift 2 ;;
        --version) FILEBASE_VERSION="$2"; shift 2 ;;
        --http-port) FILEBASE_HTTP_PORT="$2"; shift 2 ;;
        --dashboard-port) FILEBASE_DASHBOARD_PORT="$2"; shift 2 ;;
        --compose-url) FILEBASE_COMPOSE_URL="$2"; shift 2 ;;
        --help|-h)
            sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            exit 1
            ;;
    esac
done

# ---- helpers ----------------------------------------------------------------
log()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m!!\033[0m  %s\n' "$*" >&2; }
fail() { printf '\033[1;31mxx\033[0m  %s\n' "$*" >&2; exit 1; }

require_root() {
    if [ "$(id -u)" -ne 0 ]; then
        fail "this installer must run as root (use sudo)"
    fi
}

check_os() {
    if [ "$(uname -s)" != "Linux" ]; then
        fail "FileBase installer only supports Linux. Detected: $(uname -s)"
    fi
    if [ -r /etc/os-release ]; then
        # shellcheck disable=SC1091
        . /etc/os-release
        case "${ID:-}" in
            ubuntu|debian|raspbian|linuxmint|pop)
                PKG_MGR=apt ;;
            fedora|rhel|centos|rocky|almalinux|amzn)
                PKG_MGR=dnf ;;
            *)
                warn "unrecognized distribution '${ID:-unknown}'. Continuing with best effort."
                PKG_MGR=unknown
                ;;
        esac
    else
        warn "/etc/os-release missing; cannot detect distribution."
        PKG_MGR=unknown
    fi
}

install_docker_if_missing() {
    if command -v docker >/dev/null 2>&1; then
        log "Docker already installed: $(docker --version)"
        return
    fi
    log "Installing Docker via get.docker.com"
    if ! command -v curl >/dev/null 2>&1; then
        fail "curl is required to install Docker. Install curl and rerun."
    fi
    curl -fsSL https://get.docker.com | sh
    systemctl enable --now docker >/dev/null 2>&1 || true
}

ensure_compose_plugin() {
    if docker compose version >/dev/null 2>&1; then
        log "Docker Compose plugin detected: $(docker compose version | head -n1)"
        return
    fi
    log "Installing Docker Compose plugin"
    case "${PKG_MGR:-}" in
        apt)
            apt-get update -y
            apt-get install -y docker-compose-plugin
            ;;
        dnf)
            dnf install -y docker-compose-plugin
            ;;
        *)
            fail "could not install docker-compose-plugin automatically; install it manually and rerun."
            ;;
    esac
    docker compose version >/dev/null 2>&1 \
        || fail "docker compose still unavailable after install"
}

random_hex() {
    # 32-byte hex secret
    local len="${1:-32}"
    if command -v openssl >/dev/null 2>&1; then
        openssl rand -hex "$len"
    else
        head -c "$len" /dev/urandom | od -An -tx1 | tr -d ' \n'
    fi
}

detect_server_ip() {
    local ip=""
    if command -v hostname >/dev/null 2>&1; then
        ip="$(hostname -I 2>/dev/null | awk '{print $1}')"
    fi
    if [ -z "$ip" ] && command -v ip >/dev/null 2>&1; then
        ip="$(ip route get 1 2>/dev/null | awk '{for(i=1;i<=NF;i++) if($i=="src"){print $(i+1); exit}}')"
    fi
    if [ -z "$ip" ]; then
        ip="localhost"
    fi
    printf '%s' "$ip"
}

write_env() {
    local env_path="$1" ip="$2"
    if [ -f "$env_path" ]; then
        log ".env already exists at $env_path — leaving in place"
        return
    fi
    local jwt_secret encryption_key db_password
    jwt_secret="$(random_hex 32)"
    encryption_key="$(random_hex 32)"
    db_password="$(random_hex 16)"
    log "Writing $env_path with generated secrets"
    cat >"$env_path" <<EOF
APP_URL=http://${ip}:${FILEBASE_HTTP_PORT}
DASHBOARD_URL=http://${ip}:${FILEBASE_DASHBOARD_PORT}

DATABASE_URL=postgres://filebase:${db_password}@postgres:5432/filebase
DATABASE_MAX_CONNECTIONS=10
DATABASE_MIN_CONNECTIONS=1
DATABASE_CONNECT_TIMEOUT_SECONDS=5
POSTGRES_PASSWORD=${db_password}

REDIS_URL=redis://redis:6379

JWT_SECRET=${jwt_secret}
ENCRYPTION_KEY=${encryption_key}

STORAGE_DRIVER=local
LOCAL_STORAGE_PATH=/var/lib/filebase/uploads
PUBLIC_BASE_URL=http://${ip}:${FILEBASE_HTTP_PORT}/uploads

MAX_UPLOAD_SIZE=10485760
DEFAULT_DUPLICATE_STRATEGY=return_existing

FILEBASE_VERSION=${FILEBASE_VERSION}
FILEBASE_HTTP_PORT=${FILEBASE_HTTP_PORT}
FILEBASE_DASHBOARD_PORT=${FILEBASE_DASHBOARD_PORT}
EOF
    chmod 600 "$env_path"
}

write_compose() {
    local path="$1"
    if [ -f "$path" ]; then
        log "docker-compose.yml already exists — leaving in place"
        return
    fi
    if [ -n "$FILEBASE_COMPOSE_URL" ]; then
        log "Downloading docker-compose.yml from $FILEBASE_COMPOSE_URL"
        curl -fsSL "$FILEBASE_COMPOSE_URL" -o "$path" \
            || fail "failed to download compose file"
        return
    fi
    log "Writing docker-compose.yml"
    cat >"$path" <<'EOF'
services:
  api:
    image: filebase/api:${FILEBASE_VERSION:-latest}
    env_file: .env
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    ports:
      - "${FILEBASE_HTTP_PORT:-8080}:8080"
    volumes:
      - ./data/uploads:/var/lib/filebase/uploads
    restart: unless-stopped

  worker:
    image: filebase/worker:${FILEBASE_VERSION:-latest}
    env_file: .env
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    volumes:
      - ./data/uploads:/var/lib/filebase/uploads
    restart: unless-stopped

  dashboard:
    image: filebase/dashboard:${FILEBASE_VERSION:-latest}
    env_file: .env
    depends_on:
      - api
    ports:
      - "${FILEBASE_DASHBOARD_PORT:-3000}:3000"
    restart: unless-stopped

  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: filebase
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_DB: filebase
    volumes:
      - ./data/postgres:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U filebase -d filebase"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 10s
    restart: unless-stopped

  redis:
    image: redis:7-alpine
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 5s
    restart: unless-stopped
EOF
    chmod 644 "$path"
}

# ---- main -------------------------------------------------------------------
require_root
check_os

log "Installing FileBase into $FILEBASE_DIR"
mkdir -p "$FILEBASE_DIR" "$FILEBASE_DIR/data/postgres" "$FILEBASE_DIR/data/uploads"
chmod 700 "$FILEBASE_DIR/data/postgres"

install_docker_if_missing
ensure_compose_plugin

SERVER_IP="$(detect_server_ip)"
log "Detected server IP: $SERVER_IP"

write_env "$FILEBASE_DIR/.env" "$SERVER_IP"
write_compose "$FILEBASE_DIR/docker-compose.yml"

log "Pulling FileBase images (version: $FILEBASE_VERSION)"
( cd "$FILEBASE_DIR" && docker compose pull )

log "Starting FileBase services"
( cd "$FILEBASE_DIR" && docker compose up -d )

cat <<EOF

\033[1;32mFileBase is up.\033[0m

  Setup URL:    http://${SERVER_IP}:${FILEBASE_DASHBOARD_PORT}
  API URL:      http://${SERVER_IP}:${FILEBASE_HTTP_PORT}
  Install dir:  ${FILEBASE_DIR}
  .env:         ${FILEBASE_DIR}/.env  (chmod 600 — keep this secret)

Open the setup URL in a browser to create the first admin account and
configure your initial storage connection.

Common operations:
  cd ${FILEBASE_DIR}
  docker compose ps                 # status
  docker compose logs -f api        # tail logs
  docker compose pull && docker compose up -d   # upgrade
  docker compose down               # stop (data preserved)

To uninstall:
  cd ${FILEBASE_DIR} && docker compose down -v && cd - && rm -rf ${FILEBASE_DIR}
  (this deletes all uploaded files and the database — back up first)
EOF
