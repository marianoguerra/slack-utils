#!/usr/bin/env python3
"""
Formatter script for slack-utils markdown export - Future of Coding edition.

This script implements the CGI-like protocol for custom formatting of permalinks,
attachments, files, and document prefix/suffix. It reads a JSON request from stdin
and writes a JSON response to stdout.

Input format (stdin):
{
    "headers": {
        "action": "format-permalink" | "format-attachment" | "format-file" | "prefix" | "suffix",
        "channel_id": "C123",
        "channel_name": "general",
        "message_ts": "1234567890.123456"  // only for format-permalink
    },
    "body": { /* message, attachment, file JSON, or {"threads": [...]} for prefix/suffix */ }
}

Output format for format-* actions (stdout, exit code 0):
{
    "label": "Display text",
    "url": "https://..."
}

Output format for prefix/suffix actions (stdout, exit code 0):
{
    "content": "string to insert before/after markdown"
}

Exit codes:
- 0: Success
- 1: Error (error message in stderr)
"""

import argparse
import json
import sys
from datetime import datetime, timezone


def ts_to_datetime(ts: str) -> datetime:
    """Convert Slack timestamp to datetime object."""
    # Slack timestamps are Unix timestamps with microseconds after the dot
    unix_ts = float(ts)
    return datetime.fromtimestamp(unix_ts, tz=timezone.utc)


def get_iso_week(dt: datetime) -> int:
    """Get ISO week number from datetime."""
    return dt.isocalendar()[1]


def format_permalink(headers: dict, body: dict, verbose: bool = False) -> dict:
    """Format a permalink for a message.

    Returns a Future of Coding archive URL in the format:
    https://history.futureofcoding.org/history/weekly/{year}/{month}/W{week}/{channel}.html#{iso8601}
    """
    channel_name = headers.get("channel_name", "general")
    message_ts = headers.get("message_ts", "")

    # Convert timestamp to datetime
    dt = ts_to_datetime(message_ts)

    # Extract components
    year = dt.year
    month = f"{dt.month:02d}"
    week = get_iso_week(dt)

    # Format ISO 8601 timestamp for anchor (with milliseconds)
    # Slack ts format: "1736762741.815000" -> need "2026-01-13T09:32:21.815Z"
    iso_timestamp = dt.strftime("%Y-%m-%dT%H:%M:%S")
    # Add milliseconds from the original timestamp
    if "." in message_ts:
        micros = message_ts.split(".")[1]
        millis = micros[:3]  # First 3 digits are milliseconds
        iso_timestamp += f".{millis}Z"
    else:
        iso_timestamp += ".000Z"

    url = f"https://history.futureofcoding.org/history/weekly/{year}/{month}/W{week}/{channel_name}.html#{iso_timestamp}"

    if verbose:
        print(f"[format-permalink] channel={channel_name}, ts={message_ts}", file=sys.stderr)
        print(f"[format-permalink] year={year}, month={month}, week=W{week}", file=sys.stderr)
        print(f"[format-permalink] iso_timestamp={iso_timestamp}", file=sys.stderr)

    return {
        "label": "\U0001f9f5conversation",
        "url": url
    }


def format_file_url(file_id: str, filename: str, mimetype: str = "") -> str:
    """Build a Future of Coding file URL from file ID and filename.

    URL format: https://history.futureofcoding.org/history/msg_files/{prefix}/{file_id}.{ext}
    """
    # Get prefix (first 3 characters of the file ID)
    prefix = file_id[:3] if file_id else ""

    # Get extension from filename or mimetype
    ext = ""
    if filename and "." in filename:
        ext = filename[filename.rfind("."):]
    elif mimetype:
        ext_map = {
            "video/mp4": ".mp4",
            "video/quicktime": ".mov",
            "image/png": ".png",
            "image/jpeg": ".jpg",
            "image/gif": ".gif",
            "audio/mp3": ".mp3",
            "audio/mpeg": ".mp3",
            "application/pdf": ".pdf",
        }
        ext = ext_map.get(mimetype, "")

    full_filename = file_id + ext
    return f"https://history.futureofcoding.org/history/msg_files/{prefix}/{full_filename}"


