use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parameters type configuration - can be either a boolean or a struct name
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ParametersType {
    /// Auto-generate a new struct with name {QueryName}Params
    Enabled(bool),
    /// Use or generate a struct with the given name
    Named(String),
}

impl ParametersType {
    pub fn is_enabled(&self) -> bool {
        match self {
            ParametersType::Enabled(b) => *b,
            ParametersType::Named(_) => true,
        }
    }

    pub fn get_struct_name(&self) -> Option<&str> {
        match self {
            ParametersType::Enabled(_) => None,
            ParametersType::Named(name) => Some(name.as_str()),
        }
    }
}

/// Conditions type configuration - can be either a boolean or a struct name
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ConditionsType {
    /// Auto-generate a new struct with name {QueryName}Params
    Enabled(bool),
    /// Use or generate a struct with the given name
    Named(String),
}

impl ConditionsType {
    pub fn is_enabled(&self) -> bool {
        match self {
            ConditionsType::Enabled(b) => *b,
            ConditionsType::Named(_) => true,
        }
    }

    pub fn get_struct_name(&self) -> Option<&str> {
        match self {
            ConditionsType::Enabled(_) => None,
            ConditionsType::Named(name) => Some(name.as_str()),
        }
    }
}

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

/// Constraint information extracted from database schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintInfo {
    /// Constraint name
    pub name: String,
    /// Constraint type: unique, primary_key, foreign_key, check, not_null
    pub constraint_type: String,
    /// Table name
    pub table_name: String,
    /// Column names involved in the constraint
    pub column_names: Vec<String>,
    /// For foreign keys: referenced table and columns
    pub referenced_table: Option<String>,
    pub referenced_columns: Option<Vec<String>>,
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
    /// Whether to use multiunzip pattern for array parameters
    /// When true, the function accepts a Vec of tuples and unzips them into separate arrays
    /// for binding to UNNEST(...) style queries
    /// Defaults to false
    #[serde(default)]
    pub multiunzip: Option<bool>,
    /// Whether to use diff-based conditional parameters
    /// When true, generates two struct parameters (old and new) and automatically diffs them
    /// When a string, uses or generates a struct with the given name
    /// Defaults to false
    #[serde(default)]
    pub conditions_type: Option<ConditionsType>,
    /// Type of struct to use for parameters
    /// When true, all query parameters are passed as a single struct
    /// When a string, uses or generates a struct with the given name
    /// Ignored if conditions_type is enabled
    /// Defaults to false
    #[serde(default)]
    pub parameters_type: Option<ParametersType>,
    /// Type of struct to use for return values
    /// When None or not specified, uses default {QueryName}Item naming
    /// When Some(name), uses or generates a struct with the given name
    #[serde(default)]
    pub return_type: Option<String>,
    /// Type of constraint enum to use for errors
    /// When None or not specified, uses default {QueryName}Constraints naming
    /// When Some(name), uses or generates a constraint enum with the given name
    /// Only applies to mutation queries
    #[serde(default)]
    pub error_type: Option<String>,
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
