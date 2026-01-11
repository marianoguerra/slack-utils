# Slack Utils - Distribution Package

This package contains pre-built binaries for Slack archive utilities.

## Binaries Included

### slack-utils

The main CLI tool for interacting with Slack archives.

**Usage:**
```bash
# Show help
./slack-utils --help

# Launch interactive TUI
./slack-utils ui

# Export users (requires SLACK_TOKEN env var)
export SLACK_TOKEN="xoxb-your-token"
./slack-utils export-users --output users

# Export channels
./slack-utils export-channels --output channels

# Export conversations from last 7 days
./slack-utils export-conversations --from 2024-01-01 --to 2024-01-07 --output conversations

# Export conversations for current work week
./slack-utils export-conversations-week --output conversations

# Archive conversations to parquet (by week range)
./slack-utils archive-range --from-year 2024 --from-week 1 --to-year 2024 --to-week 4 --output archive

# Export to markdown
./slack-utils export-markdown --conversations conv.json --users users.json --channels channels.json --output output.md

# Export searchable index
./slack-utils export-index --conversations conv.json --users users.json --channels channels.json --output index.json

# Import to Meilisearch
./slack-utils import-index-meilisearch --input index.json --url http://localhost:7700 --api-key KEY --index-name slack

# Query Meilisearch
./slack-utils query-meilisearch "search term" --url http://localhost:7700 --api-key KEY --index-name slack

# Export custom emojis
./slack-utils export-emojis --output emojis.json --folder emojis/

# Download attachments
./slack-utils download-attachments --input conversations.json --output attachments/
```

### slack-utils-duckdb

Query parquet exports using DuckDB SQL.

**Usage:**
```bash
# Query conversations (default path: conversations/year=*/week=*/*.parquet)
./slack-utils-duckdb query "SELECT * FROM data LIMIT 10"

# Query with custom parquet path
./slack-utils-duckdb query "SELECT * FROM data" --parquet users.parquet
./slack-utils-duckdb query "SELECT * FROM data" --parquet channels.parquet

# Example queries
./slack-utils-duckdb query "SELECT channel_name, COUNT(*) as count FROM data GROUP BY channel_name ORDER BY count DESC"
./slack-utils-duckdb query "SELECT * FROM data WHERE year = 2024 AND week = 3 LIMIT 20"
./slack-utils-duckdb query "SELECT user, COUNT(*) FROM data GROUP BY user ORDER BY 2 DESC LIMIT 10"
```

### slack-archive-server

HTTP server for serving Slack archive parquet files.

**Usage:**
```bash
# Start the server with a config file
./slack-archive-server serve config.toml
```

**Configuration (config.toml):**
```toml
[server]
host = "127.0.0.1"
port = 8080
# static_assets = "./static"  # Optional

[slack-archive]
base_path = "./archive"

# Optional: Enable search via Meilisearch
# [meilisearch]
# url = "http://localhost:7700"
# api-key = "your-api-key"
# index-name = "slack"
```

**API Endpoints:**
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/archive/users` | Download users.parquet |
| GET | `/archive/channels` | Download channels.parquet |
| GET | `/archive/threads-in-range?from=YYYY-MM-DD&to=YYYY-MM-DD` | List available year/weeks |
| GET | `/archive/threads?year=YYYY&week=WW` | Download threads.parquet for week |
| POST | `/archive/search?query=text&limit=20` | Search via Meilisearch |

**Example API calls:**
```bash
# Get users
curl -O http://localhost:8080/archive/users

# Get channels
curl -O http://localhost:8080/archive/channels

# List available weeks
curl "http://localhost:8080/archive/threads-in-range?from=2024-01-01&to=2024-01-31"

# Get threads for a specific week
curl -O "http://localhost:8080/archive/threads?year=2024&week=3"

# Search (if meilisearch configured)
curl -X POST "http://localhost:8080/archive/search?query=deployment&limit=20"
```

## Archive Directory Structure

The server expects this directory structure:
```
archive/
├── users.parquet
├── channels.parquet
└── conversations/
    └── year=YYYY/
        └── week=WW/
            └── threads.parquet
```

## Environment Variables

- `SLACK_TOKEN` - Slack Bot User OAuth Token (required for export commands)

## Requirements

- For `slack-utils` and `slack-archive-server`: No additional dependencies
- For `slack-utils-duckdb`: DuckDB is bundled, no additional dependencies
- For Meilisearch search: A running Meilisearch instance

## License

See the main repository for license information.
