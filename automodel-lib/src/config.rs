use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Expected result type for a query
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedResult {
    /// Exactly one row must be returned (uses query_one, fails if 0 or >1 rows)
    ExactlyOne,
    /// Zero or one row expected (uses query_opt, returns Option)
    PossibleOne,
    /// At least one row expected (uses query, fails if 0 rows, returns Vec with first element guaranteed)
    AtLeastOne,
    /// Multiple rows expected (uses query, returns Vec which may be empty)
    Multiple,
}

/// OpenTelemetry instrumentation level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryLevel {
    /// No instrumentation
    None,
    /// Basic span creation with function name
    Info,
    /// Include SQL query in span
    Debug,
    /// Include both SQL query and parameters in span
    Trace,
}

impl Default for TelemetryLevel {
    fn default() -> Self {
        TelemetryLevel::None
    }
}

/// Default configuration for telemetry and analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DefaultsConfig {
    /// Global telemetry defaults
    #[serde(default)]
    pub telemetry: DefaultsTelemetryConfig,
    /// Whether to analyze query performance and warn about sequential scans
    /// Defaults to false
    #[serde(default)]
    pub ensure_indexes: Option<bool>,
    /// Default module for queries without a module specified
    /// If not specified, the function will be generated in queries.rs by default
    #[serde(default)]
    pub module: Option<String>,
}

/// Default configuration for telemetry and analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DefaultsTelemetryConfig {
    /// Global telemetry level
    #[serde(default)]
    pub level: Option<TelemetryLevel>,
    /// Whether to include SQL queries as fields in spans by default
    /// Defaults to false
    #[serde(default)]
    pub include_sql: Option<bool>,
}

impl Default for ExpectedResult {
    fn default() -> Self {
        ExpectedResult::ExactlyOne
    }
}

/// Represents a single SQL query definition from the YAML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryDefinition {
    /// The name of the query, which will be used as the function name
    pub name: String,
    /// The SQL query string
    pub sql: String,
    /// Optional description of what the query does
    pub description: Option<String>,
    /// Optional module name where this function should be generated
    /// If not specified, the function will be generated in queries.rs by default
    /// Must be a valid Rust module name (alphanumeric + underscore, starting with letter/underscore)
    pub module: Option<String>,
    /// Expected result type - controls fetch method and error handling
    /// Defaults to "exactly_one" if not specified
    #[serde(default)]
    pub expect: ExpectedResult,
    /// Optional per-query field type mappings
    /// Key: field name (e.g., "profile", "metadata", "status")
    /// Value: Rust type to use (e.g., "crate::models::UserProfile", "MyStruct")
    pub types: Option<HashMap<String, String>>,
    /// Optional telemetry configuration for this query
    pub telemetry: Option<QueryTelemetryConfig>,
    /// Whether to analyze this query's performance (overrides global setting)
    /// Defaults to None (use global setting)
    #[serde(default)]
    pub ensure_indexes: Option<bool>,
}

/// Per-query telemetry configuration
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct QueryTelemetryConfig {
    /// Override global telemetry level for this query
    #[serde(default)]
    pub level: Option<TelemetryLevel>,
    /// List of input parameter names to include in the span
    /// If not specified or empty, all parameters will be skipped (skip_all)
    #[serde(default)]
    pub include_params: Option<Vec<String>>,
    /// Whether to include the SQL query as a field in the span
    /// Defaults to false
    #[serde(default)]
    pub include_sql: Option<bool>,
}

/// Root structure for the YAML file containing multiple queries
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    /// List of SQL queries
    #[serde(default)]
    pub queries: Vec<QueryDefinition>,
    /// Default configuration for telemetry and analysis
    #[serde(default)]
    pub defaults: DefaultsConfig,
}
