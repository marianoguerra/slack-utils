#!/usr/bin/env bash
# Start slack-archive-server with web-duckdb-wasm as static assets
# This tests the full stack: server API + DuckDB client + web UI
# Usage: ./scripts/serve-with-server.sh <base_path> <port> <web_app_dir>
set -e

BASE_PATH="${1:-.}"
PORT="${2:-8080}"
WEB_APP_DIR="${3:-../web-duckdb-wasm}"

# Build the web app
cd "$WEB_APP_DIR" && bun install && bun run build
cd - > /dev/null

CONFIG=$(mktemp --suffix=.toml)
cat > "$CONFIG" <<EOF
[server]
host = "127.0.0.1"
port = $PORT
static_assets = "$(pwd)/$WEB_APP_DIR"

[slack-archive]
base_path = "$BASE_PATH"
EOF

echo "Starting slack-archive-server on http://127.0.0.1:$PORT"
echo "Archive base path: $BASE_PATH"
echo "Web UI: http://127.0.0.1:$PORT/index.html"
echo ""

trap "rm -f $CONFIG" EXIT
cd ../.. && cargo run --features server --bin slack-archive-server -- serve "$CONFIG"
