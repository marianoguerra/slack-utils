#!/usr/bin/env bash
# Serve this app via slack-archive-server (tests full API compatibility)
# Usage: ./scripts/serve-with-server.sh <base_path> <port>
set -e

BASE_PATH="${1:-.}"
PORT="${2:-8080}"

bun install && bun run build

CONFIG=$(mktemp --suffix=.toml)
cat > "$CONFIG" <<EOF
[server]
host = "127.0.0.1"
port = $PORT
static_assets = "$(pwd)"

[slack-archive]
base_path = "$BASE_PATH"
EOF

echo "Starting slack-archive-server on http://127.0.0.1:$PORT"
echo "Archive base path: $BASE_PATH"
echo "Web UI: http://127.0.0.1:$PORT/index.html"
echo ""

trap "rm -f $CONFIG" EXIT
cd ../.. && cargo run --features server --bin slack-archive-server -- serve "$CONFIG"
