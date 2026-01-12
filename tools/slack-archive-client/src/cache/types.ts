/**
 * A cached entry containing binary data and metadata
 */
export interface CacheEntry {
  /** The cached binary data */
  data: ArrayBuffer;
  /** Timestamp when the entry was stored (ms since epoch) */
  timestamp: number;
  /** ETag from the original response, if available */
  etag?: string;
  /** Size of the data in bytes */
  size: number;
}

/**
 * Abstract interface for cache storage backends (IndexedDB, OPFS, etc.)
 */
export interface CacheStorage {
  /** Get an entry by key, or null if not found */
  get(key: string): Promise<CacheEntry | null>;
  /** Store an entry with the given key */
  set(key: string, entry: CacheEntry): Promise<void>;
  /** Delete an entry by key, returns true if deleted */
  delete(key: string): Promise<boolean>;
  /** Clear all entries */
  clear(): Promise<void>;
  /** Get all keys in the cache */
  keys(): Promise<string[]>;
  /** Get total size of all cached data in bytes */
  size(): Promise<number>;
}

/**
 * Options for the caching fetch wrapper
 */
export interface CachingFetchOptions {
  /**
   * TTL per URL pattern (glob-like), in milliseconds.
   * Patterns support ** (any path) and * (single segment).
   * Example: { "**\/users.parquet": 86400000 }
   */
  ttl?: Record<string, number>;
  /** Default TTL if no pattern matches (default: 1 hour) */
  defaultTtl?: number;
  /** Maximum total cache size in bytes (default: 500MB) */
  maxSize?: number;
  /** Called when cache events occur (hit, miss, store, evict) */
  onCacheEvent?: (event: CacheEvent) => void;
}

/**
 * Event emitted by the caching layer
 */
export interface CacheEvent {
  /** Type of cache event */
  type: "hit" | "miss" | "store" | "evict" | "error";
  /** URL that triggered the event */
  url: string;
  /** Size in bytes (for store/evict events) */
  size?: number;
  /** Age in milliseconds (for hit events) */
  age?: number;
  /** Error message (for error events) */
  error?: string;
}

/**
 * Options for IndexedDB cache backend
 */
export interface IndexedDBCacheOptions {
  /** Database name (default: "slack-archive-cache") */
  dbName?: string;
  /** Object store name (default: "files") */
  storeName?: string;
}
