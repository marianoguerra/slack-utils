# Slack Utils justfile

# File path defaults (without extensions for format flexibility)
conversations_path := "conversations"
selected_conversations_file := "selected-conversations.json"
users_path := "users"
channels_path := "channels"
attachments_dir := "attachments"
emojis_file := "emojis.json"
emojis_dir := "emojis"
index_file := "conversation-index.json"
markdown_file := "selected-conversations.md"

# Format default (json or parquet)
default_format := "json"

# Meilisearch defaults
ms_url := "http://localhost:7700"
ms_index := "slack"

# Default recipe to list available targets
default:
    @just --list

# Launch the interactive TUI
ui:
    cargo run -- ui

# Export conversations from the last 7 days
export-conversations output=conversations_path format=default_format:
    #!/usr/bin/env bash
    from=$(date -d '7 days ago' +%Y-%m-%d)
    to=$(date +%Y-%m-%d)
    cargo run -- export-conversations --from "$from" --to "$to" --output {{output}} --format {{format}}

# Export conversations with custom date range
export-conversations-range from to output=conversations_path format=default_format:
    cargo run -- export-conversations --from {{from}} --to {{to}} --output {{output}} --format {{format}}

# Export conversations for current work week (defaults to current year and week)
export-conversations-week output=conversations_path format=default_format:
    cargo run -- export-conversations-week --output {{output}} --format {{format}}

# Export conversations for specific work week
export-conversations-week-custom year week output=conversations_path format=default_format:
    cargo run -- export-conversations-week --year {{year}} --week {{week}} --output {{output}} --format {{format}}

# Export users
export-users output=users_path format=default_format:
    cargo run -- export-users --output {{output}} --format {{format}}

# Export channels
export-channels output=channels_path format=default_format:
    cargo run -- export-channels --output {{output}} --format {{format}}

# Download attachments from conversations
download-attachments input="conversations.json" output=attachments_dir:
    cargo run -- download-attachments --input {{input}} --output {{output}}

# Export selected conversations to markdown
export-markdown conversations=selected_conversations_file users="users.json" channels="channels.json" output=markdown_file:
    cargo run -- export-markdown --conversations {{conversations}} --users {{users}} --channels {{channels}} --output {{output}}

# Export custom emojis
export-emojis output=emojis_file folder=emojis_dir:
    cargo run -- export-emojis --output {{output}} --folder {{folder}}

# Export conversations to searchable index
export-index conversations="conversations.json" users="users.json" channels="channels.json" output=index_file:
    cargo run -- export-index --conversations {{conversations}} --users {{users}} --channels {{channels}} --output {{output}}

# Import index to Meilisearch
import-meilisearch api_key input=index_file url=ms_url index_name=ms_index:
    cargo run -- import-index-meilisearch --input {{input}} --url {{url}} --api-key {{api_key}} --index-name {{index_name}}

# Import index to Meilisearch and clear existing data
import-meilisearch-clear api_key input=index_file url=ms_url index_name=ms_index:
    cargo run -- import-index-meilisearch --input {{input}} --url {{url}} --api-key {{api_key}} --index-name {{index_name}} --clear

# Query Meilisearch index
query-meilisearch query api_key url=ms_url index_name=ms_index:
    cargo run -- query-meilisearch "{{query}}" --url {{url}} --api-key {{api_key}} --index-name {{index_name}}

# Start Meilisearch server
start-meilisearch:
    ./meilisearch --master-key $MS_MASTER_KEY

# DuckDB defaults
conversations_parquet := "conversations/year=*/week=*/*.parquet"
users_parquet := "users.parquet"
channels_parquet := "channels.parquet"

# Build the duckdb binary
build-duckdb:
    cargo build --features duckdb --bin slack-utils-duckdb

# Query conversations parquet with DuckDB
query-duckdb query parquet=conversations_parquet:
    cargo run --features duckdb --bin slack-utils-duckdb -- query "{{query}}" --parquet "{{parquet}}"

# Query users parquet with DuckDB
query-duckdb-users query parquet=users_parquet:
    cargo run --features duckdb --bin slack-utils-duckdb -- query "{{query}}" --parquet "{{parquet}}"

# Query channels parquet with DuckDB
query-duckdb-channels query parquet=channels_parquet:
    cargo run --features duckdb --bin slack-utils-duckdb -- query "{{query}}" --parquet "{{parquet}}"

# Run sample DuckDB queries to show useful stats and summaries
run-duckdb-sample-queries:
    #!/usr/bin/env bash
    set -e
    DUCKDB="cargo run --features duckdb --bin slack-utils-duckdb -- query"

    echo "=== Overview Stats ==="
    echo ""
    echo "--- Total Users ---"
    $DUCKDB "SELECT COUNT(*) as total_users, SUM(CASE WHEN is_bot THEN 1 ELSE 0 END) as bots, SUM(CASE WHEN is_admin THEN 1 ELSE 0 END) as admins FROM data" --parquet users.parquet

    echo ""
    echo "--- Total Channels ---"
    $DUCKDB "SELECT COUNT(*) as total_channels, SUM(CASE WHEN is_archived THEN 1 ELSE 0 END) as archived, SUM(CASE WHEN is_private THEN 1 ELSE 0 END) as private FROM data" --parquet channels.parquet

    echo ""
    echo "--- Total Messages ---"
    $DUCKDB "SELECT COUNT(*) as total_messages, SUM(CASE WHEN is_reply THEN 1 ELSE 0 END) as thread_replies, COUNT(DISTINCT user) as unique_users FROM data"

    echo ""
    echo "=== Channel Stats ==="
    echo ""
    echo "--- Top 10 Channels by Members ---"
    $DUCKDB "SELECT name, num_members FROM data WHERE NOT is_archived ORDER BY num_members DESC LIMIT 10" --parquet channels.parquet

    echo ""
    echo "--- Top 10 Channels by Message Count ---"
    $DUCKDB "SELECT channel_name, COUNT(*) as msg_count FROM data GROUP BY channel_name ORDER BY msg_count DESC LIMIT 10"

    echo ""
    echo "=== User Activity ==="
    echo ""
    echo "--- Top 10 Most Active Users ---"
    $DUCKDB "SELECT user, COUNT(*) as msg_count FROM data GROUP BY user ORDER BY msg_count DESC LIMIT 10"

    echo ""
    echo "=== Time-based Stats ==="
    echo ""
    echo "--- Messages by Week ---"
    $DUCKDB "SELECT year, week, COUNT(*) as msg_count FROM data GROUP BY year, week ORDER BY year DESC, week DESC LIMIT 10"

    echo ""
    echo "--- Messages by Date (Last 10 Days) ---"
    $DUCKDB "SELECT date, COUNT(*) as msg_count FROM data GROUP BY date ORDER BY date DESC LIMIT 10"

    echo ""
    echo "=== Thread Activity ==="
    echo ""
    echo "--- Top 10 Threads by Reply Count ---"
    $DUCKDB "SELECT channel_name, thread_ts, COUNT(*) as reply_count FROM data WHERE is_reply GROUP BY channel_name, thread_ts ORDER BY reply_count DESC LIMIT 10"

    echo ""
    echo "=== Sample Queries Complete ==="

# Run smoke tests for all CLI commands
smoke-test:
    ./scripts/smoke-test.sh
