#!/usr/bin/env bash
# Export a self-contained static UI with parquet files for static hosting
# Creates dist/static-ui/ with everything needed for deployment
# Usage: ./scripts/export-static-ui.sh <path>
set -e

PARQUET_SOURCE="${1:?Error: Path argument required}"

# Validate source path
if [ ! -d "$PARQUET_SOURCE" ]; then
    echo "Error: Path '$PARQUET_SOURCE' does not exist or is not a directory"
    exit 1
fi

PARQUET_PATH="$(cd "$PARQUET_SOURCE" && pwd)"

# Check required files exist
if [ ! -f "$PARQUET_PATH/users.parquet" ]; then
    echo "Error: users.parquet not found in $PARQUET_SOURCE"
    exit 1
fi
if [ ! -f "$PARQUET_PATH/channels.parquet" ]; then
    echo "Error: channels.parquet not found in $PARQUET_SOURCE"
    exit 1
fi
if [ ! -d "$PARQUET_PATH/conversations" ]; then
    echo "Error: conversations/ directory not found in $PARQUET_SOURCE"
    exit 1
fi

echo "Building app..."
bun install
bun run build

echo "Creating dist/static-ui/..."
rm -rf dist/static-ui
mkdir -p dist/static-ui

# Copy web assets
cp static-files.html dist/static-ui/index.html
cp dist/app.js dist/static-ui/app.js
cp style.css dist/static-ui/style.css

# Copy parquet files
echo "Copying parquet files from $PARQUET_PATH..."
cp "$PARQUET_PATH/users.parquet" dist/static-ui/
cp "$PARQUET_PATH/channels.parquet" dist/static-ui/
cp -r "$PARQUET_PATH/conversations" dist/static-ui/

# Count files for summary
THREAD_FILES=$(find dist/static-ui/conversations -name "*.parquet" 2>/dev/null | wc -l)
TOTAL_SIZE=$(du -sh dist/static-ui | cut -f1)

echo ""
echo "Static UI exported to dist/static-ui/"
echo "  - index.html"
echo "  - app.js"
echo "  - style.css"
echo "  - users.parquet"
echo "  - channels.parquet"
echo "  - conversations/ ($THREAD_FILES thread files)"
echo ""
echo "Total size: $TOTAL_SIZE"
echo ""
echo "To serve locally: cd dist/static-ui && python3 -m http.server 8000"
echo "Or deploy to any static hosting (GitHub Pages, S3, Netlify, etc.)"
