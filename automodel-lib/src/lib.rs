mod codegen;
mod config;
mod type_extraction;
mod yaml_parser;

use codegen::*;
use config::*;
use type_extraction::*;
use yaml_parser::*;

use anyhow::Result;
use std::path::Path;

/// Main entry point for the automodel library
pub struct AutoModel {
    queries: Vec<QueryDefinition>,
}

impl AutoModel {
    /// Create a new AutoModel instance by loading queries from a YAML file
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = parse_yaml_file(path).await?;

        Ok(Self {
            queries: config.queries,
        })
    }

    /// Generate Rust code for all loaded queries
    pub async fn generate_code(&self, database_url: &str) -> Result<String> {
        self.generate_code_for_module(database_url, None).await
    }

    /// Generate Rust code for queries in a specific module
    /// If module is None, generates code for queries without a module specified
    pub async fn generate_code_for_module(
        &self,
        database_url: &str,
        module: Option<&str>,
    ) -> Result<String> {
        let mut generated_code = String::new();

        // Filter queries for this module
        let module_queries: Vec<&QueryDefinition> = self
            .queries
            .iter()
            .filter(|q| q.module.as_deref() == module)
            .collect();

        if module_queries.is_empty() {
            return Ok(generated_code);
        }

        // Check if any query in this module has custom type mappings
        let has_custom_types = module_queries.iter().any(|q| q.types.is_some());

        // Add imports first - SQLx Row is always needed for generated functions
        generated_code.push_str("use sqlx::Row;\n");
        if has_custom_types {
            generated_code.push_str("use serde::{Serialize, Deserialize};\n");
            generated_code.push_str("use sqlx::{FromRow, Type};\n");
            generated_code.push_str("use std::error::Error;\n");
        }
        generated_code.push_str("\n");

        // Collect type information for all queries in this module
        let mut type_infos = Vec::new();
        for query in &module_queries {
            let type_info =
                extract_query_types(database_url, &query.sql, query.types.as_ref()).await?;
            type_infos.push(type_info);
        }

        // Extract and generate all unique enum types for this module
        let mut all_enum_types = std::collections::HashMap::new();
        for type_info in &type_infos {
            let enum_types = extract_enum_types(&type_info.input_types, &type_info.output_types);
            for (enum_name, enum_variants, pg_type_name) in enum_types {
                all_enum_types.insert(enum_name, (enum_variants, pg_type_name));
            }
        }

        // Generate enum definitions once at the top of the module
        for (enum_name, (enum_variants, pg_type_name)) in all_enum_types {
            generated_code.push_str(&generate_enum_definition(
                &enum_variants,
                &enum_name,
                &pg_type_name,
            ));
            generated_code.push('\n');
        }

        // Generate functions without enum definitions (since they're already at the top)
        for (query, type_info) in module_queries.iter().zip(type_infos.iter()) {
            let function_code = generate_function_code_without_enums(query, type_info)?;
            generated_code.push_str(&function_code);
            generated_code.push('\n');
        }

        // Add JSON wrapper helper at the end if we have custom field type mappings
        if has_custom_types {
            generated_code.push_str(&generate_json_wrapper_helper());
        }

        Ok(generated_code)
    }

    /// Get all unique module names from the loaded queries
    pub fn get_modules(&self) -> Vec<String> {
        let mut modules: Vec<String> = self
            .queries
            .iter()
            .filter_map(|q| q.module.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        modules.sort();
        modules
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
    /// - Organize functions into modules based on the `module` field in queries
    /// - Generate separate .rs files for each module and a main mod.rs that includes them
    ///
    /// # Arguments
    ///
    /// * `yaml_file` - Path to the YAML file containing query definitions (relative to build.rs)
    /// * `output_dir` - Path to the directory where module files will be written (relative to build.rs, typically "src/generated")
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

        // Tell cargo to rerun if the input YAML file changes
        println!("cargo:rerun-if-changed={}", yaml_file);

        // Check if DATABASE_URL environment variable is set
        match env::var("DATABASE_URL") {
            Ok(database_url) => {
                // DATABASE_URL is set - generate code and fail build if something goes wrong
                println!("cargo:info=DATABASE_URL found, generating database functions...");

                // Create AutoModel instance and load queries from YAML file
                let automodel = AutoModel::new(yaml_file).await?;

                // Create output directory if it doesn't exist
                fs::create_dir_all(output_path)?;

                // Get all unique modules
                let modules = automodel.get_modules();
                let mut mod_declarations = Vec::new();

                // Generate code for queries without a module (main mod.rs content)
                let main_module_code = automodel
                    .generate_code_for_module(&database_url, None)
                    .await?;

                // Generate separate files for each named module
                for module in &modules {
                    let module_code = automodel
                        .generate_code_for_module(&database_url, Some(module))
                        .await?;
                    let module_file = output_path.join(format!("{}.rs", module));
                    fs::write(&module_file, &module_code)?;
                    mod_declarations.push(format!("pub mod {};", module));

                    // Tell cargo to rerun if any module file is manually modified
                    println!("cargo:rerun-if-changed={}", module_file.display());
                }

                // Create the main mod.rs file
                let mod_file = output_path.join("mod.rs");
                let mut mod_content = String::new();

                // Add module declarations first
                if !mod_declarations.is_empty() {
                    for declaration in mod_declarations {
                        mod_content.push_str(&declaration);
                        mod_content.push('\n');
                    }
                    mod_content.push('\n');
                }

                // Add the main module code (functions without a specific module)
                mod_content.push_str(&main_module_code);

                fs::write(&mod_file, &mod_content)?;

                // Tell cargo to rerun if the mod.rs file is manually modified
                println!("cargo:rerun-if-changed={}", mod_file.display());

                println!(
                    "cargo:info=Successfully generated database functions at {}",
                    output_path.display()
                );
                if !modules.is_empty() {
                    println!("cargo:info=Generated modules: {}", modules.join(", "));
                }
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
