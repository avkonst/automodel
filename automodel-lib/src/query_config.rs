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
pub struct QueryConfig {
    /// List of SQL queries
    pub queries: Vec<QueryDefinition>,
    /// Optional metadata about the query collection
    pub metadata: Option<QueryMetadata>,
    /// Optional field-specific type mappings
    /// Key format: "schema.table.field" or "table.field" (e.g., "public.users.profile" or "users.profile")
    /// Value: Rust type to use (e.g., "crate::models::UserProfile", "MyStruct")
    #[serde(alias = "field_type_mappings")]
    pub types: Option<HashMap<String, String>>,
}

/// Optional metadata for the query collection
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryMetadata {
    /// Version of the query collection
    pub version: Option<String>,
    /// Description of the query collection
    pub description: Option<String>,
    /// Author information
    pub author: Option<String>,
}

impl QueryConfig {
    /// Create a new empty query configuration
    pub fn new() -> Self {
        Self {
            queries: Vec::new(),
            metadata: None,
            types: None,
        }
    }

    /// Add a query to the configuration
    pub fn add_query(&mut self, query: QueryDefinition) {
        self.queries.push(query);
    }

    /// Get all queries
    pub fn queries(&self) -> &[QueryDefinition] {
        &self.queries
    }

    /// Get custom type mapping for a specific field
    /// Supports both "schema.table.field" and "table.field" formats
    pub fn get_field_type_mapping(&self, table_name: &str, field_name: &str) -> Option<&String> {
        if let Some(mappings) = &self.types {
            // Try "table.field" format first
            let table_field_key = format!("{}.{}", table_name, field_name);
            if let Some(rust_type) = mappings.get(&table_field_key) {
                return Some(rust_type);
            }

            // Try "schema.table.field" format (assume "public" schema if not specified)
            let schema_table_field_key = format!("public.{}.{}", table_name, field_name);
            mappings.get(&schema_table_field_key)
        } else {
            None
        }
    }

    /// Get all field type mappings
    pub fn field_type_mappings(&self) -> Option<&HashMap<String, String>> {
        self.types.as_ref()
    }
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self::new()
    }
}
