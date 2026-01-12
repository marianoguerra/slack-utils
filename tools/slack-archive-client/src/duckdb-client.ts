import type { SlackArchiveClient } from "./client.js";
import type * as duckdb from "@duckdb/duckdb-wasm";

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Options for creating a SlackArchiveDuckDB instance
 */
export interface DuckDBClientOptions {
  /** SlackArchiveClient instance for fetching parquet files */
  client: SlackArchiveClient;
  /** Optional DuckDB bundles (if not provided, will use jsdelivr) */
  bundles?: duckdb.DuckDBBundles;
}

/**
 * Result of a SQL query
 */
export interface QueryResult<T = Record<string, unknown>> {
  rows: T[];
  schema: Array<{ name: string; type: string }>;
  executionTimeMs: number;
}

/**
 * Progress information during data loading
 */
export interface LoadProgress {
  phase: "users" | "channels" | "threads";
  current: number;
  total: number;
  message: string;
}

/**
 * Callback for tracking loading progress
 */
export type ProgressCallback = (progress: LoadProgress) => void;

/**
 * Information about a loaded table
 */
export interface LoadedTableInfo {
  name: string;
  rowCount: number;
}

/**
 * Valid table names that can be loaded
 */
export type TableName = "users" | "channels" | "threads";

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Error thrown when DuckDB operations fail
 */
export class DuckDBClientError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "DuckDBClientError";
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Class
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Client for loading Slack archive parquet files into DuckDB WASM and querying them.
 *
 * @example
 * ```typescript
 * import { SlackArchiveClient } from "slack-archive-client";
 * import { SlackArchiveDuckDB } from "slack-archive-client/duckdb";
 *
 * const client = new SlackArchiveClient({ baseUrl: "http://localhost:8080" });
 * const db = new SlackArchiveDuckDB({ client });
 *
 * await db.init();
 * await db.loadAll("2024-01-01", "2024-03-31");
 *
 * const result = await db.query("SELECT * FROM threads LIMIT 10");
 * console.log(result.rows);
 *
 * await db.close();
 * ```
 */
export class SlackArchiveDuckDB {
  private readonly client: SlackArchiveClient;
  private readonly bundles?: duckdb.DuckDBBundles;
  private db: duckdb.AsyncDuckDB | null = null;
  private conn: duckdb.AsyncDuckDBConnection | null = null;
  private loadedTables: Set<TableName> = new Set();
  private fileCounter = 0;

  constructor(options: DuckDBClientOptions) {
    this.client = options.client;
    this.bundles = options.bundles;
  }

  /**
   * Initialize DuckDB WASM instance. Must be called before any other method.
   */
  async init(): Promise<void> {
    if (this.db) {
      return;
    }

    const duckdbModule = await import("@duckdb/duckdb-wasm");

    const bundles =
      this.bundles ?? duckdbModule.getJsDelivrBundles();
    const bundle = await duckdbModule.selectBundle(bundles);

    const workerUrl = URL.createObjectURL(
      new Blob([`importScripts("${bundle.mainWorker}");`], {
        type: "text/javascript",
      })
    );

    const worker = new Worker(workerUrl);
    const logger = new duckdbModule.ConsoleLogger();
    this.db = new duckdbModule.AsyncDuckDB(logger, worker);

    await this.db.instantiate(bundle.mainModule, bundle.pthreadWorker);
    URL.revokeObjectURL(workerUrl);

    this.conn = await this.db.connect();
  }

  /**
   * Load users.parquet into a "users" table.
   * @returns Number of rows loaded
   */
  async loadUsers(onProgress?: ProgressCallback): Promise<number> {
    this.ensureInitialized();

    onProgress?.({
      phase: "users",
      current: 0,
      total: 1,
      message: "Fetching users.parquet...",
    });

    const buffer = await this.client.getUsers();
    const rowCount = await this.loadParquetBuffer("users", buffer);

    this.loadedTables.add("users");

    onProgress?.({
      phase: "users",
      current: 1,
      total: 1,
      message: `Loaded ${rowCount} users`,
    });

    return rowCount;
  }

  /**
   * Load channels.parquet into a "channels" table.
   * @returns Number of rows loaded
   */
  async loadChannels(onProgress?: ProgressCallback): Promise<number> {
    this.ensureInitialized();

    onProgress?.({
      phase: "channels",
      current: 0,
      total: 1,
      message: "Fetching channels.parquet...",
    });

    const buffer = await this.client.getChannels();
    const rowCount = await this.loadParquetBuffer("channels", buffer);

    this.loadedTables.add("channels");

    onProgress?.({
      phase: "channels",
      current: 1,
      total: 1,
      message: `Loaded ${rowCount} channels`,
    });

    return rowCount;
  }