def format_file(headers: dict, body: dict, verbose: bool = False) -> dict:
    """Format a Slack file.

    Returns a Future of Coding file URL in the format:
    https://history.futureofcoding.org/history/msg_files/{prefix}/{file_id}.{ext}
    """
    file_id = body.get("id") or ""
    filename = body.get("name") or body.get("title") or ""
    mimetype = body.get("mimetype", "")

    url = format_file_url(file_id, filename, mimetype)

    # Try to get title from various possible fields
    title = (
        body.get("title") or
        body.get("name") or
        "Untitled"
    )

    if verbose:
        print(f"[format-file] title={title}, file_id={file_id}", file=sys.stderr)
        print(f"[format-file] url={url}", file=sys.stderr)

    return {
        "label": title,
        "url": url
    }


def format_attachment(headers: dict, body: dict, verbose: bool = False) -> dict:
    """Format an attachment (link unfurl metadata).

    This identity implementation returns the original URL and title from the body.
    """
    # Try to get URL from various possible fields
    url = (
        body.get("original_url") or
        body.get("from_url") or
        body.get("title_link") or
        body.get("url") or
        ""
    )

    # Try to get title from various possible fields
    title = (
        body.get("title") or
        body.get("name") or
        body.get("fallback") or
        "Untitled"
    )

    if verbose:
        print(f"[format-attachment] title={title}, url={url[:50]}...", file=sys.stderr)

    return {
        "label": title,
        "url": url
    }


def format_prefix(headers: dict, body: dict, verbose: bool = False) -> dict:
    """Generate content to insert before the markdown.

    The body contains {"threads": [...]} with all conversation threads.
    Returns empty content by default - customize as needed.
    """
    threads = body.get("threads", [])

    if verbose:
        print(f"[prefix] Received {len(threads)} threads", file=sys.stderr)

    # Return empty content by default
    return {"content": ""}

SUFFIX = """
----------

👨🏽‍💻 By 🐘 [@marianoguerra@hachyderm.io](https://hachyderm.io/@marianoguerra) 🦋 [@marianoguerra.org](https://bsky.app/profile/marianoguerra.org)

💬 Not a member yet? Check the [Feeling of Computing Community](https://feelingof.com/)

✉️ Not subscribed yet? [Subscribe to the Newsletter](https://newsletter.futureofcoding.org/join/) / [Archive](https://newsletter.futureofcoding.org/archive.html) / [RSS](https://history.futureofcoding.org/newsletter/rss.xml)

🎙️ Prefer podcasts? check the [Feeling of Computing Podcast](https://feelingof.com/episodes/)

"""

def format_suffix(headers: dict, body: dict, verbose: bool = False) -> dict:
    """Generate content to insert after the markdown.

    The body contains {"threads": [...]} with all conversation threads.
    Returns empty content by default - customize as needed.
    """
    threads = body.get("threads", [])

    if verbose:
        print(f"[suffix] Received {len(threads)} threads", file=sys.stderr)

    # Return empty content by default
    return {"content": SUFFIX}


