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
    queries: Vec<QueryDefinition>,
    field_type_mappings: Option<HashMap<String, String>>,
}

impl AutoModel {
    /// Create a new AutoModel instance by loading queries from a YAML file
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = parse_yaml_file(path).await?;
        Ok(Self {
            queries: config.queries,
            field_type_mappings: config.types,
        })
    }

    /// Generate Rust code for all loaded queries
    pub async fn generate_code(&self, database_url: &str) -> Result<String> {
        let mut db = DatabaseConnection::new(database_url).await?;
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
