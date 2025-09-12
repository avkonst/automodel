pub mod build_helper;
pub mod code_generation;
pub mod db_connection;
pub mod query_config;
pub mod type_extraction;
pub mod yaml_parser;

pub use build_helper::*;
pub use code_generation::{generate_json_wrapper_helper, *};
pub use db_connection::*;
pub use query_config::*;
pub use type_extraction::*;
pub use yaml_parser::*;

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

/// Main entry point for the automodel library
pub struct AutoModel {
    database_url: String,
    queries: Vec<QueryDefinition>,
    field_type_mappings: Option<HashMap<String, String>>,
}

impl AutoModel {
    /// Create a new AutoModel instance
    pub fn new(database_url: String) -> Self {
        Self {
            database_url,
            queries: Vec::new(),
            field_type_mappings: None,
        }
    }

    /// Load queries from a YAML file
    pub async fn load_queries_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let config = parse_yaml_file_full(path).await?;
        self.queries = config.queries;
        self.field_type_mappings = config.field_type_mappings;
        Ok(())
    }

    /// Generate Rust code for all loaded queries
    pub async fn generate_code(&self) -> Result<String> {
        let mut db = DatabaseConnection::new(&self.database_url).await?;
        let mut generated_code = String::new();

        // Add JSON wrapper helper if we have custom field type mappings
        if self.field_type_mappings.is_some() {
            generated_code.push_str(&generate_json_wrapper_helper());
            generated_code.push('\n');
        }

        for query in &self.queries {
            let type_info =
                extract_query_types(&mut db, &query.sql, self.field_type_mappings.as_ref()).await?;
            let function_code = generate_function_code(query, &type_info)?;
            generated_code.push_str(&function_code);
            generated_code.push('\n');
        }

        Ok(generated_code)
    }

    /// Get all loaded queries
    pub fn queries(&self) -> &[QueryDefinition] {
        &self.queries
    }
}
