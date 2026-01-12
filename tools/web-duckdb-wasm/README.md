# web-duckdb-wasm

Browser-based SQL query tool for Slack parquet exports using DuckDB WASM.

## Features

- Run SQL queries on Slack data entirely in the browser (WebAssembly)
- Auto-loads users and channels on startup
- Date range picker for loading conversation threads
- Predefined sample queries for common statistics
- Compatible with both local dev server and `slack-archive-server`

## Quick Start

```bash
# Install dependencies
bun install

# Start dev server (requires parquet files)
just serve /path/to/parquet/files

# Or with hot reload
just dev /path/to/parquet/files
```

Open http://localhost:3000

## Architecture

This app uses:
- **slack-archive-client** - Fetches parquet files from the server
- **SlackArchiveDuckDB** - Loads parquet into DuckDB WASM for SQL queries
- **@duckdb/duckdb-wasm** - In-browser SQL engine

The app is bundled with `slack-archive-client`, but `@duckdb/duckdb-wasm` is loaded at runtime via import maps.

## Import Maps

Since `@duckdb/duckdb-wasm` is an external dependency, you need to provide it via an import map. The bundled `app.js` contains:

```javascript
import * as duckdb from "@duckdb/duckdb-wasm";
```

To resolve this import in the browser, add an import map to your HTML:

```html
<script type="importmap">
{
  "imports": {
    "@duckdb/duckdb-wasm": "https://cdn.jsdelivr.net/npm/@duckdb/duckdb-wasm@1.32.0/+esm"
  }
}
</script>
<script type="module" src="app.js"></script>
```

Or use a local path if you've installed the package:

```html
<script type="importmap">
{
  "imports": {
    "@duckdb/duckdb-wasm": "./node_modules/@duckdb/duckdb-wasm/dist/duckdb-browser.mjs"
  }
}
</script>
```

The dev servers (`serve.js`, `static-file-server.js`) bundle the app with dependencies included, so import maps are only needed for standalone deployments.

## Server Modes

### 1. API Server (`serve.js`) - Recommended

Provides API endpoints compatible with `slack-archive-server`:

```bash
just serve /path/to/parquet/files
# or
bun serve.js --path /path/to/parquet/files --port 3000
```

API endpoints:
| Endpoint | Description |
|----------|-------------|
| `GET /archive/users` | Returns `users.parquet` |
| `GET /archive/channels` | Returns `channels.parquet` |
| `GET /archive/threads-in-range?from=...&to=...` | List available partitions |
| `GET /archive/threads?year=...&week=...` | Returns `threads.parquet` |

### 2. Static File Server (`static-file-server.js`)

Simple static server for pre-built deployments:

```bash
just serve-static /path/to/parquet/files
# or
bun static-file-server.js --path /path/to/parquet/files
```

### 3. With slack-archive-server

Test the app with the Rust server:

```bash
just serve-with-server /path/to/parquet/files
```

This builds the app and starts `slack-archive-server` with this directory as static assets.

## CLI Options

```
Options:
  -p, --path <path>   Base path containing parquet files (default: .)
  --port <port>       Server port (default: 3000)
  -h, --help          Show help
```

## Required File Structure

```
/path/to/files/
├── users.parquet
├── channels.parquet
└── conversations/
    └── year=2024/
        ├── week=01/
        │   └── threads.parquet
        ├── week=02/
        │   └── threads.parquet
        └── ...
```

Export data with:

```bash
slack-utils export-users --format parquet
slack-utils export-channels --format parquet
slack-utils export-conversations --format parquet
```

## URL Query Parameters

Pre-fill date range and auto-load data via URL:

### By Week

```
?fromYear=2024&fromWeek=42
?fromYear=2024&fromWeek=40&toWeek=45
?fromYear=2024&fromWeek=50&toYear=2025&toWeek=2
```

### By Date

```
?fromDate=2024-10-01&toDate=2024-10-31
```

When parameters are present, conversations auto-load after users/channels.

## Available Tables

| Table | Description |
|-------|-------------|
| `users` | User profiles (auto-loaded) |
| `channels` | Channel metadata (auto-loaded) |
| `threads` | Messages and replies (load via date range) |

## Sample Queries

The dropdown includes queries for:

- **Overview**: User/channel/message counts
- **Channels**: Top by members, top by messages
- **Users**: Most active, human users list
- **Time**: Messages by week/date
- **Threads**: Top threads, reply statistics
- **Search**: Keyword search, recent messages

Press `Ctrl+Enter` to run queries.

## Development

```bash
just install        # Install dependencies
just build          # Build minified bundle
just serve [path]   # Start dev server
just dev [path]     # Dev server with hot reload
just serve-static   # Static file server
just serve-with-server [path]  # Test with slack-archive-server
```

## Standalone Deployment

To deploy without the dev server:

1. Build the app:
   ```bash
   just build
   ```

2. Copy these files to your web server:
   - `dist/app.js` - Bundled application
   - `index.html` - Main page (add import map, see above)
   - `style.css` - Styles

3. Configure your server to:
   - Serve static files
   - Implement the `/archive/*` API endpoints
   - Set CORS headers: `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: require-corp`

## Requirements

- Modern browser with WebAssembly and ES modules support
- Bun (for development servers)
