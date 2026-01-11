import type {
  SlackArchiveClientOptions,
  ThreadsInRangeResponse,
  SearchResponse,
  ErrorResponse,
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
 * Client for interacting with the slack-archive-server HTTP API
 */
export class SlackArchiveClient {
  private readonly baseUrl: string;
  private readonly fetchFn: typeof fetch;

  constructor(options: SlackArchiveClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/$/, "");
    this.fetchFn = options.fetch ?? ((...args) => fetch(...args));
  }

  /**
   * Fetch users.parquet file as an ArrayBuffer
   */
  async getUsers(): Promise<ArrayBuffer> {
    return this.fetchParquet("/archive/users");
  }

  /**
   * Fetch channels.parquet file as an ArrayBuffer
   */
  async getChannels(): Promise<ArrayBuffer> {
    return this.fetchParquet("/archive/channels");
  }

  /**
   * Get available year/week partitions within a date range
   * @param from Start date in ISO format (YYYY-MM-DD)
   * @param to End date in ISO format (YYYY-MM-DD)
   */
  async getThreadsInRange(
    from: string,
    to: string
  ): Promise<ThreadsInRangeResponse> {
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
    return this.fetchParquet(
      `/archive/threads?year=${year}&week=${week}`
    );
  }

  /**
   * Search messages via Meilisearch
   * @param query Search query string
   * @param limit Maximum number of results (default: 20)
   */
  async search(query: string, limit: number = 20): Promise<SearchResponse> {
    const url = `${this.baseUrl}/archive/search?query=${encodeURIComponent(query)}&limit=${limit}`;
    const response = await this.fetchFn(url, { method: "POST" });
    return this.handleJsonResponse<SearchResponse>(response);
  }

  /**
   * Check if the server is reachable by fetching threads-in-range with a minimal date range
   */
  async ping(): Promise<boolean> {
    try {
      await this.getThreadsInRange("2020-01-01", "2020-01-07");
      return true;
    } catch {
      return false;
    }
  }

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
}
