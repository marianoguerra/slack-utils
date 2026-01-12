#!/usr/bin/env bash
# Build all binaries for release and create distribution package
# Usage: ./scripts/dist.sh [client_dir]
set -e

CLIENT_DIR="${1:-tools/slack-archive-client}"

echo "=== Building distribution package ==="

# Clean and create dist directory
rm -rf dist
mkdir -p dist

echo "Building slack-utils (release)..."
cargo build --release --features tui
cp target/release/slack-utils dist/

echo "Building slack-utils-duckdb (release)..."
cargo build --release --features duckdb --bin slack-utils-duckdb
cp target/release/slack-utils-duckdb dist/

echo "Building slack-archive-server (release)..."
cargo build --release --features server --bin slack-archive-server
cp target/release/slack-archive-server dist/

echo "Building slack-archive-client (JS)..."
cd "$CLIENT_DIR" && bun install && bun run dist
cd - > /dev/null
cp -r "$CLIENT_DIR/dist" dist/slack-archive-client

echo "Copying documentation and config files..."
cp resources/dist-README.md dist/README.md
cp resources/sample-server-config.toml dist/config.example.toml

echo ""
echo "=== Distribution package created in dist/ ==="
ls -lah dist/
echo ""
echo "Contents:"
echo "  - slack-utils            : Main CLI tool"
echo "  - slack-utils-duckdb     : DuckDB query tool"
echo "  - slack-archive-server   : HTTP server"
echo "  - slack-archive-client/  : JS client library (ESM + types)"
echo "  - README.md              : Usage instructions"
echo "  - config.example.toml    : Server config template"
