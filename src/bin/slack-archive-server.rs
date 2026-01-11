//! Slack Archive Server - HTTP server for serving Slack archive parquet files
//!
//! This binary provides an HTTP API to serve parquet files from a Slack archive.
//! It supports serving users, channels, and conversation thread files,
//! as well as searching via Meilisearch.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use slack_utils::archive_server::{ArchiveService, Config, MeilisearchConfig, YearWeek};
use slack_utils::{query_meilisearch, IndexEntry};
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tower_http::services::ServeDir;

/// Slack Archive Server - HTTP server for Slack archive parquet files
#[derive(Parser)]
#[command(name = "slack-archive-server")]
#[command(about = "HTTP server for serving Slack archive parquet files")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the HTTP server
    Serve {
        /// Path to the configuration file (TOML format)
        #[arg(value_name = "CONFIG_FILE")]
        config: PathBuf,
    },
}

/// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    archive: Arc<ArchiveService>,
    meilisearch: Option<MeilisearchConfig>,
}

/// Query parameters for threads-in-range endpoint
#[derive(Debug, Deserialize)]
struct ThreadsInRangeQuery {
    /// Start date in ISO format (YYYY-MM-DD)
    from: String,
    /// End date in ISO format (YYYY-MM-DD)
    to: String,
}

/// Query parameters for threads endpoint
#[derive(Debug, Deserialize)]
struct ThreadsQuery {
    year: i32,
    week: u32,
}

/// Response for threads-in-range endpoint
#[derive(Debug, Serialize, Deserialize)]
struct ThreadsInRangeResponse {
    available: Vec<YearWeek>,
}

/// Error response
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

impl ErrorResponse {
    fn new(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
        }
    }
}

/// Query parameters for search endpoint
#[derive(Debug, Deserialize)]
struct SearchQuery {
    /// Search query string
    query: String,
    /// Maximum number of results (default: 20)
    #[serde(default = "default_search_limit")]
    limit: usize,
}

fn default_search_limit() -> usize {
    20
}

/// Response for search endpoint
#[derive(Debug, Serialize)]
struct SearchResponse {
    hits: Vec<IndexEntry>,
    processing_time_ms: usize,
    estimated_total_hits: Option<usize>,
}

/// Handler for GET /archive/users
async fn get_users(State(state): State<AppState>) -> impl IntoResponse {
    serve_parquet_file(state.archive.users_path()).await
}

/// Handler for GET /archive/channels
async fn get_channels(State(state): State<AppState>) -> impl IntoResponse {
    serve_parquet_file(state.archive.channels_path()).await
}

/// Handler for GET /archive/threads-in-range
async fn get_threads_in_range(
    State(state): State<AppState>,
    Query(params): Query<ThreadsInRangeQuery>,
) -> impl IntoResponse {
    let from = match chrono::NaiveDate::parse_from_str(&params.from, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(format!(
                    "Invalid 'from' date format: {}. Expected YYYY-MM-DD",
                    params.from
                ))),
            )
                .into_response();
        }
    };

    let to = match chrono::NaiveDate::parse_from_str(&params.to, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(format!(
                    "Invalid 'to' date format: {}. Expected YYYY-MM-DD",
                    params.to
                ))),
            )
                .into_response();
        }
    };

    match state.archive.threads_in_range(from, to) {
        Ok(available) => Json(ThreadsInRangeResponse { available }).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(e.to_string())),
        )
            .into_response(),
    }
}

/// Handler for GET /archive/threads
async fn get_threads(
    State(state): State<AppState>,
    Query(params): Query<ThreadsQuery>,
) -> impl IntoResponse {
    if params.week == 0 || params.week > 53 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(format!(
                "Invalid week: {}. Week must be between 1 and 53",
                params.week
            ))),
        )
            .into_response();
    }

    let path = state.archive.threads_path(params.year, params.week);
    serve_parquet_file(path).await
}

/// Handler for POST /archive/search
async fn post_search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let meilisearch = match &state.meilisearch {
        Some(ms) => ms,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse::new(
                    "Search is not configured. Add [meilisearch] section to config.",
                )),
            )
                .into_response();
        }
    };

    match query_meilisearch(
        &meilisearch.url,
        &meilisearch.api_key,
        &meilisearch.index_name,
        &params.query,
        params.limit,
    )
    .await
    {
        Ok(result) => Json(SearchResponse {
            hits: result.hits,
            processing_time_ms: result.processing_time_ms,
            estimated_total_hits: result.estimated_total_hits,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(format!("Search failed: {}", e))),
        )
            .into_response(),
    }
}

/// Serve a parquet file as a streaming response
async fn serve_parquet_file(path: PathBuf) -> axum::response::Response {
    match File::open(&path).await {
        Ok(file) => {
            let stream = ReaderStream::new(file);
            let body = axum::body::Body::from_stream(stream);

            axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/octet-stream")
                .header(
                    "Content-Disposition",
                    format!(
                        "attachment; filename=\"{}\"",
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("data.parquet")
                    ),
                )
                .body(body)
                .unwrap_or_else(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("Failed to build response")),
                    )
                        .into_response()
                })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!(
                "File not found: {}",
                path.display()
            ))),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(format!("Failed to read file: {}", e))),
        )
            .into_response(),
    }
}