  /**
   * Load threads for a date range into a "threads" table.
   * Uses SlackArchiveClient.getThreadsInRange to discover available partitions,
   * then fetches each partition in parallel and unions them.
   *
   * @param from - Start date in YYYY-MM-DD format
   * @param to - End date in YYYY-MM-DD format
   * @returns Number of rows loaded
   */
  async loadThreads(
    from: string,
    to: string,
    onProgress?: ProgressCallback
  ): Promise<number> {
    this.ensureInitialized();

    onProgress?.({
      phase: "threads",
      current: 0,
      total: 1,
      message: "Discovering available partitions...",
    });

    const { available } = await this.client.getThreadsInRange(from, to);

    if (available.length === 0) {
      onProgress?.({
        phase: "threads",
        current: 1,
        total: 1,
        message: "No threads found in date range",
      });
      return 0;
    }

    const total = available.length;
    let completed = 0;

    onProgress?.({
      phase: "threads",
      current: 0,
      total,
      message: `Found ${total} partition(s), fetching...`,
    });

    // Fetch all partitions in parallel
    const fetchPromises = available.map(async ({ year, week }) => {
      const buffer = await this.client.getThreads(year, week);
      const fileName = `threads_${year}_${week}_${this.fileCounter++}.parquet`;
      await this.db!.registerFileBuffer(fileName, new Uint8Array(buffer));

      completed++;
      onProgress?.({
        phase: "threads",
        current: completed,
        total,
        message: `Loaded ${completed}/${total} partitions`,
      });

      return fileName;
    });

    const results = await Promise.allSettled(fetchPromises);

    // Collect successful file names
    const fileNames: string[] = [];
    for (const result of results) {
      if (result.status === "fulfilled") {
        fileNames.push(result.value);
      }
    }

    if (fileNames.length === 0) {
      throw new DuckDBClientError("Failed to load any thread partitions");
    }

    // Drop existing table if exists
    await this.conn!.query("DROP TABLE IF EXISTS threads");

    // Create table from all parquet files using UNION ALL
    let createQuery: string;
    if (fileNames.length === 1) {
      createQuery = `CREATE TABLE threads AS SELECT * FROM parquet_scan('${fileNames[0]}')`;
    } else {
      const unions = fileNames
        .map((f) => `SELECT * FROM parquet_scan('${f}')`)
        .join(" UNION ALL ");
      createQuery = `CREATE TABLE threads AS ${unions}`;
    }

    await this.conn!.query(createQuery);
    this.loadedTables.add("threads");

    const countResult = await this.conn!.query(
      "SELECT COUNT(*) as count FROM threads"
    );
    const rowCount = Number(countResult.toArray()[0].count);

    onProgress?.({
      phase: "threads",
      current: total,
      total,
      message: `Loaded ${rowCount} threads from ${fileNames.length} partition(s)`,
    });

    return rowCount;
  }

  /**
   * Load all data: users, channels, and threads for date range.
   * Convenience method that calls loadUsers, loadChannels, loadThreads.
   */
  async loadAll(
    from: string,
    to: string,
    onProgress?: ProgressCallback
  ): Promise<{ users: number; channels: number; threads: number }> {
    const users = await this.loadUsers(onProgress);
    const channels = await this.loadChannels(onProgress);
    const threads = await this.loadThreads(from, to, onProgress);

    return { users, channels, threads };
  }

  /**
   * Execute a SQL query against loaded tables.
   */
  async query<T = Record<string, unknown>>(sql: string): Promise<QueryResult<T>> {
    this.ensureInitialized();

    const startTime = performance.now();
    const result = await this.conn!.query(sql);
    const endTime = performance.now();

    const rows = result.toArray() as T[];
    const schema = result.schema.fields.map((field) => ({
      name: field.name,
      type: String(field.type),
    }));

    return {
      rows,
      schema,
      executionTimeMs: endTime - startTime,
    };
  }

  /**
   * Get set of currently loaded table names.
   */
  getLoadedTables(): Set<TableName> {
    return new Set(this.loadedTables);
  }

  /**
   * Check if a specific table is loaded.
   */
  isTableLoaded(name: TableName): boolean {
    return this.loadedTables.has(name);
  }

  /**
   * Get info about loaded tables including row counts.
   */
  async getTableInfo(): Promise<LoadedTableInfo[]> {
    this.ensureInitialized();

    const info: LoadedTableInfo[] = [];

    for (const name of this.loadedTables) {
      const result = await this.conn!.query(
        `SELECT COUNT(*) as count FROM ${name}`
      );
      const rowCount = Number(result.toArray()[0].count);
      info.push({ name, rowCount });
    }

    return info;
  }

  /**
   * Drop a loaded table from memory.
   */
  async dropTable(name: TableName): Promise<void> {
    this.ensureInitialized();

    if (!this.loadedTables.has(name)) {
      return;
    }

    await this.conn!.query(`DROP TABLE IF EXISTS ${name}`);
    this.loadedTables.delete(name);
  }

  /**
   * Close the DuckDB connection and clean up resources.
   */
  async close(): Promise<void> {
    if (this.conn) {
      await this.conn.close();
      this.conn = null;
    }

    if (this.db) {
      await this.db.terminate();
      this.db = null;
    }

    this.loadedTables.clear();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Private helpers
  // ─────────────────────────────────────────────────────────────────────────

  private ensureInitialized(): void {
    if (!this.db || !this.conn) {
      throw new DuckDBClientError(
        "DuckDB not initialized. Call init() first."
      );
    }
  }

  private async loadParquetBuffer(
    tableName: string,
    buffer: ArrayBuffer
  ): Promise<number> {
    const fileName = `${tableName}_${this.fileCounter++}.parquet`;
    await this.db!.registerFileBuffer(fileName, new Uint8Array(buffer));

    // Drop existing table if exists
    await this.conn!.query(`DROP TABLE IF EXISTS ${tableName}`);

    // Create table from parquet
    await this.conn!.query(
      `CREATE TABLE ${tableName} AS SELECT * FROM parquet_scan('${fileName}')`
    );

    // Get row count
    const countResult = await this.conn!.query(
      `SELECT COUNT(*) as count FROM ${tableName}`
    );
    return Number(countResult.toArray()[0].count);
  }
}
