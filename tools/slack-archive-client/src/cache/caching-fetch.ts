import type { CacheEntry, CacheStorage, CachingFetchOptions } from "./types.js";

/** Default TTL: 1 hour */
const DEFAULT_TTL = 60 * 60 * 1000;

/** Default max cache size: 500MB */
const DEFAULT_MAX_SIZE = 500 * 1024 * 1024;

/**
 * Create a fetch function that caches responses using the provided storage backend.
 *
 * @param baseFetch - The underlying fetch function to wrap
 * @param cache - Cache storage backend (e.g., IndexedDBCache)
 * @param options - Caching options (TTL, max size, event callbacks)
 * @returns A fetch function with caching behavior
 *
 * @example
 * ```typescript
 * const cache = new IndexedDBCache();
 * const cachedFetch = createCachingFetch(fetch, cache, {
 *   ttl: {
 *     "**\/users.parquet": 24 * 60 * 60 * 1000,
 *     "**\/conversations/**": 7 * 24 * 60 * 60 * 1000,
 *   },
 * });
 *
 * const client = new SlackArchiveClient({
 *   baseUrl: "http://localhost:8080",
 *   fetch: cachedFetch,
 * });
 * ```
 */
export function createCachingFetch(
  baseFetch: typeof fetch,
  cache: CacheStorage,
  options: CachingFetchOptions = {}
): typeof fetch {
  const {
    ttl = {},
    defaultTtl = DEFAULT_TTL,
    maxSize = DEFAULT_MAX_SIZE,
    onCacheEvent,
  } = options;

  /**
   * Get TTL for a URL based on pattern matching
   */
  function getTtlForUrl(url: string): number {
    for (const [pattern, ms] of Object.entries(ttl)) {
      if (matchPattern(pattern, url)) return ms;
    }
    return defaultTtl;
  }

  /**
   * Simple glob pattern matching
   * - ** matches any path (including /)
   * - * matches single path segment (excluding /)
   */
  function matchPattern(pattern: string, url: string): boolean {
    // Escape regex special chars except * and **
    const escaped = pattern
      .replace(/[.+?^${}()|[\]\\]/g, "\\$&")
      .replace(/\*\*/g, "<<<GLOBSTAR>>>")
      .replace(/\*/g, "[^/]*")
      .replace(/<<<GLOBSTAR>>>/g, ".*");
    return new RegExp(escaped).test(url);
  }

  /**
   * Evict oldest entries until we free enough space
   */
  async function evictOldest(bytesToFree: number): Promise<void> {
    const allKeys = await cache.keys();
    const entries: Array<{ key: string; entry: CacheEntry }> = [];

    // Fetch all entries with their timestamps
    for (const key of allKeys) {
      const entry = await cache.get(key);
      if (entry) entries.push({ key, entry });
    }

    // Sort by timestamp (oldest first)
    entries.sort((a, b) => a.entry.timestamp - b.entry.timestamp);

    let freed = 0;
    for (const { key, entry } of entries) {
      if (freed >= bytesToFree) break;
      await cache.delete(key);
      freed += entry.size;
      onCacheEvent?.({ type: "evict", url: key, size: entry.size });
    }
  }

  /**
   * The caching fetch function
   */
  return async function cachingFetch(
    input: RequestInfo | URL,
    init?: RequestInit
  ): Promise<Response> {
    const request = new Request(input, init);
    const url = request.url;
    const method = request.method.toUpperCase();

    // Only cache GET requests (HEAD might be used for probing)
    if (method !== "GET") {
      return baseFetch(input, init);
    }

    const cacheKey = url;
    const now = Date.now();

    // Check cache first
    try {
      const cached = await cache.get(cacheKey);
      const urlTtl = getTtlForUrl(url);

      if (cached && now - cached.timestamp < urlTtl) {
        const age = now - cached.timestamp;
        onCacheEvent?.({ type: "hit", url, size: cached.size, age });

        // Return cached response
        return new Response(cached.data, {
          status: 200,
          headers: {
            "Content-Type": "application/octet-stream",
            "Content-Length": String(cached.size),
            "X-Cache": "HIT",
            "X-Cache-Age": String(age),
          },
        });
      }
    } catch (error) {
      // Cache read failed, continue to network
      onCacheEvent?.({
        type: "error",
        url,
        error: error instanceof Error ? error.message : String(error),
      });
    }

    onCacheEvent?.({ type: "miss", url });

    // Fetch from network
    const response = await baseFetch(input, init);

    // Only cache successful responses
    if (response.ok) {
      try {
        // Clone response so we can read the body and still return it
        const data = await response.clone().arrayBuffer();
        const size = data.byteLength;

        // Check if we need to evict to make room
        const currentSize = await cache.size();
        if (currentSize + size > maxSize) {
          await evictOldest(currentSize + size - maxSize);
        }

        const entry: CacheEntry = {
          data,
          timestamp: now,
          etag: response.headers.get("ETag") ?? undefined,
          size,
        };

        await cache.set(cacheKey, entry);
        onCacheEvent?.({ type: "store", url, size });
      } catch (error) {
        // Cache write failed, but we can still return the response
        onCacheEvent?.({
          type: "error",
          url,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    }

    return response;
  };
}
