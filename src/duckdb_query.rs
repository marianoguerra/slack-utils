//! DuckDB query functionality for parquet exports

use duckdb::Connection;
use std::fmt::Write as FmtWrite;
use thiserror::Error;

/// Errors for DuckDB operations
#[derive(Error, Debug)]
pub enum DuckDbError {
    #[error("failed to open DuckDB connection: {0}")]
    ConnectionFailed(#[from] duckdb::Error),

    #[error("failed to execute query: {0}")]
    QueryFailed(String),

    #[error("no results returned from query")]
    NoResults,
}

/// Result type for DuckDB operations
pub type Result<T> = std::result::Result<T, DuckDbError>;

/// Configuration for DuckDB queries
#[derive(Debug, Clone)]
pub struct QueryConfig {
    /// Path to the parquet file or directory (supports Hive partitioning globs)
    pub parquet_path: String,
    /// The SQL query to execute (use 'data' as the table name)
    pub query: String,
}

/// Result of a query execution
#[derive(Debug)]
pub struct QueryResult {
    /// Column names
    pub columns: Vec<String>,
    /// Rows of data (each row is a vector of string values)
    pub rows: Vec<Vec<String>>,
}

impl QueryResult {
    /// Format the results as a table for terminal display
    pub fn format_table(&self) -> String {
        if self.columns.is_empty() {
            return String::from("(no columns)");
        }

        if self.rows.is_empty() {
            return String::from("(no rows)");
        }

        // Calculate column widths
        let mut widths: Vec<usize> = self.columns.iter().map(|c| c.len()).collect();
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.len());
                }
            }
        }

        let mut output = String::new();

        // Header
        let header: Vec<String> = self
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{:width$}", c, width = widths[i]))
            .collect();
        let _ = writeln!(output, "{}", header.join(" | "));

        // Separator
        let separator: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
        let _ = writeln!(output, "{}", separator.join("-+-"));

        // Rows
        for row in &self.rows {
            let cells: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let width = widths.get(i).copied().unwrap_or(0);
                    format!("{:width$}", cell, width = width)
                })
                .collect();
            let _ = writeln!(output, "{}", cells.join(" | "));
        }

        let _ = writeln!(output, "\n({} rows)", self.rows.len());

        output
    }
}

/// Execute a DuckDB query against parquet files
///
/// The parquet_path can be:
/// - A single file: `conversations.parquet`
/// - A glob pattern: `threads/year=*/week=*/*.parquet`
/// - A directory with Hive partitioning: `threads/`
pub fn execute_query(config: &QueryConfig) -> Result<QueryResult> {
    let conn = Connection::open_in_memory()?;

    // Disable progress bar for cleaner output
    conn.execute_batch("SET enable_progress_bar = false;")?;

    // Create a view from the parquet file(s)
    let create_view = format!(
        "CREATE VIEW data AS SELECT * FROM read_parquet('{}', hive_partitioning = true)",
        config.parquet_path
    );
    conn.execute(&create_view, [])
        .map_err(|e| DuckDbError::QueryFailed(format!("Failed to read parquet: {}", e)))?;

    // Execute the user's query
    let mut stmt = conn
        .prepare(&config.query)
        .map_err(|e| DuckDbError::QueryFailed(e.to_string()))?;

    // Execute query and collect results
    let mut result_rows = stmt
        .query([])
        .map_err(|e| DuckDbError::QueryFailed(e.to_string()))?;

    // Get column names after execution
    let column_count = result_rows.as_ref().map(|s| s.column_count()).unwrap_or(0);
    let columns: Vec<String> = (0..column_count)
        .map(|i| {
            result_rows
                .as_ref()
                .and_then(|s| s.column_name(i).ok())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "?".to_string())
        })
        .collect();

    // Collect all rows
    let mut rows = Vec::new();
    while let Some(row) = result_rows
        .next()
        .map_err(|e| DuckDbError::QueryFailed(e.to_string()))?
    {
        let mut values = Vec::with_capacity(column_count);
        for i in 0..column_count {
            let value: duckdb::types::Value = row
                .get(i)
                .map_err(|e| DuckDbError::QueryFailed(e.to_string()))?;
            values.push(format_value(&value));
        }
        rows.push(values);
    }

    Ok(QueryResult { columns, rows })
}

/// Format a DuckDB value for display
fn format_value(value: &duckdb::types::Value) -> String {
    match value {
        duckdb::types::Value::Null => String::from("NULL"),
        duckdb::types::Value::Boolean(b) => b.to_string(),
        duckdb::types::Value::TinyInt(n) => n.to_string(),
        duckdb::types::Value::SmallInt(n) => n.to_string(),
        duckdb::types::Value::Int(n) => n.to_string(),
        duckdb::types::Value::BigInt(n) => n.to_string(),
        duckdb::types::Value::HugeInt(n) => n.to_string(),
        duckdb::types::Value::UTinyInt(n) => n.to_string(),
        duckdb::types::Value::USmallInt(n) => n.to_string(),
        duckdb::types::Value::UInt(n) => n.to_string(),
        duckdb::types::Value::UBigInt(n) => n.to_string(),
        duckdb::types::Value::Float(f) => f.to_string(),
        duckdb::types::Value::Double(d) => d.to_string(),
        duckdb::types::Value::Decimal(d) => d.to_string(),
        duckdb::types::Value::Timestamp(_, n) => n.to_string(),
        duckdb::types::Value::Text(s) => s.clone(),
        duckdb::types::Value::Blob(b) => format!("<blob {} bytes>", b.len()),
        duckdb::types::Value::Date32(d) => d.to_string(),
        duckdb::types::Value::Time64(_, t) => t.to_string(),
        _ => format!("{:?}", value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_table_empty_columns() {
        let result = QueryResult {
            columns: vec![],
            rows: vec![],
        };
        assert_eq!(result.format_table(), "(no columns)");
    }

    #[test]
    fn test_format_table_no_rows() {
        let result = QueryResult {
            columns: vec!["id".to_string(), "name".to_string()],
            rows: vec![],
        };
        assert_eq!(result.format_table(), "(no rows)");
    }

    #[test]
    fn test_format_table_with_data() {
        let result = QueryResult {
            columns: vec!["id".to_string(), "name".to_string()],
            rows: vec![
                vec!["1".to_string(), "Alice".to_string()],
                vec!["2".to_string(), "Bob".to_string()],
            ],
        };
        let table = result.format_table();
        assert!(table.contains("id"));
        assert!(table.contains("name"));
        assert!(table.contains("Alice"));
        assert!(table.contains("Bob"));
        assert!(table.contains("(2 rows)"));
    }
}
