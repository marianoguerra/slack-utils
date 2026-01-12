#!/usr/bin/env bash
# Start a local archive server for testing the client with local parquet files
# Usage: ./scripts/start-local-archive-server.sh <port> <base_path> <client_dir>
set -e

PORT="${1:-8080}"
BASE_PATH="${2:-.}"
CLIENT_DIR="${3:-tools/slack-archive-client}"

CONFIG=$(mktemp --suffix=.toml)
cat > "$CONFIG" <<EOF
[server]
host = "127.0.0.1"
port = $PORT
static_assets = "$CLIENT_DIR"

[slack-archive]
base_path = "$BASE_PATH"
EOF

echo "Starting local archive server on http://127.0.0.1:$PORT"
echo "Archive base path: $BASE_PATH"
echo "Static assets: $CLIENT_DIR"
echo ""
echo "Test the client at: http://127.0.0.1:$PORT/examples/index.html"
echo ""

trap "rm -f $CONFIG" EXIT
cargo run --features server --bin slack-archive-server -- serve "$CONFIG"
