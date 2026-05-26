#!/usr/bin/env bash
# Phase 15 integration smoke tests against a running, fresh FileBase API.
# This script intentionally initializes setup, so run it only against disposable data.

set -euo pipefail

API_URL="${FILEBASE_API_URL:-http://localhost:8080}"
RUN_ID="$(date +%s)"
EMAIL="phase15-${RUN_ID}@example.test"
PASSWORD="phase15-password-${RUN_ID}"
UPLOAD_DIR="${FILEBASE_TEST_UPLOAD_DIR:-/tmp/filebase-phase15-uploads}"
PRESET="phase15_${RUN_ID}"

json_get() {
    node -e "let s='';process.stdin.on('data',d=>s+=d);process.stdin.on('end',()=>{const p=process.argv[1].split('.');let v=JSON.parse(s);for(const k of p)v=v?.[k];if(v===undefined)process.exit(2);console.log(v)})" "$1"
}

request() {
    local method="$1" path="$2" body="${3:-}"
    if [ -n "$body" ]; then
        curl -fsS -X "$method" -H "content-type: application/json" -d "$body" "${API_URL%/}${path}"
    else
        curl -fsS -X "$method" "${API_URL%/}${path}"
    fi
}

status="$(request GET /setup/status)"
setup_required="$(printf '%s' "$status" | json_get data.setup_required)"
if [ "$setup_required" != "true" ]; then
    printf 'Refusing to run: %s is already initialized. Use a disposable fresh instance.\n' "$API_URL" >&2
    exit 1
fi

mkdir -p "$UPLOAD_DIR"

initialize_body="$(cat <<JSON
{
  "admin": { "name": "Phase 15", "email": "${EMAIL}", "password": "${PASSWORD}" },
  "project": { "name": "Phase 15", "slug": "phase15-${RUN_ID}" },
  "storage": {
    "type": "local",
    "base_path": "${UPLOAD_DIR}",
    "public_base_url": "${API_URL%/}/uploads"
  },
  "preset": {
    "name": "${PRESET}",
    "folder": "phase15",
    "allowed_mime_types": ["text/plain"],
    "allowed_extensions": ["txt"],
    "max_file_size": 1048576,
    "duplicate_strategy": "return_existing",
    "filename_strategy": "hash"
  }
}
JSON
)"

request POST /setup/initialize "$initialize_body" >/dev/null

login_body="{\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}"
login="$(request POST /auth/login "$login_body")"
token="$(printf '%s' "$login" | json_get data.token)"

project_id="$(curl -fsS -H "authorization: Bearer ${token}" "${API_URL%/}/projects" | json_get data.0.id)"
api_key="$(curl -fsS -X POST -H "authorization: Bearer ${token}" -H "content-type: application/json" \
    -d "{\"project_id\":\"${project_id}\",\"name\":\"Phase 15\",\"mode\":\"test\"}" \
    "${API_URL%/}/api-keys" | json_get data.secret)"

file="$(mktemp)"
trap 'rm -f "$file"' EXIT
printf 'phase15 duplicate body\n' >"$file"

first="$(curl -fsS -H "authorization: Bearer ${api_key}" -F "preset=${PRESET}" -F "file=@${file};type=text/plain;filename=phase15.txt" "${API_URL%/}/uploads")"
second="$(curl -fsS -H "authorization: Bearer ${api_key}" -F "preset=${PRESET}" -F "file=@${file};type=text/plain;filename=phase15.txt" "${API_URL%/}/uploads")"
first_id="$(printf '%s' "$first" | json_get data.id)"
second_id="$(printf '%s' "$second" | json_get data.id)"

if [ "$first_id" != "$second_id" ]; then
    printf 'duplicate detection failed: %s != %s\n' "$first_id" "$second_id" >&2
    exit 1
fi

printf 'Phase 15 integration smoke passed: setup, auth, local upload, duplicate detection.\n'