/// Build the router with all archive endpoints
fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/archive/users", get(get_users))
        .route("/archive/channels", get(get_channels))
        .route("/archive/threads-in-range", get(get_threads_in_range))
        .route("/archive/threads", get(get_threads))
        .route("/archive/search", post(post_search))
        .with_state(state)
}

/// Build the complete application router including static file serving
fn build_app(state: AppState, static_assets: Option<&str>) -> Router {
    let api_router = build_router(state);

    match static_assets {
        Some(path) => api_router.fallback_service(ServeDir::new(path)),
        None => api_router,
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config } => {
            if let Err(e) = run_server(&config).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

async fn run_server(config_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_file(config_path)?;

    let archive = Arc::new(ArchiveService::new(&config.slack_archive.base_path));
    let state = AppState {
        archive,
        meilisearch: config.meilisearch.clone(),
    };

    let app = build_app(state, config.server.static_assets.as_deref());

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;

    println!("Starting Slack Archive Server on {}", addr);
    println!("Archive base path: {}", config.slack_archive.base_path);
    if let Some(ref static_path) = config.server.static_assets {
        println!("Serving static assets from: {}", static_path);
    }
    if let Some(ref ms) = config.meilisearch {
        println!("Meilisearch: {} (index: {})", ms.url, ms.index_name);
    }
    println!();
    println!("Endpoints:");
    println!("  GET  /archive/users              - Get users.parquet");
    println!("  GET  /archive/channels           - Get channels.parquet");
    println!("  GET  /archive/threads-in-range   - List available year/weeks in date range");
    println!("        ?from=YYYY-MM-DD&to=YYYY-MM-DD");
    println!("  GET  /archive/threads            - Get threads.parquet for a specific week");
    println!("        ?year=YYYY&week=WW");
    println!("  POST /archive/search             - Search messages via Meilisearch");
    println!("        ?query=<search-query>&limit=<max-results>");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::fs;
    use tempfile::tempdir;
    use tower::ServiceExt;

    fn create_test_app() -> (tempfile::TempDir, Router) {
        let dir = tempdir().unwrap();
        let archive = Arc::new(ArchiveService::new(dir.path()));
        let state = AppState {
            archive,
            meilisearch: None,
        };
        let router = build_router(state);
        (dir, router)
    }

    fn create_threads_partition(dir: &std::path::Path, year: i32, week: u32) {
        let partition_path = dir
            .join("conversations")
            .join(format!("year={}", year))
            .join(format!("week={:02}", week));
        fs::create_dir_all(&partition_path).unwrap();
        fs::write(partition_path.join("threads.parquet"), b"test parquet data").unwrap();
    }

    #[tokio::test]
    async fn test_get_users_not_found() {
        let (_dir, app) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/users")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_users_success() {
        let (dir, app) = create_test_app();
        fs::write(dir.path().join("users.parquet"), b"users data").unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/users")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("Content-Type").unwrap(),
            "application/octet-stream"
        );
    }

    #[tokio::test]
    async fn test_get_channels_not_found() {
        let (_dir, app) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/channels")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_channels_success() {
        let (dir, app) = create_test_app();
        fs::write(dir.path().join("channels.parquet"), b"channels data").unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/channels")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_threads_in_range_invalid_from_date() {
        let (_dir, app) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/threads-in-range?from=invalid&to=2024-01-21")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_threads_in_range_invalid_to_date() {
        let (_dir, app) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/threads-in-range?from=2024-01-15&to=invalid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_threads_in_range_empty() {
        let (_dir, app) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/threads-in-range?from=2024-01-15&to=2024-01-21")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: ThreadsInRangeResponse = serde_json::from_slice(&body).unwrap();
        assert!(result.available.is_empty());
    }

    #[tokio::test]
    async fn test_get_threads_in_range_with_data() {
        let (dir, app) = create_test_app();
        create_threads_partition(dir.path(), 2024, 3);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/threads-in-range?from=2024-01-15&to=2024-01-21")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: ThreadsInRangeResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(result.available.len(), 1);
        assert_eq!(result.available[0].year, 2024);
        assert_eq!(result.available[0].week, 3);
    }

    #[tokio::test]
    async fn test_get_threads_in_range_from_after_to() {
        let (_dir, app) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/threads-in-range?from=2024-01-21&to=2024-01-15")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_threads_not_found() {
        let (_dir, app) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/threads?year=2024&week=3")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_threads_success() {
        let (dir, app) = create_test_app();
        create_threads_partition(dir.path(), 2024, 3);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/threads?year=2024&week=3")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("Content-Type").unwrap(),
            "application/octet-stream"
        );
    }

    #[tokio::test]
    async fn test_get_threads_invalid_week_zero() {
        let (_dir, app) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/threads?year=2024&week=0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_threads_invalid_week_too_high() {
        let (_dir, app) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/threads?year=2024&week=54")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_threads_padded_week() {
        let (dir, app) = create_test_app();
        // Create week 03 (padded)
        create_threads_partition(dir.path(), 2024, 3);

        // Query with unpadded week number
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/archive/threads?year=2024&week=3")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_search_not_configured() {
        let (_dir, app) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/archive/search?query=test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
