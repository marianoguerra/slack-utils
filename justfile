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

# Archive last 4 weeks to parquet
archive-last-4-weeks output=conversations_path:
    #!/usr/bin/env bash
    CURRENT_YEAR=$(date +%G)
    CURRENT_WEEK=$(date +%V)
    THREE_WEEKS_AGO=$(date -d '3 weeks ago' +%G-W%V)
    FROM_YEAR=$(echo "$THREE_WEEKS_AGO" | cut -d'-' -f1)
    FROM_WEEK=$(echo "$THREE_WEEKS_AGO" | cut -d'W' -f2)
    cargo run -- archive-range \
        --from-year "$FROM_YEAR" --from-week "$FROM_WEEK" \
        --to-year "$CURRENT_YEAR" --to-week "$CURRENT_WEEK" \
        --output {{output}}

# Archive a custom range of weeks to parquet
archive-range from_year from_week to_year to_week output=conversations_path:
    cargo run -- archive-range \
        --from-year {{from_year}} --from-week {{from_week}} \
        --to-year {{to_year}} --to-week {{to_week}} \
        --output {{output}}

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
    ./scripts/run-duckdb-sample-queries.sh

# Run smoke tests for all CLI commands
smoke-test:
    ./scripts/smoke-test.sh

# Build the archive server binary
build-server:
    cargo build --features server --bin slack-archive-server

# Run the archive server with a config file
run-server config="config.toml":
    cargo run --features server --bin slack-archive-server -- serve {{config}}

# Run smoke tests for the archive server
server-smoke-test:
    ./scripts/server-smoke-test.sh

# Start a local archive server for testing the client with local parquet files
start-local-archive-server port="8080" base_path=".":
    ./scripts/start-local-archive-server.sh {{port}} {{base_path}} {{client_dir}}

# JS client directory
client_dir := "tools/slack-archive-client"

# Build the JS client library (ESM + TypeScript declarations)
build-client:
    cd {{client_dir}} && bun install && bun run dist

# Clean the JS client dist folder
clean-client:
    rm -rf {{client_dir}}/dist

# Type check the JS client
typecheck-client:
    cd {{client_dir}} && bun run typecheck

# Build all binaries for release and create distribution package
dist:
    ./scripts/dist.sh {{client_dir}}
