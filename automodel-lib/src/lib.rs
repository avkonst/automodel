mod codegen;
mod definition;
mod sqlparser;
mod type_extraction;

use codegen::*;
use definition::*;
use sqlparser::*;
use type_extraction::*;

use anyhow::Result;
use std::path::Path;

pub use definition::TelemetryLevel;

/// Default configuration for telemetry and analysis
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DefaultsConfig {
    /// Global telemetry defaults
    pub telemetry: DefaultsTelemetryConfig,
    /// Whether to analyze query performance and warn about sequential scans
    /// Defaults to false
    pub ensure_indexes: bool,
}

/// Default configuration for telemetry and analysis
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DefaultsTelemetryConfig {
    /// Global telemetry level
    pub level: TelemetryLevel,
    /// Whether to include SQL queries as fields in spans by default
    /// Defaults to false
    pub include_sql: bool,
}

/// Main entry point for the automodel library
pub struct AutoModel {
    queries: Vec<QueryDefinition>,
}

impl AutoModel {
    /// Create a new AutoModel instance by loading queries from SQL files in a directory
    /// with explicit defaults configuration (no YAML file required)
    pub async fn new<P: AsRef<Path>>(queries_dir: P, defaults: DefaultsConfig) -> Result<Self> {
        // Scan SQL files from the queries directory
        let queries = scan_sql_files(queries_dir.as_ref(), defaults).await?;

        Ok(Self { queries })
    }

    /// Build script helper for automatically generating code at build time.
    ///
    /// This function should be called from your build.rs script. It will:
    /// - Calculate hash of YAML file and check if generated code is up to date
    /// - If generated code is up to date, skip database connection entirely
    /// - If not up to date and AUTOMODEL_DATABASE_URL is set, regenerate code
    /// - If not up to date and no AUTOMODEL_DATABASE_URL, fail the build
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
        queries_dir: &str,
        output_dir: &str,
        defaults: crate::DefaultsConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use sha2::{Digest, Sha256};
        use std::fs;

        println!("cargo:rerun-if-changed={}", output_dir);

        let output_path = Path::new(output_dir);
        let mod_file = output_path.join("mod.rs");
        println!("cargo:rerun-if-changed={}", mod_file.display());

        let mut hasher = Sha256::new();

        let queries_dir = Path::new(queries_dir);
        if queries_dir.exists() && queries_dir.is_dir() {
            println!("cargo:rerun-if-changed={}", queries_dir.display());
            // Collect all SQL files and sort them for deterministic hashing
            let mut sql_files = Vec::new();
            for module_entry in fs::read_dir(queries_dir)? {
                let module_entry = module_entry?;
                let module_path = module_entry.path();
                if module_path.is_dir() {
                    println!("cargo:rerun-if-changed={}", module_path.display());
                    for sql_entry in fs::read_dir(&module_path)? {
                        let sql_entry = sql_entry?;
                        let sql_path = sql_entry.path();
                        if sql_path.extension().and_then(|e| e.to_str()) == Some("sql") {
                            println!("cargo:rerun-if-changed={}", sql_path.display());
                            sql_files.push(sql_path);
                        }
                    }
                    let output_module_path = output_path.join(module_path.file_name().unwrap());
                    println!("cargo:rerun-if-changed={}.rs", output_module_path.display());
                }
            }

            // Sort for deterministic hashing
            sql_files.sort();

            // Hash each SQL file
            for sql_file in sql_files {
                let sql_contents = fs::read(&sql_file)?;
                hasher.update(&sql_contents);
            }
        }

        let result = hasher.finalize();

        // Convert first 8 bytes of SHA-256 to u64 for a stable hash
        let hash_bytes = &result[0..8];
        let mut hash_u64 = 0u64;
        for (i, &byte) in hash_bytes.iter().enumerate() {
            hash_u64 |= (byte as u64) << (i * 8);
        }
        let source_hash = hash_u64;
        // Check if generated code is up to date
        if Self::is_generated_mod_rs_code_up_to_date(source_hash, &mod_file).unwrap_or(false) {
            println!("cargo:info=Skipping code generation as everything is up to date");
            return Ok(());
        }

        let database_url = std::env::var("AUTOMODEL_DATABASE_URL")
            .map_err(|_| {
                println!("cargo:error=AUTOMODEL_DATABASE_URL environment variable must be set for code generation");
                std::io::Error::new(std::io::ErrorKind::NotConnected, "AUTOMODEL_DATABASE_URL environment variable not set")
            })?;

        let automodel = AutoModel::new(queries_dir, defaults).await?;
        automodel
            .generate_to_directory(&database_url, output_dir, source_hash)
            .await?;

