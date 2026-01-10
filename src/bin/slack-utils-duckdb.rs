//! DuckDB CLI for querying parquet exports
//!
//! This binary provides DuckDB-based querying capabilities for parquet exports
//! created by slack-utils. It supports Hive-style partitioning for efficient
//! queries on large conversation exports.

use clap::{Parser, Subcommand};
use slack_utils::duckdb_query::{execute_query, QueryConfig};
use std::process;

/// DuckDB query interface for slack-utils parquet exports
#[derive(Parser)]
#[command(name = "slack-utils-duckdb")]
#[command(about = "Query parquet exports using DuckDB")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a SQL query against parquet files
    ///
    /// The parquet data is exposed as a view named 'data'.
    /// Hive-style partitioning is automatically detected for paths like:
    /// threads/year=2024/week=01/threads.parquet
    Query {
        /// The SQL query to execute (use 'data' as the table name)
        ///
        /// Examples:
        ///   "SELECT * FROM data LIMIT 10"
        ///   "SELECT channel_name, COUNT(*) FROM data GROUP BY channel_name"
        ///   "SELECT * FROM data WHERE year = 2024 AND week = 1"
        query: String,

        /// Path to parquet file or directory with Hive partitioning
        ///
        /// Supports glob patterns for Hive-partitioned directories:
        ///   conversations/year=*/week=*/*.parquet
        ///
        /// For a single file, just provide the path:
        ///   users.parquet
        #[arg(short, long, default_value = "conversations/year=*/week=*/*.parquet")]
        parquet: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Query { query, parquet } => {
            run_query(&query, &parquet);
        }
    }
}

fn run_query(query: &str, parquet_path: &str) {
    let config = QueryConfig {
        parquet_path: parquet_path.to_string(),
        query: query.to_string(),
    };

    match execute_query(&config) {
        Ok(result) => {
            print!("{}", result.format_table());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!("Hints:");
            eprintln!("  - Make sure the parquet file/directory exists");
            eprintln!("  - For Hive-partitioned data, use a glob pattern like:");
            eprintln!("    conversations/year=*/week=*/*.parquet");
            eprintln!("  - Use 'data' as the table name in your query");
            eprintln!("  - Check your SQL syntax");
            process::exit(1);
        }
    }
}
