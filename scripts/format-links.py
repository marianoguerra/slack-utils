#!/usr/bin/env python3
"""
Formatter script for slack-utils markdown export.

This script implements the CGI-like protocol for custom formatting of permalinks,
attachments, and files. It reads a JSON request from stdin and writes a JSON response
to stdout.

Input format (stdin):
{
    "headers": {
        "action": "format-permalink" | "format-attachment" | "format-file",
        "channel_id": "C123",
        "channel_name": "general",
        "message_ts": "1234567890.123456"  // only for format-permalink
    },
    "body": { /* message, attachment, or file JSON */ }
}

Output format (stdout, exit code 0):
{
    "label": "Display text",
    "url": "https://..."
}

Exit codes:
- 0: Success
- 1: Error (error message in stderr)
"""

import argparse
import json
import sys


def format_permalink(headers: dict, body: dict, verbose: bool = False) -> dict:
    """Format a permalink for a message.

    This identity implementation returns a simple Slack archive URL.
    """
    channel_id = headers.get("channel_id", "")
    message_ts = headers.get("message_ts", "")

    # Convert message_ts to URL format (remove the dot)
    ts_for_url = message_ts.replace(".", "")

    url = f"https://app.slack.com/archives/{channel_id}/p{ts_for_url}"

    if verbose:
        print(f"[format-permalink] channel={channel_id}, ts={message_ts}", file=sys.stderr)

    return {
        "label": "Conversation permalink",
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


def format_file(headers: dict, body: dict, verbose: bool = False) -> dict:
    """Format a file (uploaded content).

    This identity implementation returns the original Slack URL and title from the body.
    """
    # Try to get URL from various possible fields
    url = (
        body.get("url_private") or
        body.get("permalink") or
        body.get("url") or
        ""
    )

    # Try to get title from various possible fields
    title = (
        body.get("title") or
        body.get("name") or
        "Untitled file"
    )

    if verbose:
        print(f"[format-file] title={title}, url={url[:50]}...", file=sys.stderr)

    return {
        "label": title,
        "url": url
    }


def main():
    parser = argparse.ArgumentParser(
        description="Formatter script for slack-utils markdown export"
    )
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Enable verbose output to stderr"
    )
    args = parser.parse_args()

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


if __name__ == "__main__":
    main()
