# slack-archive-client

JavaScript/TypeScript client library for `slack-archive-server`.

## Features

- **SlackArchiveClient**: Fetch parquet files via HTTP API or static file paths
- **SlackArchiveDuckDB**: Load parquet files into DuckDB WASM for in-browser SQL queries
- **Browser caching**: Optional IndexedDB-based caching for offline access
- **Two client modes**: API mode (server endpoints) or Static mode (direct file access)
- Full TypeScript support with type definitions
- Progress callbacks for data loading operations
- Works in modern browsers and Node.js/Bun/Deno

## Installation

```bash
bun add slack-archive-client

# For DuckDB support:
bun add @duckdb/duckdb-wasm
```

## Usage

### Basic API Client

```typescript
import { SlackArchiveClient } from "slack-archive-client";

const client = new SlackArchiveClient({
  baseUrl: "http://localhost:8080",
});

// Fetch parquet files as ArrayBuffer
const usersBuffer = await client.getUsers();
const channelsBuffer = await client.getChannels();

// Get available thread partitions for a date range
const { available } = await client.getThreadsInRange("2024-01-01", "2024-03-31");
// [{ year: 2024, week: 1 }, { year: 2024, week: 2 }, ...]

// Fetch threads for a specific week
const threadsBuffer = await client.getThreads(2024, 3);

// Search messages (requires Meilisearch configured on server)
const results = await client.search("deployment", 20);

// Check server connectivity
const isUp = await client.ping();
```

### Static Mode (No Server API)

For static file hosting (e.g., GitHub Pages, S3), use `mode: "static"` to fetch parquet files directly without server API endpoints:

```typescript
import { SlackArchiveClient } from "slack-archive-client";

const client = new SlackArchiveClient({
  baseUrl: "https://your-static-site.com",
  mode: "static",  // Direct file access mode
});

// Fetches /users.parquet directly
const usersBuffer = await client.getUsers();

// Fetches /channels.parquet directly
const channelsBuffer = await client.getChannels();

// In static mode, probes for available week partitions via HEAD requests
// Checks files like /conversations/year=2024/week=01/threads.parquet
const { available } = await client.getThreadsInRange("2024-01-01", "2024-03-31");

// Fetches /conversations/year=2024/week=03/threads.parquet
const threadsBuffer = await client.getThreads(2024, 3);

// Note: search() is not available in static mode (requires Meilisearch)
```

Static mode expects this file structure on your static host:

```
/
├── users.parquet
├── channels.parquet
└── conversations/
    └── year=2024/
        ├── week=01/threads.parquet
        ├── week=02/threads.parquet
        └── ...
```

### DuckDB Client (In-Browser SQL)

```typescript
import { SlackArchiveClient, SlackArchiveDuckDB } from "slack-archive-client";

const client = new SlackArchiveClient({ baseUrl: "http://localhost:8080" });
const db = new SlackArchiveDuckDB({ client });

// Initialize DuckDB WASM
await db.init();

// Load all data with progress tracking
await db.loadAll("2024-01-01", "2024-03-31", (progress) => {
  console.log(`${progress.phase}: ${progress.current}/${progress.total}`);
});

// Run SQL queries
const result = await db.query<{ channel_name: string; msg_count: bigint }>(`
  SELECT channel_name, COUNT(*) as msg_count
  FROM threads
  GROUP BY channel_name
  ORDER BY msg_count DESC
  LIMIT 10
`);

console.log(result.rows);
console.log(`Query took ${result.executionTimeMs}ms`);

// Clean up
await db.close();
```

### Loading Tables Separately

```typescript
await db.init();

// Load individual tables
const userCount = await db.loadUsers();
const channelCount = await db.loadChannels();
const threadCount = await db.loadThreads("2024-01-01", "2024-03-31");

// Check what's loaded
db.getLoadedTables();        // Set { "users", "channels", "threads" }
db.isTableLoaded("threads"); // true

// Get row counts
const info = await db.getTableInfo();
// [{ name: "users", rowCount: 150 }, ...]

// Drop and reload
await db.dropTable("threads");
await db.loadThreads("2024-06-01", "2024-06-30");
```

### Browser Caching

Enable persistent caching of parquet files using IndexedDB for faster repeat loads and offline access:

```typescript
import { SlackArchiveClient, SlackArchiveDuckDB } from "slack-archive-client";
import { createCachingFetch, IndexedDBCache } from "slack-archive-client";

// Create cache storage
const cache = new IndexedDBCache();

// Create caching fetch wrapper with TTL settings
const cachedFetch = createCachingFetch(fetch, cache, {
  ttl: {
    "**/users.parquet": 24 * 60 * 60 * 1000,      // 24 hours
    "**/channels.parquet": 24 * 60 * 60 * 1000,   // 24 hours
    "**/conversations/**": 7 * 24 * 60 * 60 * 1000, // 7 days (immutable)
  },
  maxSize: 500 * 1024 * 1024, // 500MB limit
});

// Use cached fetch with client
const client = new SlackArchiveClient({
  baseUrl: "http://localhost:8080",
  fetch: cachedFetch,
});

const db = new SlackArchiveDuckDB({ client });
await db.init();
await db.loadAll("2024-01-01", "2024-03-31"); // Cached after first load
```

**Disable caching** (default behavior):

```typescript
const client = new SlackArchiveClient({
  baseUrl: "http://localhost:8080",
  // No fetch override = no caching
});
```

**Monitor cache events**:

```typescript
const cachedFetch = createCachingFetch(fetch, cache, {
  onCacheEvent: (event) => {
    console.log(`Cache ${event.type}: ${event.url}`);
    if (event.type === "hit") {
      console.log(`  Age: ${event.age}ms`);
    }
  },
});
```

