# slack-utils

Command-line utilities for exporting, archiving, and querying Slack data.

## Requirements

- Rust (edition 2024)
- Slack API token (environment variable `SLACK_TOKEN`)
- just (optional, for running tasks)
- Meilisearch (optional, for full-text search)

## Building

The project has three binaries, each behind a feature flag.

```bash
# Main CLI with TUI
cargo build --features tui

# DuckDB query tool
cargo build --features duckdb --bin slack-utils-duckdb

# HTTP server
cargo build --features server --bin slack-archive-server

# Build all for release
just dist
```

## Binaries

### slack-utils

Main CLI tool for exporting data from Slack.

**Export Commands**

```bash
# Export users
slack-utils export-users --output users --format json
slack-utils export-users --output users --format parquet

# Export channels
slack-utils export-channels --output channels --format json

# Export conversations by date range
slack-utils export-conversations --from 2024-01-01 --to 2024-01-31 --output conversations --format json

# Export conversations for a specific ISO week
slack-utils export-conversations-week --year 2024 --week 42 --output conversations

# Export custom emojis
slack-utils export-emojis --output emojis.json --folder emojis/
```

**Archive Commands**

Archives store conversations as parquet files in Hive-partitioned directories (`year=YYYY/week=WW/threads.parquet`). Existing weeks are skipped.

```bash
# Archive a single week
slack-utils archive-range --from-year 2024 --from-week 42

# Archive a range of weeks
slack-utils archive-range --from-year 2024 --from-week 1 --to-week 52

# Archive across year boundary
slack-utils archive-range --from-year 2024 --from-week 50 --to-year 2025 --to-week 10 --output ./archive
```

**Processing Commands**

```bash
# Download attachments from exported conversations
slack-utils download-attachments --input conversations.json --output attachments/

# Convert conversations to markdown
slack-utils export-markdown --conversations selected-conversations.json --users users.json --channels channels.json --output output.md

# Create searchable index
slack-utils export-index --conversations conversations.json --users users.json --channels channels.json --output index.json
```

**Meilisearch Commands**

```bash
# Import index to Meilisearch
slack-utils import-index-meilisearch --input index.json --url http://localhost:7700 --api-key KEY --index-name slack

# Import with clear (atomic swap)
slack-utils import-index-meilisearch --input index.json --url http://localhost:7700 --api-key KEY --index-name slack --clear

# Query
slack-utils query-meilisearch "search term" --url http://localhost:7700 --api-key KEY --index-name slack --limit 20
```

**Interactive TUI**

```bash
slack-utils ui
```

### slack-utils-duckdb

Query parquet exports using DuckDB. Data is exposed as a table named `data`.

```bash
# Query conversation threads (default path)
slack-utils-duckdb query "SELECT * FROM data LIMIT 10"

# Query with custom parquet path
slack-utils-duckdb query "SELECT * FROM data" --parquet users.parquet
slack-utils-duckdb query "SELECT * FROM data" --parquet channels.parquet

# Messages per channel
slack-utils-duckdb query "SELECT channel_name, COUNT(*) as msg_count FROM data GROUP BY channel_name ORDER BY msg_count DESC"

# Filter by Hive partition
slack-utils-duckdb query "SELECT * FROM data WHERE year = 2024 AND week = 42 LIMIT 20"

# Search message content
slack-utils-duckdb query "SELECT channel_name, user, text FROM data WHERE text LIKE '%deploy%'"

# Thread reply counts
slack-utils-duckdb query "SELECT channel_name, thread_ts, COUNT(*) as replies FROM data WHERE is_reply GROUP BY channel_name, thread_ts ORDER BY replies DESC LIMIT 10"
```

Default parquet path: `conversations/year=*/week=*/*.parquet`

### slack-archive-server

HTTP server for serving parquet files.

**Configuration**

Create a TOML config file. See `resources/sample-server-config.toml` for documentation.

```toml
[server]
host = "127.0.0.1"
port = 8080
# static_assets = "./static"

[slack-archive]
base_path = "./archive"

# Optional: enable search
# [meilisearch]
# url = "http://localhost:7700"
# api-key = "your-api-key"
# index-name = "slack"
```

**Running**

```bash
slack-archive-server serve config.toml
```

