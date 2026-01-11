# Slack Utils - Distribution Package

Pre-built binaries for exporting, archiving, and querying Slack data.

## Requirements

- Slack API token (environment variable `SLACK_TOKEN`) for export commands
- Meilisearch (optional, for full-text search)

## Binaries

### slack-utils

Main CLI tool for exporting data from Slack.

**Export Commands**

```bash
export SLACK_TOKEN="xoxb-your-token"

# Export users
./slack-utils export-users --output users --format json
./slack-utils export-users --output users --format parquet

# Export channels
./slack-utils export-channels --output channels --format json

# Export conversations by date range
./slack-utils export-conversations --from 2024-01-01 --to 2024-01-31 --output conversations --format json

# Export conversations for a specific ISO week
./slack-utils export-conversations-week --year 2024 --week 42 --output conversations

# Export custom emojis
./slack-utils export-emojis --output emojis.json --folder emojis/
```

**Archive Commands**

Archives store conversations as parquet files in Hive-partitioned directories (`year=YYYY/week=WW/threads.parquet`). Existing weeks are skipped.

```bash
# Archive a single week
./slack-utils archive-range --from-year 2024 --from-week 42

# Archive a range of weeks
./slack-utils archive-range --from-year 2024 --from-week 1 --to-week 52

# Archive across year boundary
./slack-utils archive-range --from-year 2024 --from-week 50 --to-year 2025 --to-week 10 --output ./archive
```

**Processing Commands**

```bash
# Download attachments from exported conversations
./slack-utils download-attachments --input conversations.json --output attachments/

# Convert conversations to markdown
./slack-utils export-markdown --conversations selected-conversations.json --users users.json --channels channels.json --output output.md

# Create searchable index
./slack-utils export-index --conversations conversations.json --users users.json --channels channels.json --output index.json
```

**Meilisearch Commands**

```bash
# Import index to Meilisearch
./slack-utils import-index-meilisearch --input index.json --url http://localhost:7700 --api-key KEY --index-name slack

# Import with clear (atomic swap)
./slack-utils import-index-meilisearch --input index.json --url http://localhost:7700 --api-key KEY --index-name slack --clear

# Query
./slack-utils query-meilisearch "search term" --url http://localhost:7700 --api-key KEY --index-name slack --limit 20
```

**Interactive TUI**

```bash
./slack-utils ui
```

### slack-utils-duckdb

Query parquet exports using DuckDB. Data is exposed as a table named `data`.

```bash
# Query conversation threads (default path)
./slack-utils-duckdb query "SELECT * FROM data LIMIT 10"

# Query with custom parquet path
./slack-utils-duckdb query "SELECT * FROM data" --parquet users.parquet
./slack-utils-duckdb query "SELECT * FROM data" --parquet channels.parquet

# Messages per channel
./slack-utils-duckdb query "SELECT channel_name, COUNT(*) as msg_count FROM data GROUP BY channel_name ORDER BY msg_count DESC"

# Filter by Hive partition
./slack-utils-duckdb query "SELECT * FROM data WHERE year = 2024 AND week = 42 LIMIT 20"

# Search message content
./slack-utils-duckdb query "SELECT channel_name, user, text FROM data WHERE text LIKE '%deploy%'"

# Thread reply counts
./slack-utils-duckdb query "SELECT channel_name, thread_ts, COUNT(*) as replies FROM data WHERE is_reply GROUP BY channel_name, thread_ts ORDER BY replies DESC LIMIT 10"
```

Default parquet path: `conversations/year=*/week=*/*.parquet`

### slack-archive-server

HTTP server for serving parquet files.

**Configuration**

Create a TOML config file (see included `config.example.toml`).

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
./slack-archive-server serve config.toml
```

**API Endpoints**

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/archive/users` | Returns `users.parquet` |
| GET | `/archive/channels` | Returns `channels.parquet` |
| GET | `/archive/threads-in-range?from=YYYY-MM-DD&to=YYYY-MM-DD` | Lists available year/week partitions |
| GET | `/archive/threads?year=YYYY&week=WW` | Returns `threads.parquet` for a week |
| POST | `/archive/search?query=<text>&limit=<n>` | Search via Meilisearch |

**Example API Calls**

```bash
curl -O http://localhost:8080/archive/users
curl -O http://localhost:8080/archive/channels
curl "http://localhost:8080/archive/threads-in-range?from=2024-01-01&to=2024-01-31"
curl -O "http://localhost:8080/archive/threads?year=2024&week=3"
curl -X POST "http://localhost:8080/archive/search?query=deployment&limit=20"
```

### slack-archive-client

JavaScript/TypeScript client library for the `slack-archive-server` HTTP API. Located in `slack-archive-client/`.

**Installation**

```bash
# Copy to your project
cp -r slack-archive-client /path/to/your/project/

# Or use as a local dependency in package.json
"dependencies": {
  "slack-archive-client": "file:./slack-archive-client"
}
```

**Usage**

```typescript
import { SlackArchiveClient } from './slack-archive-client/index.js';

const client = new SlackArchiveClient({
  baseUrl: 'http://localhost:8080'
});

// Check server connectivity
const isUp = await client.ping();

// Get available thread partitions in a date range
const { available } = await client.getThreadsInRange('2024-01-01', '2024-12-31');

// Download parquet files as ArrayBuffer
const usersParquet = await client.getUsers();
const channelsParquet = await client.getChannels();
const threadsParquet = await client.getThreads(2024, 3); // year, week

// Search messages (requires Meilisearch configured on server)
const results = await client.search('deployment', 20);
```

**API**

| Method | Description | Returns |
|--------|-------------|---------|
| `ping()` | Check if server is reachable | `Promise<boolean>` |
| `getUsers()` | Download users.parquet | `Promise<ArrayBuffer>` |
| `getChannels()` | Download channels.parquet | `Promise<ArrayBuffer>` |
| `getThreadsInRange(from, to)` | List available year/week partitions | `Promise<ThreadsInRangeResponse>` |
| `getThreads(year, week)` | Download threads.parquet for a week | `Promise<ArrayBuffer>` |
| `search(query, limit?)` | Search messages via Meilisearch | `Promise<SearchResponse>` |

**Error Handling**

```typescript
import { SlackArchiveClient, SlackArchiveError } from './slack-archive-client/index.js';

try {
  await client.getUsers();
} catch (error) {
  if (error instanceof SlackArchiveError) {
    console.log('Status code:', error.statusCode);
    console.log('Server message:', error.serverError);
  }
}
```

**Contents**

- `index.js` - ESM module
- `index.d.ts` - TypeScript type declarations
- `client.d.ts` - Client class types
- `types.d.ts` - Data type definitions

## Directory Structure

**Expected archive layout:**

```
archive/
├── users.parquet
├── channels.parquet
└── conversations/
    └── year=YYYY/
        └── week=WW/
            └── threads.parquet
```

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
