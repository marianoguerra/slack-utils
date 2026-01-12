/**
 * Year and week identifier for thread partitions
 */
export interface YearWeek {
  year: number;
  week: number;
}

/**
 * User information in search results
 */
export interface IndexUser {
  id: string;
  name: string;
}

/**
 * Channel information in search results
 */
export interface IndexChannel {
  id: string;
  name: string;
}

/**
 * A single entry from the conversation index (search result)
 */
export interface IndexEntry {
  /** Unique identifier (ts with dots replaced for Meilisearch compatibility) */
  id: string;
  /** Original Slack timestamp (e.g., "1767636991.559059") */
  ts: string;
  /** ISO 8601 datetime of the message */
  date: string;
  /** Markdown rendering of the message blocks including thread replies */
  text: string;
  /** List of users involved in this thread */
  users: IndexUser[];
  /** Channel information */
  channel: IndexChannel;
}

/**
 * Response from the threads-in-range endpoint
 */
export interface ThreadsInRangeResponse {
  available: YearWeek[];
}

/**
 * Response from the search endpoint
 */
export interface SearchResponse {
  hits: IndexEntry[];
  processing_time_ms: number;
  estimated_total_hits?: number;
}

/**
 * Error response from the server
 */
export interface ErrorResponse {
  error: string;
}

/**
 * Client mode: "api" for slack-archive-server, "static" for static file hosting
 */
export type ClientMode = "api" | "static";

/**
 * Client configuration options
 */
export interface SlackArchiveClientOptions {
  /** Base URL of the slack-archive-server or static file host (e.g., "http://localhost:8080") */
  baseUrl: string;
  /**
   * Client mode:
   * - "api" (default): Use slack-archive-server API endpoints
   * - "static": Fetch files directly from static paths (for static hosting)
   */
  mode?: ClientMode;
  /** Optional custom fetch implementation */
  fetch?: typeof fetch;
}