**API Endpoints**

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/archive/users` | Returns `users.parquet` |
| GET | `/archive/channels` | Returns `channels.parquet` |
| GET | `/archive/threads-in-range?from=YYYY-MM-DD&to=YYYY-MM-DD` | Lists available year/week partitions |
| GET | `/archive/threads?year=YYYY&week=WW` | Returns `threads.parquet` for a week |
| POST | `/archive/search?query=<text>&limit=<n>` | Search via Meilisearch |

**Expected Directory Structure**

```
archive/
├── users.parquet
├── channels.parquet
└── conversations/
    └── year=YYYY/
        └── week=WW/
            └── threads.parquet
```

## Justfile Targets

Run `just` to list all targets.

**Export from Slack**

| Target | Description |
|--------|-------------|
| `just export-users [output] [format]` | Export users (default: users, json) |
| `just export-channels [output] [format]` | Export channels (default: channels, json) |
| `just export-conversations [output] [format]` | Export last 7 days |
| `just export-conversations-range <from> <to> [output] [format]` | Export date range |
| `just export-conversations-week [output] [format]` | Export current ISO week |
| `just export-conversations-week-custom <year> <week> [output] [format]` | Export specific week |
| `just export-emojis [output] [folder]` | Export custom emojis |

**Archive**

| Target | Description |
|--------|-------------|
| `just archive-last-4-weeks [output]` | Archive last 4 weeks as parquet |
| `just archive-range <from_year> <from_week> <to_year> <to_week> [output]` | Archive week range |

**Processing**

| Target | Description |
|--------|-------------|
| `just download-attachments [input] [output]` | Download attachments |
| `just export-markdown [conversations] [users] [channels] [output]` | Convert to markdown |
| `just export-index [conversations] [users] [channels] [output]` | Create search index |

**Meilisearch**

| Target | Description |
|--------|-------------|
| `just import-meilisearch <api_key> [input] [url] [index_name]` | Import index |
| `just import-meilisearch-clear <api_key> [input] [url] [index_name]` | Import with clear |
| `just query-meilisearch <query> <api_key> [url] [index_name]` | Search |
| `just start-meilisearch` | Start server (requires `$MS_MASTER_KEY`) |

**DuckDB**

| Target | Description |
|--------|-------------|
| `just build-duckdb` | Build DuckDB binary |
| `just query-duckdb <query> [parquet]` | Query conversations |
| `just query-duckdb-users <query> [parquet]` | Query users.parquet |
| `just query-duckdb-channels <query> [parquet]` | Query channels.parquet |
| `just run-duckdb-sample-queries` | Run example queries |

**Server**

| Target | Description |
|--------|-------------|
| `just build-server` | Build server binary |
| `just run-server [config]` | Run server |
| `just server-smoke-test` | Run server smoke tests |

**Other**

| Target | Description |
|--------|-------------|
| `just ui` | Launch interactive TUI |
| `just smoke-test` | Run CLI smoke tests |
| `just dist` | Build release binaries |

## Justfile Defaults

Variables can be overridden with `--set`:

```bash
just --set ms_url "http://other:7700" import-meilisearch "API_KEY"
```

| Variable | Default |
|----------|---------|
| `conversations_path` | `conversations` |
| `users_path` | `users` |
| `channels_path` | `channels` |
| `attachments_dir` | `attachments` |
| `emojis_file` | `emojis.json` |
| `emojis_dir` | `emojis` |
| `index_file` | `conversation-index.json` |
| `markdown_file` | `selected-conversations.md` |
| `default_format` | `json` |
| `ms_url` | `http://localhost:7700` |
| `ms_index` | `slack` |
| `conversations_parquet` | `conversations/year=*/week=*/*.parquet` |
| `users_parquet` | `users.parquet` |
| `channels_parquet` | `channels.parquet` |

## Output Formats

- **json**: Single file with all data
- **parquet**: Binary columnar format, efficient for queries

For parquet exports of conversations, files are organized with Hive partitioning:

```
conversations/
└── year=2024/
    └── week=42/
        └── threads.parquet
```

## Rate Limiting

Slack API operations handle rate limits automatically. The CLI displays wait times when rate limited. Operations retry up to 5 times using the `Retry-After` header.
