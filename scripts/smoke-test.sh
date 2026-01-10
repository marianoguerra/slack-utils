#!/usr/bin/env bash
set -e

# Source .env if it exists
if [ -f .env ]; then
    echo "=== Loading .env file ==="
    set -a
    source .env
    set +a
else
    echo "=== No .env file found, SLACK_TOKEN commands will be skipped ==="
fi

# Create temp directory for test outputs
TEMP_DIR=$(mktemp -d)
echo "=== Using temp directory: $TEMP_DIR ==="

# Cleanup function
cleanup() {
    echo ""
    echo "=== Cleaning up temp directory ==="
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo ""
echo "=== Building all binaries ==="
cargo build --all-features

echo ""
echo "=== Running tests ==="
cargo test --all-features

echo ""
echo "=== Running clippy ==="
cargo clippy --all-features

echo ""
echo "=== Testing slack-utils --help ==="
cargo run -- --help

echo ""
echo "=== Testing slack-utils subcommand help ==="
cargo run -- ui --help
cargo run -- export-conversations --help
cargo run -- export-conversations-week --help
cargo run -- archive-range --help
cargo run -- export-users --help
cargo run -- export-channels --help
cargo run -- download-attachments --help
cargo run -- export-markdown --help
cargo run -- export-emojis --help
cargo run -- export-index --help
cargo run -- import-index-meilisearch --help
cargo run -- query-meilisearch --help

echo ""
echo "=== Testing slack-utils-duckdb --help ==="
cargo run --features duckdb --bin slack-utils-duckdb -- --help
cargo run --features duckdb --bin slack-utils-duckdb -- query --help

echo ""
echo "=== Creating test fixture files ==="

# Create minimal users.json
cat > "$TEMP_DIR/users.json" << 'USERS_EOF'
[
    {"id": "U001", "name": "testuser", "real_name": "Test User", "profile": {"email": "test@example.com"}}
]
USERS_EOF

# Create minimal channels.json
cat > "$TEMP_DIR/channels.json" << 'CHANNELS_EOF'
[
    {"id": "C001", "name": "general", "is_channel": true, "num_members": 10}
]
CHANNELS_EOF

# Create minimal conversations.json
cat > "$TEMP_DIR/conversations.json" << 'CONVERSATIONS_EOF'
[
    {
        "channel_id": "C001",
        "channel_name": "general",
        "messages": [
            {"ts": "1700000000.000001", "user": "U001", "text": "Hello world", "type": "message"}
        ]
    }
]
CONVERSATIONS_EOF

# Create minimal selected-conversations.json
cat > "$TEMP_DIR/selected-conversations.json" << 'SELECTED_EOF'
[
    {
        "channel_id": "C001",
        "channel_name": "general",
        "messages": [
            {"ts": "1700000000.000001", "user": "U001", "text": "Hello world", "type": "message"}
        ]
    }
]
SELECTED_EOF

echo ""
echo "=== Testing export-index with fixtures ==="
cargo run -- export-index \
    --conversations "$TEMP_DIR/conversations.json" \
    --users "$TEMP_DIR/users.json" \
    --channels "$TEMP_DIR/channels.json" \
    --output "$TEMP_DIR/conversation-index.json"
test -f "$TEMP_DIR/conversation-index.json" && echo "export-index: OK"

echo ""
echo "=== Testing export-markdown with fixtures ==="
cargo run -- export-markdown \
    --conversations "$TEMP_DIR/selected-conversations.json" \
    --users "$TEMP_DIR/users.json" \
    --channels "$TEMP_DIR/channels.json" \
    --output "$TEMP_DIR/output.md"
test -f "$TEMP_DIR/output.md" && echo "export-markdown: OK"

echo ""
echo "=== Testing commands that require SLACK_TOKEN ==="

if [ -n "$SLACK_TOKEN" ]; then
    echo "SLACK_TOKEN is set, running Slack API commands..."

    cargo run -- export-users --output "$TEMP_DIR/users-export"
    test -f "$TEMP_DIR/users-export.json" && echo "export-users: OK"

    cargo run -- export-channels --output "$TEMP_DIR/channels-export"
    test -f "$TEMP_DIR/channels-export.json" && echo "export-channels: OK"

    cargo run -- export-conversations --output "$TEMP_DIR/conv-export"
    test -f "$TEMP_DIR/conv-export.json" && echo "export-conversations: OK"

    cargo run -- export-conversations-week --output "$TEMP_DIR/conv-week-export"
    test -f "$TEMP_DIR/conv-week-export.json" && echo "export-conversations-week: OK"

    cargo run -- export-emojis --output "$TEMP_DIR/emojis.json" --folder "$TEMP_DIR/emojis"
    test -f "$TEMP_DIR/emojis.json" && echo "export-emojis: OK"

    # Archive range: fetch last 4 weeks
    # Calculate week numbers (current week and 3 weeks ago)
    CURRENT_YEAR=$(date +%G)
    CURRENT_WEEK=$(date +%V)
    # Calculate 3 weeks ago using date arithmetic
    THREE_WEEKS_AGO=$(date -d '3 weeks ago' +%G-W%V)
    FROM_YEAR=$(echo "$THREE_WEEKS_AGO" | cut -d'-' -f1)
    FROM_WEEK=$(echo "$THREE_WEEKS_AGO" | cut -d'W' -f2)
    cargo run -- archive-range \
        --from-year "$FROM_YEAR" --from-week "$FROM_WEEK" \
        --to-year "$CURRENT_YEAR" --to-week "$CURRENT_WEEK" \
        --output "$TEMP_DIR/archive"
    test -d "$TEMP_DIR/archive" && echo "archive-range: OK"

    # download-attachments needs a conversations file with actual attachments, skip for now
    echo "download-attachments: SKIPPED (requires conversations with attachments)"
else
    echo "SLACK_TOKEN not set, skipping Slack API commands"
    echo "  - export-users: SKIPPED"
    echo "  - export-channels: SKIPPED"
    echo "  - export-conversations: SKIPPED"
    echo "  - export-conversations-week: SKIPPED"
    echo "  - export-emojis: SKIPPED"
    echo "  - archive-range: SKIPPED"
    echo "  - download-attachments: SKIPPED"
fi

echo ""
echo "=== Testing slack-utils-duckdb with existing parquet files ==="

if [ -f "users.parquet" ]; then
    cargo run --features duckdb --bin slack-utils-duckdb -- query "SELECT COUNT(*) FROM data" --parquet users.parquet
    echo "query users.parquet: OK"
else
    echo "users.parquet not found, skipping"
fi

if [ -f "channels.parquet" ]; then
    cargo run --features duckdb --bin slack-utils-duckdb -- query "SELECT COUNT(*) FROM data" --parquet channels.parquet
    echo "query channels.parquet: OK"
else
    echo "channels.parquet not found, skipping"
fi

if [ -d "conversations" ]; then
    cargo run --features duckdb --bin slack-utils-duckdb -- query "SELECT COUNT(*) FROM data"
    echo "query conversations parquet: OK"
else
    echo "conversations directory not found, skipping"
fi

echo ""
echo "=== All smoke tests passed ==="
