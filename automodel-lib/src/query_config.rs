use serde::{Deserialize, Serialize};

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
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self::new()
    }
}
