#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVER_URL="${SERVER_URL:-http://127.0.0.1:3000}"
TARGET_URL="https://example.com"
OHA_DURATION="30s"
OHA_CONNECTIONS="100"

# Workload shape: 80% hot, 20% cold by default.
HOT_CODES=20
COLD_CODES=200
HOT_WEIGHT=80
COLD_WEIGHT=20

cd "${ROOT_DIR}"

# Build optimized binary for realistic performance numbers.
echo "Building release binary..."
cargo build --release

# Start server in background and capture PID for cleanup.
echo "Starting server..."
RUST_LOG=info cargo run --release >/tmp/linx-bench.log 2>&1 &
SERVER_PID=$!

# Ensure the background server is stopped on exit.
cleanup() {
  if kill -0 "${SERVER_PID}" 2>/dev/null; then
    kill "${SERVER_PID}" >/dev/null 2>&1 || true
    wait "${SERVER_PID}" >/dev/null 2>&1 || true
  fi
}

trap cleanup EXIT

# Wait for the health endpoint to report ready.
echo "Waiting for server..."
for _ in {1..30}; do
  if curl -fsS "${SERVER_URL}/api/health" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! curl -fsS "${SERVER_URL}/api/health" >/dev/null 2>&1; then
  echo "Server did not become ready. Check /tmp/linx-bench.log."
  exit 1
fi

WORK_DIR="${ROOT_DIR}/.bench"
mkdir -p "${WORK_DIR}"
URL_FILE="${WORK_DIR}/urls.txt"
HOT_URLS="${WORK_DIR}/hot.txt"
COLD_URLS="${WORK_DIR}/cold.txt"

rm -f "${URL_FILE}" "${HOT_URLS}" "${COLD_URLS}"

echo "Creating hot codes (${HOT_CODES})..."
for i in $(seq 1 "${HOT_CODES}"); do
  code="benchhot${i}"
  curl -sS -X POST "${SERVER_URL}/api/shorten" \
    -H 'Content-Type: application/json' \
    -d "{\"url\":\"${TARGET_URL}\",\"code\":\"${code}\"}" >/dev/null || true
  echo "${SERVER_URL}/${code}" >>"${HOT_URLS}"
done

echo "Creating cold codes (${COLD_CODES})..."
for i in $(seq 1 "${COLD_CODES}"); do
  code="benchcold${i}"
  curl -sS -X POST "${SERVER_URL}/api/shorten" \
    -H 'Content-Type: application/json' \
    -d "{\"url\":\"${TARGET_URL}\",\"code\":\"${code}\"}" >/dev/null || true
  echo "${SERVER_URL}/${code}" >>"${COLD_URLS}"
done

echo "Building mixed workload file..."
for _ in $(seq 1 "${HOT_WEIGHT}"); do
  cat "${HOT_URLS}" >>"${URL_FILE}"
done
for _ in $(seq 1 "${COLD_WEIGHT}"); do
  cat "${COLD_URLS}" >>"${URL_FILE}"
done

# Run oha without following redirects to measure Linx itself.
echo "Running oha..."
OHA_CMD=(oha -z "${OHA_DURATION}" -c "${OHA_CONNECTIONS}" -r 0 --urls-from-file "${URL_FILE}")

"${OHA_CMD[@]}"
