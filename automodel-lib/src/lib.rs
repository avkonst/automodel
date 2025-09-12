pub mod build_helper;
pub mod code_generation;
pub mod db_connection;
pub mod query_config;
pub mod type_extraction;
pub mod yaml_parser;

pub use build_helper::*;
pub use code_generation::*;
pub use db_connection::*;
pub use query_config::*;
pub use type_extraction::*;
pub use yaml_parser::*;

use anyhow::Result;
use std::path::Path;

/// Main entry point for the automodel library
pub struct AutoModel {
    database_url: String,
    queries: Vec<QueryDefinition>,
}

impl AutoModel {
    /// Create a new AutoModel instance
    pub fn new(database_url: String) -> Self {
        Self {
            database_url,
            queries: Vec::new(),
        }
    }

    /// Load queries from a YAML file
    pub async fn load_queries_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.queries = parse_yaml_file(path).await?;
        Ok(())
    }

    /// Generate Rust code for all loaded queries
    pub async fn generate_code(&self) -> Result<String> {
        let mut db = DatabaseConnection::new(&self.database_url).await?;
        let mut generated_code = String::new();

        for query in &self.queries {
            let type_info = extract_query_types(&mut db, &query.sql).await?;
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