        Ok(())
    }

    /// Get all unique module names from the loaded queries
    fn get_modules(&self) -> Vec<String> {
        let mut modules: Vec<String> = self
            .queries
            .iter()
            .map(|q| q.module.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        modules.sort();
        modules
    }

    /// Check if generated code is up to date by comparing file hash
    fn is_generated_mod_rs_code_up_to_date<Q: AsRef<Path>>(
        source_hash: u64,
        generated_mod_rs_file: Q,
    ) -> Result<bool> {
        use std::fs;

        // If generated file doesn't exist, we need to regenerate
        if !generated_mod_rs_file.as_ref().exists() {
            return Ok(false);
        }

        // Read first line of generated file to check for hash comment
        let generated_content = fs::read_to_string(generated_mod_rs_file)?;
        let first_line = generated_content.lines().next().unwrap_or("");

        if let Some(hash_comment) = first_line.strip_prefix("// AUTOMODEL_HASH: ") {
            if let Ok(generated_source_hash) = hash_comment.trim().parse::<u64>() {
                return Ok(generated_source_hash == source_hash);
            }
        }

        // No valid hash found, need to regenerate
        Ok(false)
    }

    /// Generate code to output directory with provided database URL
    async fn generate_to_directory(
        &self,
        database_url: &str,
        output_dir: &str,
        source_hash: u64,
    ) -> anyhow::Result<()> {
        use std::fs;
        use std::path::Path;
        use tokio_postgres::{connect, NoTls};

        let output_path = Path::new(output_dir);
        let modules = self.get_modules();

        // Create output directory
        fs::create_dir_all(output_path)?;

        Self::cleanup_unused_files(output_path, &modules)?;

        let (client, connection) = connect(database_url, NoTls).await?;
        // Spawn the connection task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        // Temporarily disable sequential scans to force index usage in analysis
        // This helps detect queries that would benefit from indexes even with empty/small tables
        client.execute("SET enable_seqscan = false", &[]).await?;

        let mut mod_declarations = Vec::new();

        // Generate separate files for each named module
        for module in &modules {
            let module_code = self.generate_code_for_module(&client, module).await?;
            let module_file = output_path.join(format!("{}.rs", module));
            fs::write(&module_file, &module_code)?;
            mod_declarations.push(format!("pub mod {};", module));
        }

        // Create the main mod.rs file
        let mod_file = output_path.join("mod.rs");
        let mut mod_content = String::new();

        // Add hash comment at the top for consistency with build-time generation
        mod_content.push_str(&format!("// AUTOMODEL_HASH: {}\n", source_hash));
        mod_content.push_str(
            "// This file was automatically generated by AutoModel. Do not edit manually.\n\n",
        );

        // Add module declarations first
        if !mod_declarations.is_empty() {
            for declaration in mod_declarations {
                mod_content.push_str(&declaration);
                mod_content.push('\n');
            }
            mod_content.push('\n');
        }

        // Add generic Error type
        mod_content.push_str(&crate::codegen::generate_generic_error_type());

        fs::write(&mod_file, &mod_content)?;

        Ok(())
    }

    /// Clean up generated files for modules that no longer exist in the YAML config
    fn cleanup_unused_files(
        output_dir: &std::path::Path,
        current_modules: &Vec<String>,
    ) -> Result<(), std::io::Error> {
        use std::fs;

        // Read all files in the output directory
        let entries = fs::read_dir(output_dir)?;

        for entry in entries {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // Skip mod.rs and non-.rs files
            if file_name_str == "mod.rs" || !file_name_str.ends_with(".rs") {
                continue;
            }

            // Extract module name from filename (remove .rs extension)
            let module_name = &file_name_str[..file_name_str.len() - 3];

            // Check if this module still exists in current YAML config
            if !current_modules.iter().any(|m| m == module_name) {
                let file_path = entry.path();
                fs::remove_file(&file_path)?;
            }
        }

        Ok(())
    }

    /// Internal implementation that uses an existing database connection
    async fn generate_code_for_module(
        &self,
        client: &tokio_postgres::Client,
        module: &str,
    ) -> Result<String> {
        let mut generated_code = String::new();

        // Add hash comment at the top if provided
        generated_code.push_str(
            "// This file was automatically generated by AutoModel. Do not edit manually.\n\n",
        );

        // Filter queries for this module
        let module_queries: Vec<&QueryDefinition> =
            self.queries.iter().filter(|q| q.module == module).collect();

        if module_queries.is_empty() {
            return Ok(generated_code);
        }

        // Collect type information for all queries in this module
        let mut type_infos = Vec::new();
        let mut query_constraints: Vec<Vec<crate::type_extraction::ConstraintInfo>> = Vec::new();

        for query in &module_queries {
            let type_info = extract_query_types(client, &query.sql, query.types.as_ref()).await?;
            type_infos.push(type_info);

            // Analyze query performance if enabled
            if query.ensure_indexes {
                let _analysis =
                    Self::analyze_query_performance(client, &query.sql, &query.name).await?;
                // Analysis warnings are printed in the analyze_query_performance function
            }

            // Detect if this is a mutation query and extract constraints only for mutations
            // For queries with conditional syntax, use the base variant (without conditional blocks)
            let query_for_analysis = Self::remove_conditional_blocks(&query.sql);

            match Self::is_mutation_query(client, &query_for_analysis, &query.name).await {
                true => {
                    // This is a mutation query, extract constraints
                    let (converted_sql, _) =
                        convert_named_params_to_positional(&query_for_analysis);
                    match client.prepare(&converted_sql).await {
                        Ok(statement) => {
                            match extract_constraints_from_statement(client, &statement, &query.sql)
                                .await
                            {
                                Ok(constraints) => {
                                    query_constraints.push(constraints);
                                }
                                Err(e) => {
                                    println!(
                                        "cargo:warning=Failed to extract constraints for query '{}': {}",
                                        query.name, e
                                    );
                                    query_constraints.push(Vec::new());
                                }
                            }
                        }
                        Err(e) => {
                            println!("cargo:info=Failed to prepare statement for constraint extraction for query '{}': {}", query.name, e);
                            query_constraints.push(Vec::new());
                        }
                    }
                }
                false => {
                    // This is a read-only query, no constraints needed
                    query_constraints.push(Vec::new());
                }
            }
        }

        // Check if any query has output types (needs Row trait for try_get method)
        let needs_row_import = type_infos.iter().any(|ti| !ti.output_types.is_empty());
        if needs_row_import {
            generated_code.push_str("use sqlx::Row;\n");
        }

        if needs_row_import {
            generated_code.push_str("\n");
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

        // Track generated structs for validation of struct references
        // Map: struct_name -> Vec<(field_name, field_type)>
        let mut generated_structs: std::collections::HashMap<String, Vec<(String, String)>> =
            std::collections::HashMap::new();

        // Generate functions without enum definitions (since they're already at the top)
        for (query, type_info) in module_queries.iter().zip(type_infos.iter()) {
            // Handle conditions_type struct
            if let Some(struct_name) = query.conditions_type.get_struct_name() {
                // Check if struct already exists
                if !generated_structs.contains_key(struct_name) {
                    // Struct doesn't exist, so we'll generate it
                    let param_names = parse_parameter_names_from_sql(&query.sql);
                    let mut fields = Vec::new();
                    for (i, param_name) in param_names.iter().enumerate() {
                        // Only track conditional parameters (those with '?')
                        if param_name.ends_with('?') {
                            let clean_param = param_name.trim_end_matches('?');
                            if let Some(param_type) = type_info.input_types.get(i) {
                                // For conditions_type, preserve nullable types for NULL support
                                let type_str = if param_type.is_nullable {
                                    format!("Option<{}>", param_type.rust_type)
                                } else {
                                    param_type.rust_type.clone()
                                };
                                if !fields.iter().any(|(name, _)| name == clean_param) {
                                    fields.push((clean_param.to_string(), type_str));
                                }
                            }
                        }
                    }
                    generated_structs.insert(struct_name.to_string(), fields);
                } else {
                    // Struct exists, validate it matches
                    let param_names = parse_parameter_names_from_sql(&query.sql);
                    let conditional_param_names: Vec<String> = param_names
                        .iter()
                        .filter(|p| p.ends_with('?'))
                        .map(|p| p.trim_end_matches('?').to_string())
                        .collect();
                    let conditional_param_types: Vec<_> = param_names
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| p.ends_with('?'))
                        .filter_map(|(i, _)| type_info.input_types.get(i))
                        .cloned()
                        .collect();

                    validate_struct_reference(
                        struct_name,
                        &conditional_param_names,
                        &conditional_param_types,
                        &generated_structs,
                        true, // is_conditional_diff - allow flexible type matching
                    )?;
                }
            } else if query.conditions_type.is_enabled() && type_info.parsed_sql.is_some() {
                // Auto-generate struct with default name
                let struct_name = format!("{}Params", to_pascal_case(&query.name));
                let param_names = parse_parameter_names_from_sql(&query.sql);
                let mut fields = Vec::new();
                for (i, param_name) in param_names.iter().enumerate() {
                    if param_name.ends_with('?') {
                        let clean_param = param_name.trim_end_matches('?');
                        if let Some(param_type) = type_info.input_types.get(i) {
                            // For conditions_type, preserve nullable types for NULL support
                            let type_str = if param_type.is_nullable {
                                format!("Option<{}>", param_type.rust_type)
                            } else {
                                param_type.rust_type.clone()
                            };
                            if !fields.iter().any(|(name, _)| name == clean_param) {
                                fields.push((clean_param.to_string(), type_str));
                            }
                        }
                    }
                }
                generated_structs.insert(struct_name, fields);
            }

            // Handle parameters_type struct
            if let Some(struct_name) = query.parameters_type.get_struct_name() {
                // Check if struct already exists
                if !generated_structs.contains_key(struct_name) {
                    // Struct doesn't exist, so we'll generate it
                    let param_names = parse_parameter_names_from_sql(&query.sql);
                    let mut fields = Vec::new();
                    for (i, param_name) in param_names.iter().enumerate() {
                        let clean_param = param_name.trim_end_matches('?');
                        if let Some(param_type) = type_info.input_types.get(i) {
                            let type_str = if param_type.is_nullable || param_type.is_optional {
                                format!("Option<{}>", param_type.rust_type)
                            } else {
                                param_type.rust_type.clone()
                            };
                            if !fields.iter().any(|(name, _)| name == clean_param) {
                                fields.push((clean_param.to_string(), type_str));
                            }
                        }
                    }
                    generated_structs.insert(struct_name.to_string(), fields);
                } else {
                    // Struct exists, validate it matches
                    let param_names = parse_parameter_names_from_sql(&query.sql);
                    validate_struct_reference(
                        struct_name,
                        &param_names,
                        &type_info.input_types,
                        &generated_structs,
                        false, // not conditional_diff - require exact type match
                    )?;
                }
            } else if query.parameters_type.is_enabled() {
                // Auto-generate struct with default name
                let struct_name = format!("{}Params", to_pascal_case(&query.name));
                let param_names = parse_parameter_names_from_sql(&query.sql);
                let mut fields = Vec::new();
                for (i, param_name) in param_names.iter().enumerate() {
                    let clean_param = param_name.trim_end_matches('?');
                    if let Some(param_type) = type_info.input_types.get(i) {
                        let type_str = if param_type.is_nullable || param_type.is_optional {
                            format!("Option<{}>", param_type.rust_type)
                        } else {
                            param_type.rust_type.clone()
                        };
                        // Only add unique fields
                        if !fields.iter().any(|(name, _)| name == clean_param) {
                            fields.push((clean_param.to_string(), type_str));
                        }
                    }
                }
                generated_structs.insert(struct_name, fields);
            }

            // Track or validate return type struct if this query has one
            if type_info.output_types.len() > 1 {
                // Determine the struct name
                let struct_name = if let Some(ref custom_name) = query.return_type {
                    custom_name.to_string()
                } else {
                    format!("{}Item", to_pascal_case(&query.name))
                };

                // Check if struct already exists
                if generated_structs.contains_key(&struct_name) {
                    // Validate that output columns match the existing struct
                    let existing_fields = generated_structs.get(&struct_name).unwrap();

                    // Build expected fields from output_types
                    let expected_fields: Vec<(String, String)> = type_info
                        .output_types
                        .iter()
                        .map(|ot| {
                            let type_str = if ot.rust_type.is_nullable {
                                format!("Option<{}>", ot.rust_type.rust_type)
                            } else {
                                ot.rust_type.rust_type.clone()
                            };
                            (ot.name.clone(), type_str)
                        })
                        .collect();

                    // Check if field counts match
                    if existing_fields.len() != expected_fields.len() {
                        // Determine which fields are missing and which are redundant
                        let expected_names: std::collections::HashSet<_> =
                            expected_fields.iter().map(|(n, _)| n.as_str()).collect();
                        let existing_names: std::collections::HashSet<_> =
                            existing_fields.iter().map(|(n, _)| n.as_str()).collect();

                        let missing: Vec<_> = expected_fields
                            .iter()
                            .filter(|(n, _)| !existing_names.contains(n.as_str()))
                            .map(|(n, t)| format!("{}: {}", n, t))
                            .collect();

                        let redundant: Vec<_> = existing_fields
                            .iter()
                            .filter(|(n, _)| !expected_names.contains(n.as_str()))
                            .map(|(n, t)| format!("{}: {}", n, t))
                            .collect();

                        let mut error_msg = format!(
                            "Query '{}' return type references struct '{}' but field count mismatch: expected {} fields, found {} fields in struct",
                            query.name,
                            struct_name,
                            expected_fields.len(),
                            existing_fields.len()
                        );

                        if !missing.is_empty() {
                            error_msg.push_str(&format!(
                                "\n  Fields in query but not in struct: [{}]",
                                missing.join(", ")
                            ));
                        }

                        if !redundant.is_empty() {
                            error_msg.push_str(&format!(
                                "\n  Fields in struct but not in query: [{}]",
                                redundant.join(", ")
                            ));
                        }

                        anyhow::bail!(error_msg);
                    }

                    // Check if fields match (name and type)
                    for (expected_name, expected_type) in &expected_fields {
                        if let Some(existing) =
                            existing_fields.iter().find(|(n, _)| n == expected_name)
                        {
                            if &existing.1 != expected_type {
                                anyhow::bail!(
                                    "Query '{}' return type references struct '{}' but field '{}' has incompatible type: expected '{}', found '{}'",
                                    query.name,
                                    struct_name,
                                    expected_name,
                                    expected_type,
                                    existing.1
                                );
                            }
                        } else {
                            anyhow::bail!(
                                "Query '{}' return type references struct '{}' but field '{}' is missing",
                                query.name,
                                struct_name,
                                expected_name
                            );
                        }
                    }
                } else {
                    // Struct doesn't exist yet, track it for generation
                    let mut fields = Vec::new();
                    for output_type in &type_info.output_types {
                        let field_name = output_type.name.clone();
                        let type_str = if output_type.rust_type.is_nullable {
                            format!("Option<{}>", output_type.rust_type.rust_type)
                        } else {
                            output_type.rust_type.rust_type.clone()
                        };
                        fields.push((field_name, type_str));
                    }
                    generated_structs.insert(struct_name, fields);
                }
            }
        }

        // Track constraint enums for validation of error_type reuse
        // Map: enum_name -> Vec<constraint_name>
        let mut generated_constraint_enums: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        // Validate or track error_type constraint enums
        for (query, constraints) in module_queries.iter().zip(query_constraints.iter()) {
            if let Some(ref enum_name) = query.error_type {
                // error_type is specified, validate it
                let expected_constraints: Vec<String> =
                    constraints.iter().map(|c| c.name.clone()).collect();

                // Check if enum already exists
                if let Some(existing_constraints) = generated_constraint_enums.get(enum_name) {
                    // Validate that constraints match exactly
                    if existing_constraints.len() != expected_constraints.len() {
                        let existing_set: std::collections::HashSet<_> =
                            existing_constraints.iter().collect();
                        let expected_set: std::collections::HashSet<_> =
                            expected_constraints.iter().collect();

                        let missing: Vec<_> = expected_constraints
                            .iter()
                            .filter(|c| !existing_set.contains(c))
                            .map(|c| c.as_str())
                            .collect();

                        let redundant: Vec<_> = existing_constraints
                            .iter()
                            .filter(|c| !expected_set.contains(c))
                            .map(|c| c.as_str())
                            .collect();

                        let mut error_msg = format!(
                                "Query '{}' error_type references enum '{}' but constraint mismatch:\n  Expected {} constraints in query, but enum has {} constraints",
                                query.name,
                                enum_name,
                                expected_constraints.len(),
                                existing_constraints.len()
                            );

                        if !missing.is_empty() {
                            error_msg.push_str(&format!(
                                "\n  Constraints in query but not in enum: [{}]",
                                missing.join(", ")
                            ));
                        }

                        if !redundant.is_empty() {
                            error_msg.push_str(&format!(
                                "\n  Constraints in enum but not in query: [{}]",
                                redundant.join(", ")
                            ));
                        }

                        anyhow::bail!(error_msg);
                    }

                    // Check if all constraints match exactly
                    for expected_constraint in &expected_constraints {
                        if !existing_constraints.contains(expected_constraint) {
                            anyhow::bail!(
                                    "Query '{}' error_type references enum '{}' but constraint '{}' is missing from the enum",
                                    query.name,
                                    enum_name,
                                    expected_constraint
                                );
                        }
                    }
                } else {
                    // First query using this enum, track it
                    generated_constraint_enums.insert(enum_name.to_string(), expected_constraints);
                }
            } else if !constraints.is_empty() {
                // No error_type specified, but query has constraints - auto-generate with default name
                let enum_name = format!("{}Constraints", to_pascal_case(&query.name));
                let expected_constraints: Vec<String> =
                    constraints.iter().map(|c| c.name.clone()).collect();
                generated_constraint_enums.insert(enum_name, expected_constraints);
            }
            // If no error_type and no constraints, it's a read-only query - nothing to track
        }

        // Track which struct names have been emitted to avoid duplicates
        let mut emitted_struct_names = std::collections::HashSet::new();

        // Generate functions with per-query constraint enums
        for ((query, type_info), constraints) in module_queries
            .iter()
            .zip(type_infos.iter())
            .zip(query_constraints.iter())
        {
            let function_code = generate_function_code_without_enums(
                query,
                type_info,
                &mut emitted_struct_names,
                constraints,
            )?;
            generated_code.push_str(&function_code);
            generated_code.push('\n');
        }

        Ok(generated_code)
    }

    /// Generate SQL query variants for analysis by handling conditional syntax
    fn generate_query_variants(sql: &str) -> Vec<String> {
        let mut variants = Vec::new();

        // First variant: remove all conditional blocks $[...]
        let base_query = Self::remove_conditional_blocks(sql);
        if !base_query.trim().is_empty() {
            variants.push(base_query);
        }

        // Additional variants: include each conditional block separately
        let conditional_variants = Self::extract_conditional_variants(sql);
        variants.extend(conditional_variants);

        variants
    }

    /// Remove all conditional blocks $[...] from SQL
    fn remove_conditional_blocks(sql: &str) -> String {
        let mut result = sql.to_string();

        // Remove $[...] blocks using simple string replacement
        while let Some(start) = result.find("$[") {
            if let Some(end) = result[start..].find("]") {
                let end_pos = start + end + 1;
                result.replace_range(start..end_pos, "");
            } else {
                break;
            }
        }

        // Clean up extra whitespace
        result = result.replace("  ", " ").trim().to_string();
        result
    }

    /// Extract variants where each conditional block is included
    fn extract_conditional_variants(sql: &str) -> Vec<String> {
        let mut variants = Vec::new();
        let mut pos = 0;

        while let Some(start) = sql[pos..].find("$[") {
            let start_pos = pos + start;
            if let Some(end) = sql[start_pos..].find("]") {
                let end_pos = start_pos + end + 1;
                let conditional_content = &sql[start_pos + 2..end_pos - 1]; // Remove $[ and ]

                // Create variant with this conditional block included
                let mut variant = sql.to_string();
                variant.replace_range(start_pos..end_pos, conditional_content);

                // Remove any remaining conditional blocks from this variant
                variant = Self::remove_conditional_blocks(&variant);

                if !variant.trim().is_empty() {
                    variants.push(variant);
                }

                pos = end_pos;
            } else {
                break;
            }
        }

        variants
    }

    /// Create dummy parameter values for EXPLAIN queries
    /// Returns (dummy_params, special_params) where special_params contains info about enums and numeric types
    async fn create_dummy_params(
        client: &tokio_postgres::Client,
        param_types: &[tokio_postgres::types::Type],
    ) -> Result<(
        Vec<Box<dyn tokio_postgres::types::ToSql + Sync>>,
        Vec<(usize, String, String)>,
    )> {
        use tokio_postgres::types::Type;

        let mut dummy_params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> = Vec::new();
        let mut special_params: Vec<(usize, String, String)> = Vec::new(); // (param_index, type_name, value)

        for param_type in param_types {
            // Check if this is an enum type and get actual enum values
            if let Ok(Some(enum_info)) =
                crate::type_extraction::get_enum_type_info(client, param_type.oid()).await
            {
                special_params.push((
                    dummy_params.len(),
                    enum_info.type_name.clone(),
                    enum_info.variants[0].clone(),
                ));
                dummy_params.push(Box::new("ENUM_PLACEHOLDER".to_string()));
                continue;
            }

            // Handle numeric type specially - PostgreSQL is strict about numeric conversion
            if param_type.name() == "numeric" {
                special_params.push((dummy_params.len(), "numeric".to_string(), "0".to_string()));
                dummy_params.push(Box::new("NUMERIC_PLACEHOLDER".to_string()));
                continue;
            }

            // Handle range types - these need special casting
            if param_type.name().ends_with("range") {
                let type_name = param_type.name();
                special_params.push((
                    dummy_params.len(),
                    type_name.to_string(),
                    "empty".to_string(),
                ));
                dummy_params.push(Box::new("RANGE_PLACEHOLDER".to_string()));
                continue;
            }

            // Handle geometric types - these need special casting
            let geometric_default = match param_type.name() {
                "point" => Some("(0,0)"),
                "line" => Some("{0,0,0}"),
                "lseg" => Some("[(0,0),(0,0)]"),
                "box" => Some("((0,0),(0,0))"),
                "path" => Some("[(0,0)]"),
                "polygon" => Some("((0,0))"),
                "circle" => Some("<(0,0),0>"),
                _ => None,
            };
            if let Some(default_value) = geometric_default {
                let type_name = param_type.name();
                special_params.push((
                    dummy_params.len(),
                    type_name.to_string(),
                    default_value.to_string(),
                ));
                dummy_params.push(Box::new("GEOMETRIC_PLACEHOLDER".to_string()));
                continue;
            }

            // Handle built-in PostgreSQL types
            let dummy_value: Box<dyn tokio_postgres::types::ToSql + Sync> = match param_type {
                // Boolean & Numeric Types
                &Type::BOOL => Box::new(false),
                &Type::CHAR => Box::new(0i8),
                &Type::INT2 => Box::new(0i16),
                &Type::INT4 => Box::new(0i32),
                &Type::INT8 => Box::new(0i64),
                &Type::FLOAT4 => Box::new(0.0f32),
                &Type::FLOAT8 => Box::new(0.0f64),
                &Type::OID | &Type::REGPROC | &Type::XID | &Type::CID => Box::new(0u32),

                // String & Text Types
                &Type::TEXT
                | &Type::VARCHAR
                | &Type::BPCHAR
                | &Type::NAME
                | &Type::XML
                | &Type::UNKNOWN => Box::new("dummy".to_string()),

                // Binary & Bit Types
                &Type::BYTEA => Box::new(vec![0u8]),

                // JSON Types
                &Type::JSON | &Type::JSONB => Box::new(serde_json::Value::Null),

                // Date & Time Types
                &Type::TIMESTAMPTZ => Box::new(chrono::Utc::now()),
                &Type::TIMESTAMP => {
                    Box::new(chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc())
                }
                &Type::DATE => Box::new(chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()),
                &Type::TIME => Box::new(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()),

                // UUID
                &Type::UUID => Box::new(uuid::Uuid::nil()),

                // Array types - use empty arrays
                &Type::BOOL_ARRAY => Box::new(Vec::<bool>::new()),
                &Type::CHAR_ARRAY => Box::new(Vec::<i8>::new()),
                &Type::INT2_ARRAY => Box::new(Vec::<i16>::new()),
                &Type::INT4_ARRAY => Box::new(Vec::<i32>::new()),
                &Type::INT8_ARRAY => Box::new(Vec::<i64>::new()),
                &Type::FLOAT4_ARRAY => Box::new(Vec::<f32>::new()),
                &Type::FLOAT8_ARRAY => Box::new(Vec::<f64>::new()),
                &Type::TEXT_ARRAY
                | &Type::VARCHAR_ARRAY
                | &Type::BPCHAR_ARRAY
                | &Type::NAME_ARRAY
                | &Type::XML_ARRAY => Box::new(Vec::<String>::new()),
                &Type::BYTEA_ARRAY => Box::new(Vec::<Vec<u8>>::new()),
                &Type::JSON_ARRAY | &Type::JSONB_ARRAY => Box::new(Vec::<serde_json::Value>::new()),
                &Type::DATE_ARRAY => Box::new(Vec::<chrono::NaiveDate>::new()),
                &Type::TIME_ARRAY => Box::new(Vec::<chrono::NaiveTime>::new()),
                &Type::TIMESTAMP_ARRAY => Box::new(Vec::<chrono::NaiveDateTime>::new()),
                &Type::TIMESTAMPTZ_ARRAY => Box::new(Vec::<chrono::DateTime<chrono::Utc>>::new()),
                &Type::UUID_ARRAY => Box::new(Vec::<uuid::Uuid>::new()),

                // Fallback for unknown types - use string
                _ => Box::new("dummy".to_string()),
            };
            dummy_params.push(dummy_value);
        }

        Ok((dummy_params, special_params))
    }

    /// Prepare EXPLAIN query with proper parameter handling
    /// Returns (final_explain_sql, param_refs) ready for execution
    fn prepare_explain_query<'a>(
        base_sql: &str,
        dummy_params: &'a [Box<dyn tokio_postgres::types::ToSql + Sync>],
        special_params: &[(usize, String, String)],
    ) -> (String, Vec<&'a (dyn tokio_postgres::types::ToSql + Sync)>) {
        let explain_sql = format!("EXPLAIN (FORMAT TEXT, ANALYZE false) {}", base_sql);

        if special_params.is_empty() {
            // No special parameters, use original approach
            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                dummy_params.iter().map(|p| p.as_ref()).collect();
            (explain_sql, param_refs)
        } else {
            // Replace special parameters (enums and numeric) with cast values
            let mut modified_sql = base_sql.to_string();
            let mut param_mapping = Vec::new();

            // Build new parameter mapping (non-special parameters only)
            for (i, _) in dummy_params.iter().enumerate() {
                if !special_params
                    .iter()
                    .any(|(special_idx, _, _)| *special_idx == i)
                {
                    param_mapping.push(i);
                }
            }

            // Replace parameters from highest index to lowest to avoid position shifts
            let mut sorted_special_params = special_params.to_vec();
            sorted_special_params.sort_by(|a, b| b.0.cmp(&a.0));

            for (param_index, param_type, param_value) in sorted_special_params {
                let old_placeholder = format!("${}", param_index + 1);
                let cast_value = if param_type == "numeric" {
                    format!("{}::numeric", param_value)
                } else {
                    format!("'{}'::{}", param_value, param_type)
                };
                modified_sql = modified_sql.replace(&old_placeholder, &cast_value);
            }

            // Renumber remaining parameters
            let mut new_param_num = 1;
            for &original_index in &param_mapping {
                let old_placeholder = format!("${}", original_index + 1);
                let new_placeholder = format!("${}", new_param_num);
                modified_sql = modified_sql.replace(&old_placeholder, &new_placeholder);
                new_param_num += 1;
            }

            let final_sql = format!("EXPLAIN (FORMAT TEXT, ANALYZE false) {}", modified_sql);
            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = param_mapping
                .iter()
                .map(|&i| dummy_params[i].as_ref())
                .collect();

            (final_sql, param_refs)
        }
    }

    /// Detect if a query is a mutation (INSERT/UPDATE/DELETE) using keyword detection
    /// Returns Ok(true) for mutations, Ok(false) for read-only queries
    async fn is_mutation_query(
        _client: &tokio_postgres::Client,
        sql: &str,
        _query_name: &str,
    ) -> bool {
        // Case-insensitive mutation keyword detection with word boundaries
        // Use regex-like word boundary check to avoid false positives like "updated_at" matching "UPDATE"
        let mutation_keywords = ["insert", "update", "delete", "modify", "merge"];
        let sql_lower = sql.to_lowercase();

        // Check if any mutation keyword appears as a standalone word (not part of identifier)
        let is_mutation = mutation_keywords.iter().any(|kw| {
            // Look for keyword followed by whitespace or common SQL punctuation
            sql_lower.contains(&format!("{} ", kw))
                || sql_lower.contains(&format!("{}(", kw))
                || sql_lower.starts_with(kw)
        });

        is_mutation
    }

    /// Analyze query execution plan to detect potential performance issues

    async fn analyze_query_performance(
        client: &tokio_postgres::Client,
        sql: &str,
        query_name: &str,
    ) -> Result<()> {
        // Generate query variants to handle conditional syntax
        let query_variants = Self::generate_query_variants(sql);

        // Analyze each variant
        for (i, variant_sql) in query_variants.iter().enumerate() {
            let variant_name = if i == 0 {
                format!("{} (base)", query_name)
            } else {
                format!("{} (variant {})", query_name, i)
            };

            Self::analyze_single_query(client, variant_sql, &variant_name).await?;
        }

        Ok(())
    }

    /// Analyze a single SQL query variant
    async fn analyze_single_query(
        client: &tokio_postgres::Client,
        sql: &str,
        query_name: &str,
    ) -> Result<()> {
        // Convert named parameters ${param} to positional parameters $1, $2, etc.
        let (converted_sql, param_names) =
            crate::type_extraction::convert_named_params_to_positional(sql);

        // Execute EXPLAIN query with appropriate parameters
        let query_result = if !param_names.is_empty() {
            match client.prepare(&converted_sql).await {
                Ok(statement) => {
                    let param_types = statement.params();

                    // Create dummy parameters with proper type handling
                    let (dummy_params, special_params) =
                        Self::create_dummy_params(client, param_types).await?;

                    // Prepare EXPLAIN query
                    let (explain_query, param_refs) =
                        Self::prepare_explain_query(&converted_sql, &dummy_params, &special_params);

                    client.query(&explain_query, &param_refs).await
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Failed to prepare statement for analysis: {}",
                        e
                    ));
                }
            }
        } else {
            // No parameters, execute directly
            let explain_sql = format!("EXPLAIN (FORMAT TEXT, ANALYZE false) {}", converted_sql);
            client.query(&explain_sql, &[]).await
        };

        let Ok(rows) = query_result else {
            println!("cargo:warning=Query '{}' had EXPLAIN failed", query_name);
            return Ok(());
        };

        let mut has_sequential_scan = false;
        // PostgreSQL returns EXPLAIN as text lines
        for row in rows {
            let plan_line: String = row.get(0);

            // Check for sequential scans
            if plan_line.contains("Seq Scan") {
                has_sequential_scan = true;

                // Extract table name from the plan line
                // Format is usually "Seq Scan on table_name"
                if let Some(on_pos) = plan_line.find(" on ") {
                    let after_on = &plan_line[on_pos + 4..];
                    let table_name = after_on.split_whitespace().next().unwrap_or("unknown");

                    println!(
                        "cargo:warning=Query '{}' performs sequential scan on table '{}'",
                        query_name, table_name
                    );
                }
            }

            // Also check for expensive operations that might indicate missing indexes
            if plan_line.contains("Index Scan") && plan_line.contains("rows=") {
                // This is good - index is being used
            } else if plan_line.contains("Filter:") || plan_line.contains("Sort") {
                // These operations on large tables might benefit from indexes
                // But only report if we haven't already flagged a sequential scan
                if !has_sequential_scan && plan_line.contains("Filter:") {
                    if let Some(on_pos) = plan_line.find(" on ") {
                        let after_on = &plan_line[on_pos + 4..];
                        let table_name = after_on.split_whitespace().next().unwrap_or("unknown");

                        println!(
                            "cargo:warning=Query '{}' uses filtering on table '{}' - verify appropriate indexes exist",
                            query_name, table_name
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
