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
    /// If not specified, the function will be generated in mod.rs
    /// Must be a valid Rust module name (alphanumeric + underscore, starting with letter/underscore)
    pub module: Option<String>,
    /// Expected result type - controls fetch method and error handling
    /// Defaults to "exactly_one" if not specified
    #[serde(default)]
    pub expect: ExpectedResult,
}

/// Root structure for the YAML file containing multiple queries
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// List of SQL queries
    pub queries: Vec<QueryDefinition>,
    /// Optional field-specific type mappings
    /// Key format: "schema.table.field" or "table.field" (e.g., "public.users.profile" or "users.profile")
    /// Value: Rust type to use (e.g., "crate::models::UserProfile", "MyStruct")
    #[serde(alias = "field_type_mappings")]
    pub types: Option<HashMap<String, String>>,
}

impl Config {
    /// Create a new empty query configuration
    pub fn new() -> Self {
        Self {
            queries: Vec::new(),
            types: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
