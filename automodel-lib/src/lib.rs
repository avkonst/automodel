mod codegen;
mod config;
mod type_extraction;
mod yaml_parser;

use codegen::*;
use config::*;
use type_extraction::*;
use yaml_parser::*;

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
        let mut generated_code = String::new();

        // Add imports first
        if self.field_type_mappings.is_some() {
            generated_code.push_str("use serde::{Serialize, Deserialize};\n");
            generated_code.push_str("use tokio_postgres::types::{FromSql, ToSql, Type};\n");
            generated_code.push_str("use std::error::Error;\n\n");
        }

        for query in &self.queries {
            let type_info =
                extract_query_types(database_url, &query.sql, self.field_type_mappings.as_ref())
                    .await?;
            let function_code = generate_function_code(query, &type_info)?;
            generated_code.push_str(&function_code);
            generated_code.push('\n');
        }

        // Add JSON wrapper helper at the end if we have custom field type mappings
        if self.field_type_mappings.is_some() {
            generated_code.push_str(&generate_json_wrapper_helper());
        }

        Ok(generated_code)
    }

    /// Get all loaded queries
    pub fn queries(&self) -> &[QueryDefinition] {
        &self.queries
    }

    /// Build script helper for automatically generating code at build time.
    ///
    /// This function should be called from your build.rs script. It will:
    /// - Check if DATABASE_URL environment variable is set
    /// - If set, generate code and fail the build if something goes wrong
    /// - If not set, skip code generation silently (letting compilation fail if generated code is used)
    ///
    /// # Arguments
    ///
    /// * `yaml_file` - Path to the YAML file containing query definitions (relative to build.rs)
    /// * `output_dir` - Path to the directory where mod.rs will be written (relative to build.rs, typically "src/generated")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// // build.rs
    /// use automodel::AutoModel;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     AutoModel::generate_at_build_time("queries.yaml", "src/generated").await?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn generate_at_build_time(
        yaml_file: &str,
        output_dir: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::env;
        use std::fs;
        use std::path::Path;

        let output_path = Path::new(output_dir);
        let mod_file = output_path.join("mod.rs");

        // Tell cargo to rerun if the input YAML file changes
        println!("cargo:rerun-if-changed={}", yaml_file);
        // Tell cargo to rerun if the mod.rs file is manually modified
        println!("cargo:rerun-if-changed={}", mod_file.display());

        // Check if DATABASE_URL environment variable is set
        match env::var("DATABASE_URL") {
            Ok(database_url) => {
                // DATABASE_URL is set - generate code and fail build if something goes wrong
                println!("cargo:info=DATABASE_URL found, generating database functions...");

                // Create AutoModel instance and load queries from YAML file
                let automodel = AutoModel::new(yaml_file).await?;

                // Generate Rust code
                let generated_code = automodel.generate_code(&database_url).await?;

                // Create output directory if it doesn't exist
                fs::create_dir_all(output_path)?;

                // Write the generated code to mod.rs in the output directory
                fs::write(&mod_file, &generated_code)?;

                println!(
                    "cargo:info=Successfully generated database functions at {}",
                    mod_file.display()
                );
            }
            Err(_) => {
                // DATABASE_URL is not set - skip codegen silently
                println!("cargo:info=DATABASE_URL not set, skipping code generation");
                // Don't generate any code - let the app fail compilation if it tries to use generated functions
            }
        }

        Ok(())
    }
}
