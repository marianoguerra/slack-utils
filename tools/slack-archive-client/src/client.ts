import type {
  SlackArchiveClientOptions,
  ClientMode,
  ThreadsInRangeResponse,
  SearchResponse,
  ErrorResponse,
  YearWeek,
} from "./types.js";

/**
 * Error thrown when the server returns an error response
 */
export class SlackArchiveError extends Error {
  public readonly statusCode: number;
  public readonly serverError?: string;

  constructor(message: string, statusCode: number, serverError?: string) {
    super(message);
    this.name = "SlackArchiveError";
    this.statusCode = statusCode;
    this.serverError = serverError;
  }
}

/**
 * Client for interacting with Slack archive parquet files.
 *
 * Supports two modes:
 * - "api" (default): Uses slack-archive-server HTTP API endpoints
 * - "static": Fetches files directly from static paths (for static file hosting)
 */
export class SlackArchiveClient {
  private readonly baseUrl: string;
  private readonly mode: ClientMode;
  private readonly fetchFn: typeof fetch;

  constructor(options: SlackArchiveClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/$/, "");
    this.mode = options.mode ?? "api";
    this.fetchFn = options.fetch ?? ((...args) => fetch(...args));
  }

  /**
   * Fetch users.parquet file as an ArrayBuffer
   */
  async getUsers(): Promise<ArrayBuffer> {
    if (this.mode === "static") {
      return this.fetchStaticParquet("/users.parquet");
    }
    return this.fetchParquet("/archive/users");
  }

  /**
   * Fetch channels.parquet file as an ArrayBuffer
   */
  async getChannels(): Promise<ArrayBuffer> {
    if (this.mode === "static") {
      return this.fetchStaticParquet("/channels.parquet");
    }
    return this.fetchParquet("/archive/channels");
  }

  /**
   * Get available year/week partitions within a date range.
   *
   * In "api" mode, queries the server endpoint.
   * In "static" mode, generates possible weeks and probes for existing files.
   *
   * @param from Start date in ISO format (YYYY-MM-DD)
   * @param to End date in ISO format (YYYY-MM-DD)
   */
  async getThreadsInRange(
    from: string,
    to: string
  ): Promise<ThreadsInRangeResponse> {
    if (this.mode === "static") {
      return this.probeThreadsInRange(from, to);
    }

    const url = `${this.baseUrl}/archive/threads-in-range?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`;
    const response = await this.fetchFn(url);
    return this.handleJsonResponse<ThreadsInRangeResponse>(response);
  }

  /**
   * Fetch threads.parquet file for a specific year and week
   * @param year The year (e.g., 2024)
   * @param week The ISO week number (1-53)
   */
  async getThreads(year: number, week: number): Promise<ArrayBuffer> {
    if (this.mode === "static") {
      const weekStr = String(week).padStart(2, "0");
      return this.fetchStaticParquet(
        `/conversations/year=${year}/week=${weekStr}/threads.parquet`
      );
    }
    return this.fetchParquet(`/archive/threads?year=${year}&week=${week}`);
  }

  /**
   * Search messages via Meilisearch.
   * Only available in "api" mode.
   * @param query Search query string
   * @param limit Maximum number of results (default: 20)
   */
  async search(query: string, limit: number = 20): Promise<SearchResponse> {
    if (this.mode === "static") {
      throw new SlackArchiveError(
        "Search is not available in static mode",
        501
      );
    }
    const url = `${this.baseUrl}/archive/search?query=${encodeURIComponent(query)}&limit=${limit}`;
    const response = await this.fetchFn(url, { method: "POST" });
    return this.handleJsonResponse<SearchResponse>(response);
  }

  /**
   * Check if the server/files are reachable.
   * In "api" mode, pings the threads-in-range endpoint.
   * In "static" mode, tries to fetch users.parquet.
   */
  async ping(): Promise<boolean> {
    try {
      if (this.mode === "static") {
        const response = await this.fetchFn(`${this.baseUrl}/users.parquet`, {
          method: "HEAD",
        });
        return response.ok;
      }
      await this.getThreadsInRange("2020-01-01", "2020-01-07");
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Get the current client mode
   */
  getMode(): ClientMode {
    return this.mode;
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Private: API mode helpers
  // ─────────────────────────────────────────────────────────────────────────

  private async fetchParquet(path: string): Promise<ArrayBuffer> {
    const url = `${this.baseUrl}${path}`;
    const response = await this.fetchFn(url);

    if (!response.ok) {
      await this.throwError(response);
    }

    return response.arrayBuffer();
  }

  private async handleJsonResponse<T>(response: Response): Promise<T> {
    if (!response.ok) {
      await this.throwError(response);
    }
    return response.json() as Promise<T>;
  }

  private async throwError(response: Response): Promise<never> {
    let serverError: string | undefined;
    try {
      const errorBody = (await response.json()) as ErrorResponse;
      serverError = errorBody.error;
    } catch {
      // Response body is not JSON
    }

    throw new SlackArchiveError(
      serverError ?? `HTTP ${response.status}: ${response.statusText}`,
      response.status,
      serverError
    );
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Private: Static mode helpers
  // ─────────────────────────────────────────────────────────────────────────

  private async fetchStaticParquet(path: string): Promise<ArrayBuffer> {
    const url = `${this.baseUrl}${path}`;
    const response = await this.fetchFn(url);

    if (!response.ok) {
      throw new SlackArchiveError(
        `File not found: ${path}`,
        response.status
      );
    }

    return response.arrayBuffer();
  }

  /**
   * In static mode, we don't have an API to query available weeks.
   * Instead, we generate all possible weeks in the range and probe for files.
   */
  private async probeThreadsInRange(
    from: string,
    to: string
  ): Promise<ThreadsInRangeResponse> {
    const possibleWeeks = this.generateWeeksInRange(from, to);
    const available: YearWeek[] = [];

    // Probe all weeks in parallel
    const probes = possibleWeeks.map(async ({ year, week }) => {
      const weekStr = String(week).padStart(2, "0");
      const url = `${this.baseUrl}/conversations/year=${year}/week=${weekStr}/threads.parquet`;

      try {
        const response = await this.fetchFn(url, { method: "HEAD" });
        if (response.ok) {
          return { year, week };
        }
      } catch {
        // Network error, file doesn't exist
      }
      return null;
    });

    const results = await Promise.all(probes);
    for (const result of results) {
      if (result) {
        available.push(result);
      }
    }

    // Sort by year, then week
    available.sort((a, b) =>
      a.year !== b.year ? a.year - b.year : a.week - b.week
    );

    return { available };
  }

  /**
   * Generate all ISO week numbers between two dates
   */
  private generateWeeksInRange(from: string, to: string): YearWeek[] {
    const weeks: YearWeek[] = [];
    const startDate = new Date(from);
    const endDate = new Date(to);

    const startIso = this.getISOWeek(startDate);
    const endIso = this.getISOWeek(endDate);

    let currentYear = startIso.year;
    let currentWeek = startIso.week;

    while (
      currentYear < endIso.year ||
      (currentYear === endIso.year && currentWeek <= endIso.week)
    ) {
      weeks.push({ year: currentYear, week: currentWeek });

      currentWeek++;

      // Check if we need to move to next year
      const lastWeekOfYear = this.getISOWeek(
        new Date(currentYear, 11, 28)
      ).week;
      if (currentWeek > lastWeekOfYear) {
        currentYear++;
        currentWeek = 1;
      }

      // Safety limit (about 4 years)
      if (weeks.length > 200) break;
    }

    return weeks;
  }

  /**
   * Get ISO week number for a date
   */
  private getISOWeek(date: Date): YearWeek {
    const d = new Date(date);
    d.setHours(0, 0, 0, 0);
    d.setDate(d.getDate() + 4 - (d.getDay() || 7));
    const yearStart = new Date(d.getFullYear(), 0, 1);
    const weekNo = Math.ceil(
      ((d.getTime() - yearStart.getTime()) / 86400000 + 1) / 7
    );
    return { year: d.getFullYear(), week: weekNo };
  }
}