**Manual cache management**:

```typescript
const cache = new IndexedDBCache();

// Get cache size
const size = await cache.size();
console.log(`Cache: ${(size / 1024 / 1024).toFixed(2)} MB`);

// List cached files
const keys = await cache.keys();

// Clear all cached data
await cache.clear();
```

## API Reference

### SlackArchiveClient

```typescript
new SlackArchiveClient(options: {
  baseUrl: string;
  mode?: "api" | "static";  // Default: "api"
  fetch?: typeof fetch;
})
```

| Method | Returns | Description |
|--------|---------|-------------|
| `getUsers()` | `Promise<ArrayBuffer>` | Fetch `users.parquet` |
| `getChannels()` | `Promise<ArrayBuffer>` | Fetch `channels.parquet` |
| `getThreadsInRange(from, to)` | `Promise<{ available: YearWeek[] }>` | List available partitions |
| `getThreads(year, week)` | `Promise<ArrayBuffer>` | Fetch `threads.parquet` |
| `search(query, limit?)` | `Promise<SearchResponse>` | Search via Meilisearch (API mode only) |
| `ping()` | `Promise<boolean>` | Check server connectivity |
| `getMode()` | `ClientMode` | Get current client mode |

**Mode differences:**

| Operation | API Mode | Static Mode |
|-----------|----------|-------------|
| `getUsers()` | `GET /archive/users` | `GET /users.parquet` |
| `getChannels()` | `GET /archive/channels` | `GET /channels.parquet` |
| `getThreadsInRange()` | `GET /archive/threads-in-range` | HEAD probes for each week |
| `getThreads(y, w)` | `GET /archive/threads?year=...` | `GET /conversations/year=.../week=.../threads.parquet` |
| `search()` | `POST /archive/search` | Not available (throws error) |

### SlackArchiveDuckDB

```typescript
new SlackArchiveDuckDB(options: {
  client: SlackArchiveClient;
  bundles?: DuckDBBundles;
})
```

| Method | Returns | Description |
|--------|---------|-------------|
| `init()` | `Promise<void>` | Initialize DuckDB WASM |
| `loadUsers(onProgress?)` | `Promise<number>` | Load into `users` table |
| `loadChannels(onProgress?)` | `Promise<number>` | Load into `channels` table |
| `loadThreads(from, to, onProgress?)` | `Promise<number>` | Load into `threads` table |
| `loadAll(from, to, onProgress?)` | `Promise<{ users, channels, threads }>` | Load all tables |
| `query<T>(sql)` | `Promise<QueryResult<T>>` | Execute SQL |
| `getLoadedTables()` | `Set<TableName>` | Get loaded table names |
| `isTableLoaded(name)` | `boolean` | Check if table loaded |
| `getTableInfo()` | `Promise<LoadedTableInfo[]>` | Get table row counts |
| `dropTable(name)` | `Promise<void>` | Drop a table |
| `close()` | `Promise<void>` | Clean up resources |

### Types

```typescript
type ClientMode = "api" | "static";

interface QueryResult<T> {
  rows: T[];
  schema: Array<{ name: string; type: string }>;
  executionTimeMs: number;
}

interface LoadProgress {
  phase: "users" | "channels" | "threads";
  current: number;
  total: number;
  message: string;
}

type TableName = "users" | "channels" | "threads";

interface YearWeek {
  year: number;
  week: number;
}

interface SearchResponse {
  hits: IndexEntry[];
  processing_time_ms: number;
  estimated_total_hits?: number;
}

// Cache types
interface CacheStorage {
  get(key: string): Promise<CacheEntry | null>;
  set(key: string, entry: CacheEntry): Promise<void>;
  delete(key: string): Promise<boolean>;
  clear(): Promise<void>;
  keys(): Promise<string[]>;
  size(): Promise<number>;
}

interface CachingFetchOptions {
  ttl?: Record<string, number>;     // TTL per URL pattern
  defaultTtl?: number;              // Default: 1 hour
  maxSize?: number;                 // Default: 500MB
  onCacheEvent?: (event: CacheEvent) => void;
}

interface CacheEvent {
  type: "hit" | "miss" | "store" | "evict" | "error";
  url: string;
  size?: number;
  age?: number;
}
```

### Error Handling

```typescript
import { SlackArchiveClient, SlackArchiveError, DuckDBClientError } from "slack-archive-client";

try {
  await client.getUsers();
} catch (error) {
  if (error instanceof SlackArchiveError) {
    console.log("Status:", error.statusCode);
    console.log("Message:", error.serverError);
  }
}

try {
  await db.query("SELECT * FROM nonexistent");
} catch (error) {
  if (error instanceof DuckDBClientError) {
    console.log("DuckDB error:", error.message);
  }
}
```

## Server Compatibility

Works with both:

- **slack-archive-server** (Rust) - Production HTTP server
- **web-duckdb-wasm/serve.js** (Bun) - Development server

API endpoints:

| Endpoint | Description |
|----------|-------------|
| `GET /archive/users` | Returns `users.parquet` |
| `GET /archive/channels` | Returns `channels.parquet` |
| `GET /archive/threads-in-range?from=...&to=...` | List partitions |
| `GET /archive/threads?year=...&week=...` | Returns `threads.parquet` |
| `POST /archive/search?query=...&limit=...` | Meilisearch query |

## Development

```bash
just install     # Install dependencies
just typecheck   # Type check
just build       # Build library
just check       # typecheck + build

# Test with web UI
just serve-test-ui /path/to/parquet/files

# Test with slack-archive-server
just serve-with-server /path/to/parquet/files
```

## Requirements

- Modern JavaScript runtime (ES2022+)
- `@duckdb/duckdb-wasm >= 1.32.0` for DuckDB features

## License

MIT
