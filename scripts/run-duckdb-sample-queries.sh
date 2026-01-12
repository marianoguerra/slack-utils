#!/usr/bin/env bash
# Run sample DuckDB queries to show useful stats and summaries
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
