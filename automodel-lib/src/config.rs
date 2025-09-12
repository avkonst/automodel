use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a single SQL query definition from the YAML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryDefinition {
    /// The name of the query, which will be used as the function name
    pub name: String,
    /// The SQL query string
    pub sql: String,
    /// Optional description of what the query does
    pub description: Option<String>,
    /// Optional tags for categorization
    pub tags: Option<Vec<String>>,
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
