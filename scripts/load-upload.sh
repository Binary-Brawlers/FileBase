#!/usr/bin/env bash
# Basic upload endpoint smoke/load helper.
#
# Required:
#   FILEBASE_API_URL       e.g. https://filebase.example.com
#   FILEBASE_API_KEY       private API key
#   FILEBASE_PRESET        preset name
#
# Optional:
#   FILEBASE_PROJECT_ID
#   FILEBASE_UPLOAD_FILE   file to upload; defaults to a generated temp file
#   REQUESTS               total requests, default 20
#   CONCURRENCY            parallel uploads, default 4

set -euo pipefail

API_URL="${FILEBASE_API_URL:?set FILEBASE_API_URL}"
API_KEY="${FILEBASE_API_KEY:?set FILEBASE_API_KEY}"
PRESET="${FILEBASE_PRESET:?set FILEBASE_PRESET}"
PROJECT_ID="${FILEBASE_PROJECT_ID:-}"
REQUESTS="${REQUESTS:-20}"
CONCURRENCY="${CONCURRENCY:-4}"

TEMP_FILE=""
UPLOAD_FILE="${FILEBASE_UPLOAD_FILE:-}"
if [ -z "$UPLOAD_FILE" ]; then
    TEMP_FILE="$(mktemp)"
    printf 'filebase-load-test-%s\n' "$(date +%s)" >"$TEMP_FILE"
    UPLOAD_FILE="$TEMP_FILE"
fi

cleanup() {
    if [ -n "$TEMP_FILE" ]; then
        rm -f "$TEMP_FILE"
    fi
}
trap cleanup EXIT

upload_once() {
    local code
    if [ -n "$PROJECT_ID" ]; then
        code="$(curl -sS -o /dev/null -w '%{http_code}' \
            -H "Authorization: Bearer ${API_KEY}" \
            -F "file=@${UPLOAD_FILE}" \
            -F "preset=${PRESET}" \
            -F "project_id=${PROJECT_ID}" \
            "${API_URL%/}/uploads")"
    else
        code="$(curl -sS -o /dev/null -w '%{http_code}' \
            -H "Authorization: Bearer ${API_KEY}" \
            -F "file=@${UPLOAD_FILE}" \
            -F "preset=${PRESET}" \
            "${API_URL%/}/uploads")"
    fi
    case "$code" in
        200|201|409) return 0 ;;
        *) printf 'upload failed with HTTP %s\n' "$code" >&2; return 1 ;;
    esac
}

export -f upload_once
export API_URL API_KEY PRESET PROJECT_ID UPLOAD_FILE

printf 'Running %s uploads with concurrency %s against %s\n' "$REQUESTS" "$CONCURRENCY" "$API_URL"
seq "$REQUESTS" | xargs -n1 -P "$CONCURRENCY" bash -c 'upload_once'
