export { SlackArchiveClient, SlackArchiveError } from "./client.js";
export type {
  SlackArchiveClientOptions,
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
