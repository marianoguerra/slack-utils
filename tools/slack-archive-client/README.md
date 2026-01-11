# Slack Archive Client

JavaScript/TypeScript client library for the `slack-archive-server` HTTP API.

## Installation

```bash
# Using bun
bun add slack-archive-client

# Or link locally during development
cd tools/slack-archive-client
bun install
bun run build
```

## Building

```bash
# Build ESM module and TypeScript declarations
bun run dist

# Or step by step
bun run build        # Builds both JS and type declarations
bun run clean        # Remove dist folder
bun run typecheck    # Check types without emitting
```

## Usage

```typescript
import { SlackArchiveClient } from 'slack-archive-client';

const client = new SlackArchiveClient({
  baseUrl: 'http://localhost:8080'
});

// Check server connectivity
const isUp = await client.ping();

// Get available thread partitions in a date range
const { available } = await client.getThreadsInRange('2024-01-01', '2024-12-31');
console.log('Available weeks:', available);

// Download parquet files as ArrayBuffer
const usersParquet = await client.getUsers();
const channelsParquet = await client.getChannels();
const threadsParquet = await client.getThreads(2024, 3); // year, week

// Search messages (requires Meilisearch configured on server)
const results = await client.search('deployment', 20);
console.log('Hits:', results.hits);
console.log('Processing time:', results.processing_time_ms, 'ms');
```

## API Reference

### `SlackArchiveClient`

#### Constructor

```typescript
new SlackArchiveClient(options: SlackArchiveClientOptions)
```

Options:
- `baseUrl` (required): Base URL of the slack-archive-server
- `fetch` (optional): Custom fetch implementation

#### Methods

| Method | Description | Returns |
|--------|-------------|---------|
| `ping()` | Check if server is reachable | `Promise<boolean>` |
| `getUsers()` | Download users.parquet | `Promise<ArrayBuffer>` |
| `getChannels()` | Download channels.parquet | `Promise<ArrayBuffer>` |
| `getThreadsInRange(from, to)` | List available year/week partitions | `Promise<ThreadsInRangeResponse>` |
| `getThreads(year, week)` | Download threads.parquet for a week | `Promise<ArrayBuffer>` |
| `search(query, limit?)` | Search messages via Meilisearch | `Promise<SearchResponse>` |

### Error Handling

The client throws `SlackArchiveError` for HTTP errors:

```typescript
import { SlackArchiveClient, SlackArchiveError } from 'slack-archive-client';

try {
  await client.getUsers();
} catch (error) {
  if (error instanceof SlackArchiveError) {
    console.log('Status code:', error.statusCode);
    console.log('Server message:', error.serverError);
  }
}
```

## Types

```typescript
interface YearWeek {
  year: number;
  week: number;
}

interface ThreadsInRangeResponse {
  available: YearWeek[];
}

interface SearchResponse {
  hits: IndexEntry[];
  processing_time_ms: number;
  estimated_total_hits?: number;
}

interface IndexEntry {
  id: string;
  ts: string;
  date: string;
  text: string;
  users: IndexUser[];
  channel: IndexChannel;
}

interface IndexUser {
  id: string;
  name: string;
}

interface IndexChannel {
  id: string;
  name: string;
}
```

## Examples

### Browser Smoke Test

Open `examples/index.html` in a browser to run a visual smoke test against a running server. Make sure to:

1. Build the client first: `bun run dist`
2. Start the slack-archive-server
3. Open the HTML file and check the browser console for detailed results

### Using with DuckDB-WASM

The parquet files can be loaded directly into DuckDB-WASM:

```typescript
import { SlackArchiveClient } from 'slack-archive-client';
import * as duckdb from '@duckdb/duckdb-wasm';

const client = new SlackArchiveClient({ baseUrl: 'http://localhost:8080' });

// Fetch parquet data
const usersBuffer = await client.getUsers();

// Register with DuckDB
await db.registerFileBuffer('users.parquet', new Uint8Array(usersBuffer));

// Query
const result = await conn.query(`
  SELECT name, real_name, email
  FROM 'users.parquet'
  WHERE is_bot = false
`);
```

## Development

```bash
# Install dependencies
bun install

# Type check
bun run typecheck

# Build
bun run dist
```
