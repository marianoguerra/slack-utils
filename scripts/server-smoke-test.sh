#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Create temp directory for test outputs
TEMP_DIR=$(mktemp -d)
echo "=== Using temp directory: $TEMP_DIR ==="

# Server PID for cleanup
SERVER_PID=""

# Cleanup function
cleanup() {
    echo ""
    echo "=== Cleaning up ==="
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "Stopping server (PID: $SERVER_PID)..."
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    echo "Removing temp directory..."
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo ""
echo "=== Building slack-archive-server (no tui) ==="
cargo build --no-default-features --features server --bin slack-archive-server

echo ""
echo "=== Setting up test archive structure ==="

# Create archive directory structure
ARCHIVE_DIR="$TEMP_DIR/archive"
mkdir -p "$ARCHIVE_DIR"
mkdir -p "$ARCHIVE_DIR/conversations/year=2024/week=03"
mkdir -p "$ARCHIVE_DIR/conversations/year=2024/week=04"

# Create minimal parquet files using the library
# We'll create JSON files and convert them using the export functionality
# For smoke testing, we just need files that exist - content doesn't matter for basic endpoint tests

# Create users.parquet (minimal valid parquet file)
cat > "$TEMP_DIR/users.json" << 'EOF'
[
    {"id": "U001", "name": "testuser", "real_name": "Test User", "profile": {"display_name": "Test", "email": "test@example.com"}, "is_bot": false, "is_admin": true, "tz": "America/New_York"},
    {"id": "U002", "name": "botuser", "real_name": "Bot User", "profile": {"display_name": "Bot"}, "is_bot": true, "is_admin": false}
]
EOF

# Create channels.parquet
cat > "$TEMP_DIR/channels.json" << 'EOF'
[
    {"id": "C001", "name": "general", "topic": {"value": "General discussion"}, "purpose": {"value": "Company-wide"}, "is_private": false, "is_archived": false, "created": 1609459200, "num_members": 100},
    {"id": "C002", "name": "random", "topic": {"value": "Random stuff"}, "purpose": {"value": "Fun"}, "is_private": false, "is_archived": false, "created": 1609459200, "num_members": 50}
]
EOF

# Create conversations for week 3 and 4 of 2024
cat > "$TEMP_DIR/conversations.json" << 'EOF'
[
    {
        "channel_id": "C001",
        "channel_name": "general",
        "messages": [
            {"ts": "1705312800.000001", "user": "U001", "text": "Hello from week 3"},
            {"ts": "1705917600.000001", "user": "U002", "text": "Hello from week 4"}
        ]
    }
]
EOF

echo "Converting JSON to parquet files..."

# Use a small Rust program to create the parquet files
# We can use cargo run with specific exports, but for simplicity let's create dummy parquet files
# The server just needs to serve files - the content validation is done by clients

# Actually, let's create proper parquet files using the library
cd "$PROJECT_DIR"

# Create a helper script to generate parquet files
cat > "$TEMP_DIR/generate_parquet.rs" << 'GENEOF'
// This is just for documentation - we'll use the actual binary
GENEOF

# Export users to parquet using a minimal approach
# Since we can't easily call the library directly, let's create minimal binary parquet files
# For the smoke test, we just need valid files the server can read

# Create minimal parquet files (these are technically valid parquet files with minimal data)
# Using Python/DuckDB would be cleaner, but let's use what we have

# For now, let's write raw bytes that represent minimal valid parquet files
# Actually, simpler approach: write some test bytes and let the server serve them
# The client will just check HTTP status codes

echo "Creating test parquet files (minimal binary files for endpoint testing)..."

# Create minimal test files (the server just streams them, doesn't parse)
echo "PAR1 - test users data" > "$ARCHIVE_DIR/users.parquet"
echo "PAR1 - test channels data" > "$ARCHIVE_DIR/channels.parquet"
echo "PAR1 - test threads week 3" > "$ARCHIVE_DIR/conversations/year=2024/week=03/threads.parquet"
echo "PAR1 - test threads week 4" > "$ARCHIVE_DIR/conversations/year=2024/week=04/threads.parquet"

echo "Archive structure created:"
find "$ARCHIVE_DIR" -type f

echo ""
echo "=== Creating server config ==="

CONFIG_FILE="$TEMP_DIR/config.toml"
cat > "$CONFIG_FILE" << EOF
[server]
host = "127.0.0.1"
port = 18080

[slack-archive]
base_path = "$ARCHIVE_DIR"
EOF

echo "Config file:"
cat "$CONFIG_FILE"

echo ""
echo "=== Starting slack-archive-server ==="

# Start the server in the background
"$PROJECT_DIR/target/debug/slack-archive-server" serve "$CONFIG_FILE" &
SERVER_PID=$!

echo "Server started with PID: $SERVER_PID"

# Wait for server to be ready
echo "Waiting for server to be ready..."
MAX_RETRIES=30
RETRY_COUNT=0
while ! curl -s http://127.0.0.1:18080/archive/users > /dev/null 2>&1; do
    RETRY_COUNT=$((RETRY_COUNT + 1))
    if [ $RETRY_COUNT -ge $MAX_RETRIES ]; then
        echo "ERROR: Server failed to start after $MAX_RETRIES attempts"
        exit 1
    fi
    sleep 0.1
done
echo "Server is ready!"

echo ""
echo "=== Running client tests with bun ==="

# Create the test client
cat > "$TEMP_DIR/test-client.ts" << 'CLIENTEOF'
const BASE_URL = "http://127.0.0.1:18080";

interface YearWeek {
  year: number;
  week: number;
}

interface ThreadsInRangeResponse {
  available: YearWeek[];
}

interface TestResult {
  name: string;
  passed: boolean;
  error?: string;
}

const results: TestResult[] = [];

async function test(name: string, fn: () => Promise<void>): Promise<void> {
  try {
    await fn();
    results.push({ name, passed: true });
    console.log(`✓ ${name}`);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    results.push({ name, passed: false, error: errorMessage });
    console.log(`✗ ${name}: ${errorMessage}`);
  }
}

function assert(condition: boolean, message: string): void {
  if (!condition) {
    throw new Error(message);
  }
}

async function runTests(): Promise<void> {
  console.log("=== Testing Archive Server Endpoints ===\n");

  // Test GET /archive/users
  await test("GET /archive/users returns 200", async () => {
    const response = await fetch(`${BASE_URL}/archive/users`);
    assert(response.status === 200, `Expected 200, got ${response.status}`);
    assert(
      response.headers.get("content-type") === "application/octet-stream",
      `Expected content-type application/octet-stream`
    );
  });

  // Test GET /archive/channels
  await test("GET /archive/channels returns 200", async () => {
    const response = await fetch(`${BASE_URL}/archive/channels`);
    assert(response.status === 200, `Expected 200, got ${response.status}`);
  });

  // Test GET /archive/threads-in-range with valid dates
  await test("GET /archive/threads-in-range returns available weeks", async () => {
    const response = await fetch(
      `${BASE_URL}/archive/threads-in-range?from=2024-01-15&to=2024-01-28`
    );
    assert(response.status === 200, `Expected 200, got ${response.status}`);

    const data: ThreadsInRangeResponse = await response.json();
    assert(Array.isArray(data.available), "Expected available to be an array");
    assert(data.available.length === 2, `Expected 2 weeks, got ${data.available.length}`);

    // Check week 3 is present
    const week3 = data.available.find(w => w.year === 2024 && w.week === 3);
    assert(week3 !== undefined, "Expected week 3 of 2024 to be available");

    // Check week 4 is present
    const week4 = data.available.find(w => w.year === 2024 && w.week === 4);
    assert(week4 !== undefined, "Expected week 4 of 2024 to be available");
  });

  // Test GET /archive/threads-in-range with no data in range
  await test("GET /archive/threads-in-range returns empty for no data", async () => {
    const response = await fetch(
      `${BASE_URL}/archive/threads-in-range?from=2023-01-01&to=2023-01-07`
    );
    assert(response.status === 200, `Expected 200, got ${response.status}`);

    const data: ThreadsInRangeResponse = await response.json();
    assert(data.available.length === 0, `Expected 0 weeks, got ${data.available.length}`);
  });

  // Test GET /archive/threads-in-range with invalid from date
  await test("GET /archive/threads-in-range returns 400 for invalid from date", async () => {
    const response = await fetch(
      `${BASE_URL}/archive/threads-in-range?from=invalid&to=2024-01-21`
    );
    assert(response.status === 400, `Expected 400, got ${response.status}`);
  });

  // Test GET /archive/threads-in-range with invalid to date
  await test("GET /archive/threads-in-range returns 400 for invalid to date", async () => {
    const response = await fetch(
      `${BASE_URL}/archive/threads-in-range?from=2024-01-15&to=invalid`
    );
    assert(response.status === 400, `Expected 400, got ${response.status}`);
  });

  // Test GET /archive/threads-in-range with from > to
  await test("GET /archive/threads-in-range returns 400 when from > to", async () => {
    const response = await fetch(
      `${BASE_URL}/archive/threads-in-range?from=2024-01-28&to=2024-01-15`
    );
    assert(response.status === 400, `Expected 400, got ${response.status}`);
  });

  // Test GET /archive/threads with valid week
  await test("GET /archive/threads returns 200 for existing week", async () => {
    const response = await fetch(`${BASE_URL}/archive/threads?year=2024&week=3`);
    assert(response.status === 200, `Expected 200, got ${response.status}`);
    assert(
      response.headers.get("content-type") === "application/octet-stream",
      `Expected content-type application/octet-stream`
    );
  });

  // Test GET /archive/threads with another valid week
  await test("GET /archive/threads returns 200 for week 4", async () => {
    const response = await fetch(`${BASE_URL}/archive/threads?year=2024&week=4`);
    assert(response.status === 200, `Expected 200, got ${response.status}`);
  });

  // Test GET /archive/threads with non-existent week
  await test("GET /archive/threads returns 404 for non-existent week", async () => {
    const response = await fetch(`${BASE_URL}/archive/threads?year=2024&week=1`);
    assert(response.status === 404, `Expected 404, got ${response.status}`);
  });

  // Test GET /archive/threads with invalid week (0)
  await test("GET /archive/threads returns 400 for week 0", async () => {
    const response = await fetch(`${BASE_URL}/archive/threads?year=2024&week=0`);
    assert(response.status === 400, `Expected 400, got ${response.status}`);
  });

  // Test GET /archive/threads with invalid week (54)
  await test("GET /archive/threads returns 400 for week 54", async () => {
    const response = await fetch(`${BASE_URL}/archive/threads?year=2024&week=54`);
    assert(response.status === 400, `Expected 400, got ${response.status}`);
  });

  // Summary
  console.log("\n=== Test Summary ===");
  const passed = results.filter(r => r.passed).length;
  const failed = results.filter(r => !r.passed).length;
  console.log(`Passed: ${passed}`);
  console.log(`Failed: ${failed}`);
  console.log(`Total:  ${results.length}`);

  if (failed > 0) {
    console.log("\nFailed tests:");
    results.filter(r => !r.passed).forEach(r => {
      console.log(`  - ${r.name}: ${r.error}`);
    });
    process.exit(1);
  }
}

runTests().catch(error => {
  console.error("Test runner error:", error);
  process.exit(1);
});
CLIENTEOF

# Run the test client with bun
cd "$TEMP_DIR"
bun run test-client.ts

echo ""
echo "=== Server smoke test completed successfully ==="
