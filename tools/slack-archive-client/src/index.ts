export { SlackArchiveClient, SlackArchiveError } from "./client.js";
export type {
  SlackArchiveClientOptions,
  ClientMode,
  YearWeek,
  IndexUser,
  IndexChannel,
  IndexEntry,
  ThreadsInRangeResponse,
  SearchResponse,
  ErrorResponse,
} from "./types.js";

export { SlackArchiveDuckDB, DuckDBClientError } from "./duckdb-client.js";
export type {
  DuckDBClientOptions,
  QueryResult,
  LoadProgress,
  ProgressCallback,
  LoadedTableInfo,
  TableName,
} from "./duckdb-client.js";

// Cache utilities
export { IndexedDBCache, createCachingFetch } from "./cache/index.js";
export type {
  CacheEntry,
  CacheStorage,
  CachingFetchOptions,
  CacheEvent,
  IndexedDBCacheOptions,
} from "./cache/index.js";
