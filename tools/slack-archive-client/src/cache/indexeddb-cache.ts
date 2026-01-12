import type { CacheEntry, CacheStorage, IndexedDBCacheOptions } from "./types.js";

/**
 * IndexedDB-based cache storage for binary data.
 * Provides persistent caching of ArrayBuffer data with good browser support.
 */
export class IndexedDBCache implements CacheStorage {
  private db: IDBDatabase | null = null;
  private readonly dbName: string;
  private readonly storeName: string;
  private initPromise: Promise<IDBDatabase> | null = null;

  constructor(options: IndexedDBCacheOptions = {}) {
    this.dbName = options.dbName ?? "slack-archive-cache";
    this.storeName = options.storeName ?? "files";
  }

  private async getDB(): Promise<IDBDatabase> {
    if (this.db) return this.db;

    // Ensure only one initialization happens
    if (this.initPromise) return this.initPromise;

    this.initPromise = new Promise((resolve, reject) => {
      const request = indexedDB.open(this.dbName, 1);

      request.onerror = () => {
        this.initPromise = null;
        reject(new Error(`Failed to open IndexedDB: ${request.error?.message}`));
      };

      request.onsuccess = () => {
        this.db = request.result;
        resolve(this.db);
      };

      request.onupgradeneeded = (event) => {
        const db = (event.target as IDBOpenDBRequest).result;
        if (!db.objectStoreNames.contains(this.storeName)) {
          db.createObjectStore(this.storeName);
        }
      };
    });

    return this.initPromise;
  }

  async get(key: string): Promise<CacheEntry | null> {
    const db = await this.getDB();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(this.storeName, "readonly");
      const store = tx.objectStore(this.storeName);
      const request = store.get(key);

      request.onerror = () => reject(new Error(`Failed to get "${key}": ${request.error?.message}`));
      request.onsuccess = () => resolve(request.result ?? null);
    });
  }

  async set(key: string, entry: CacheEntry): Promise<void> {
    const db = await this.getDB();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(this.storeName, "readwrite");
      const store = tx.objectStore(this.storeName);
      const request = store.put(entry, key);

      request.onerror = () => reject(new Error(`Failed to set "${key}": ${request.error?.message}`));
      request.onsuccess = () => resolve();
    });
  }

  async delete(key: string): Promise<boolean> {
    const db = await this.getDB();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(this.storeName, "readwrite");
      const store = tx.objectStore(this.storeName);
      const request = store.delete(key);

      request.onerror = () => reject(new Error(`Failed to delete "${key}": ${request.error?.message}`));
      request.onsuccess = () => resolve(true);
    });
  }

  async clear(): Promise<void> {
    const db = await this.getDB();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(this.storeName, "readwrite");
      const store = tx.objectStore(this.storeName);
      const request = store.clear();

      request.onerror = () => reject(new Error(`Failed to clear cache: ${request.error?.message}`));
      request.onsuccess = () => resolve();
    });
  }

  async keys(): Promise<string[]> {
    const db = await this.getDB();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(this.storeName, "readonly");
      const store = tx.objectStore(this.storeName);
      const request = store.getAllKeys();

      request.onerror = () => reject(new Error(`Failed to get keys: ${request.error?.message}`));
      request.onsuccess = () => resolve(request.result as string[]);
    });
  }

  async size(): Promise<number> {
    const allKeys = await this.keys();
    let total = 0;

    // Batch read all entries to calculate total size
    const db = await this.getDB();
    const tx = db.transaction(this.storeName, "readonly");
    const store = tx.objectStore(this.storeName);

    const promises = allKeys.map(
      (key) =>
        new Promise<number>((resolve) => {
          const request = store.get(key);
          request.onsuccess = () => {
            const entry = request.result as CacheEntry | undefined;
            resolve(entry?.size ?? 0);
          };
          request.onerror = () => resolve(0);
        })
    );

    const sizes = await Promise.all(promises);
    total = sizes.reduce((sum, size) => sum + size, 0);

    return total;
  }

  /**
   * Close the database connection.
   * Call this when done with the cache to release resources.
   */
  close(): void {
    if (this.db) {
      this.db.close();
      this.db = null;
      this.initPromise = null;
    }
  }
}