def main():
    parser = argparse.ArgumentParser(
        description="Formatter script for slack-utils markdown export (Future of Coding)"
    )
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Enable verbose output to stderr"
    )
    parser.add_argument(
        "--test",
        action="store_true",
        help="Run integration tests"
    )
    args = parser.parse_args()

    if args.test:
        run_tests()
        return

    try:
        # Read JSON request from stdin
        input_data = sys.stdin.read()
        if not input_data.strip():
            print("Error: Empty input", file=sys.stderr)
            sys.exit(1)

        request = json.loads(input_data)
        headers = request.get("headers", {})
        body = request.get("body", {})
        action = headers.get("action", "")

        if args.verbose:
            print(f"[formatter] Action: {action}", file=sys.stderr)

        # Dispatch based on action
        if action == "format-permalink":
            response = format_permalink(headers, body, args.verbose)
        elif action == "format-attachment":
            response = format_attachment(headers, body, args.verbose)
        elif action == "format-file":
            response = format_file(headers, body, args.verbose)
        elif action == "prefix":
            response = format_prefix(headers, body, args.verbose)
        elif action == "suffix":
            response = format_suffix(headers, body, args.verbose)
        else:
            print(f"Error: Unknown action '{action}'", file=sys.stderr)
            sys.exit(1)

        # Write JSON response to stdout
        print(json.dumps(response))
        sys.exit(0)

    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON input: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def run_tests():
    """Run integration tests."""
    import traceback

    tests_passed = 0
    tests_failed = 0

    def test(name, func):
        nonlocal tests_passed, tests_failed
        try:
            func()
            print(f"  PASS: {name}")
            tests_passed += 1
        except AssertionError as e:
            print(f"  FAIL: {name}")
            print(f"        {e}")
            tests_failed += 1
        except Exception as e:
            print(f"  ERROR: {name}")
            print(f"        {e}")
            traceback.print_exc()
            tests_failed += 1

    print("Running integration tests...\n")

    # Test 1: Permalink format - example from user
    def test_permalink_example():
        # 1736762741.815 = 2025-01-13T10:05:41.815Z UTC (week 3 of 2025)
        headers = {
            "action": "format-permalink",
            "channel_id": "C123ABC",
            "channel_name": "share-your-work",
            "message_ts": "1736762741.815000"
        }
        body = {}
        result = format_permalink(headers, body)

        assert result["label"] == "\U0001f9f5conversation", f"Expected '\U0001f9f5conversation', got '{result['label']}'"

        expected_url = "https://history.futureofcoding.org/history/weekly/2025/01/W3/share-your-work.html#2025-01-13T10:05:41.815Z"
        assert result["url"] == expected_url, f"Expected '{expected_url}', got '{result['url']}'"

    test("Permalink format - share-your-work example", test_permalink_example)

    # Test 1b: Permalink format - 2026 example matching user's URL format exactly
    def test_permalink_2026_example():
        # Calculate timestamp for 2026-01-13T09:32:21.815Z
        # This is the exact format from the user's example
        headers = {
            "action": "format-permalink",
            "channel_id": "C123ABC",
            "channel_name": "share-your-work",
            "message_ts": "1768298741.815000"  # 2026-01-13T09:05:41.815Z UTC
        }
        body = {}
        result = format_permalink(headers, body)

        assert result["label"] == "\U0001f9f5conversation"
        assert "history.futureofcoding.org" in result["url"]
        assert "/2026/01/W3/" in result["url"]
        assert "share-your-work.html" in result["url"]

    test("Permalink format - 2026 example", test_permalink_2026_example)

    # Test 2: Permalink - different channel and timestamp
    def test_permalink_different_channel():
        headers = {
            "action": "format-permalink",
            "channel_id": "C456DEF",
            "channel_name": "thinking-together",
            "message_ts": "1640000000.123456"  # 2021-12-20T11:33:20.123Z (week 51)
        }
        body = {}
        result = format_permalink(headers, body)

        assert result["label"] == "\U0001f9f5conversation"
        assert "thinking-together.html" in result["url"]
        assert "/2021/12/W51/" in result["url"]
        assert "#2021-12-20T11:33:20.123Z" in result["url"]

    test("Permalink format - thinking-together channel", test_permalink_different_channel)

    # Test 3: Permalink - week boundary (first week of year)
    def test_permalink_week_boundary():
        headers = {
            "action": "format-permalink",
            "channel_id": "C789",
            "channel_name": "general",
            "message_ts": "1704067200.000000"  # 2024-01-01T00:00:00.000Z (week 1)
        }
        body = {}
        result = format_permalink(headers, body)

        assert "/W1/" in result["url"], f"Expected week 1, got: {result['url']}"
        assert "/2024/01/" in result["url"]

    test("Permalink format - week boundary (new year)", test_permalink_week_boundary)

    # Test 4: Attachment format - link unfurl with original_url
    def test_attachment_link_unfurl():
        headers = {
            "action": "format-attachment",
            "channel_id": "C123",
            "channel_name": "share-your-work"
        }
        body = {
            "title": "GitHub - example/repo",
            "original_url": "https://github.com/example/repo",
            "fallback": "GitHub repo"
        }
        result = format_attachment(headers, body)

        assert result["label"] == "GitHub - example/repo", f"Expected 'GitHub - example/repo', got '{result['label']}'"
        assert result["url"] == "https://github.com/example/repo", f"Expected original URL, got '{result['url']}'"

    test("Attachment format - link unfurl (identity)", test_attachment_link_unfurl)

    # Test 5: Attachment format - from_url field
    def test_attachment_from_url():
        headers = {
            "action": "format-attachment",
            "channel_id": "C456",
            "channel_name": "random"
        }
        body = {
            "title": "Article Title",
            "from_url": "https://example.com/article"
        }
        result = format_attachment(headers, body)

        assert result["label"] == "Article Title"
        assert result["url"] == "https://example.com/article"

    test("Attachment format - from_url field (identity)", test_attachment_from_url)

    # Test 6: Attachment format - uses name when no title
    def test_attachment_no_title():
        headers = {
            "action": "format-attachment",
            "channel_id": "C789",
            "channel_name": "files"
        }
        body = {
            "name": "Document Name",
            "title_link": "https://example.com/doc"
        }
        result = format_attachment(headers, body)

        assert result["label"] == "Document Name", f"Expected 'Document Name', got '{result['label']}'"
        assert result["url"] == "https://example.com/doc"

    test("Attachment format - uses name when no title (identity)", test_attachment_no_title)

    # Test 7: Attachment format - fallback label
    def test_attachment_fallback():
        headers = {
            "action": "format-attachment",
            "channel_id": "C111",
            "channel_name": "test"
        }
        body = {
            "fallback": "Fallback text",
            "url": "https://example.com/fallback"
        }
        result = format_attachment(headers, body)

        assert result["label"] == "Fallback text"
        assert result["url"] == "https://example.com/fallback"

    test("Attachment format - uses fallback label (identity)", test_attachment_fallback)

    # Test 8: File format - from selected-conversations.md example
    def test_file_screenshot():
        headers = {
            "action": "format-file",
            "channel_id": "C123",
            "channel_name": "devlog-together"
        }
        body = {
            "id": "F0AASCM79SN",
            "name": "screenshot_20260124_214114_samsung_internet.png",
            "title": "Screenshot_20260124_214114_Samsung Internet.png",
            "mimetype": "image/png"
        }
        result = format_file(headers, body)

        assert result["label"] == "Screenshot_20260124_214114_Samsung Internet.png"
        expected_url = "https://history.futureofcoding.org/history/msg_files/F0A/F0AASCM79SN.png"
        assert result["url"] == expected_url, f"Expected '{expected_url}', got '{result['url']}'"

    test("File format - screenshot from devlog-together", test_file_screenshot)

    # Test 9: File format - jpg image from selected-conversations.md
    def test_file_jpg():
        headers = {
            "action": "format-file",
            "channel_id": "C123",
            "channel_name": "devlog-together"
        }
        body = {
            "id": "F0AAAV5QZ71",
            "name": "1000001187.jpg",
            "title": "1000001187.jpg",
            "mimetype": "image/jpeg"
        }
        result = format_file(headers, body)

        assert result["label"] == "1000001187.jpg"
        expected_url = "https://history.futureofcoding.org/history/msg_files/F0A/F0AAAV5QZ71.jpg"
        assert result["url"] == expected_url, f"Expected '{expected_url}', got '{result['url']}'"

    test("File format - jpg image", test_file_jpg)

    # Test 10: File format - video file
    def test_file_video():
        headers = {
            "action": "format-file",
            "channel_id": "C456",
            "channel_name": "share-your-work"
        }
        body = {
            "id": "F0B123XYZ99",
            "name": "demo_video.mp4",
            "title": "Demo Video",
            "mimetype": "video/mp4"
        }
        result = format_file(headers, body)

        assert result["label"] == "Demo Video"
        expected_url = "https://history.futureofcoding.org/history/msg_files/F0B/F0B123XYZ99.mp4"
        assert result["url"] == expected_url, f"Expected '{expected_url}', got '{result['url']}'"

    test("File format - video mp4", test_file_video)

    # Test 11: Week calculation for different dates
    def test_week_calculations():
        # Test various dates and their expected weeks
        test_cases = [
            ("1736762741.815000", 3),   # 2025-01-13 -> Week 3
            ("1609459200.000000", 53),  # 2021-01-01 -> Week 53 (belongs to 2020)
            ("1672531200.000000", 52),  # 2023-01-01 -> Week 52 (belongs to 2022)
            ("1704067200.000000", 1),   # 2024-01-01 -> Week 1
        ]

        for ts, expected_week in test_cases:
            dt = ts_to_datetime(ts)
            week = get_iso_week(dt)
            assert week == expected_week, f"ts={ts}: Expected week {expected_week}, got {week}"

    test("Week calculations for various dates", test_week_calculations)

    print(f"\nResults: {tests_passed} passed, {tests_failed} failed")
    sys.exit(0 if tests_failed == 0 else 1)


if __name__ == "__main__":
    main()
