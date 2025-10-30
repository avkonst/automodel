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

#[derive(Debug)]
struct QueryAnalysis {
    has_sequential_scan: bool,
    cost_estimate: Option<f64>,
    warnings: Vec<String>,
}

#[derive(Debug)]
pub struct QueryAnalysisResult {
    pub query_name: String,
    pub has_sequential_scan: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug)]
pub struct QueryAnalysisResults {
    pub query_results: Vec<QueryAnalysisResult>,
}

/// Main entry point for the automodel library
pub struct AutoModel {
    queries: Vec<QueryDefinition>,
    defaults: Option<DefaultsConfig>,
}

impl AutoModel {
    /// Calculate SHA-256 hash of a file's contents
    pub fn calculate_file_hash(file_path: &str) -> Result<u64, std::io::Error> {
        use sha2::{Digest, Sha256};
        use std::fs;

        let contents = fs::read(file_path)?;
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let result = hasher.finalize();

        // Convert first 8 bytes of SHA-256 to u64 for a stable hash
        let hash_bytes = &result[0..8];
        let mut hash_u64 = 0u64;
        for (i, &byte) in hash_bytes.iter().enumerate() {
            hash_u64 |= (byte as u64) << (i * 8);
        }

        Ok(hash_u64)
    }

    /// Clean up generated files for modules that no longer exist in the YAML config
    fn cleanup_unused_module_files(
        output_path: &std::path::Path,
        current_modules: &std::collections::HashSet<&String>,
    ) -> Result<(), std::io::Error> {
        use std::fs;

        // Read all files in the output directory
        let entries = fs::read_dir(output_path)?;

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
            if !current_modules.iter().any(|&m| m == module_name) {
                let file_path = entry.path();
                println!(
                    "cargo:info=Removing unused module file: {}",
                    file_path.display()
                );
                fs::remove_file(&file_path)?;
            }
        }

        Ok(())
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

    /// Analyze query execution plan to detect potential performance issues
    pub async fn analyze_query_performance(
        database_url: &str,
        sql: &str,
        query_name: &str,
    ) -> Result<QueryAnalysis> {
        use tokio_postgres::{connect, NoTls};

        let (client, connection) = connect(database_url, NoTls).await?;

        // Spawn the connection task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        let mut overall_analysis = QueryAnalysis {
            has_sequential_scan: false,
            cost_estimate: None,
            warnings: Vec::new(),
        };

        // Generate query variants to handle conditional syntax
        let query_variants = Self::generate_query_variants(sql);

        if query_variants.is_empty() {
            return Err(anyhow::anyhow!(
                "No valid query variants found for analysis"
            ));
        }

        // Analyze each variant
        for (i, variant_sql) in query_variants.iter().enumerate() {
            let variant_name = if i == 0 {
                format!("{} (base)", query_name)
            } else {
                format!("{} (variant {})", query_name, i)
            };

            match Self::analyze_single_query(&client, variant_sql, &variant_name).await {
                Ok(variant_analysis) => {
                    if variant_analysis.has_sequential_scan {
                        overall_analysis.has_sequential_scan = true;
                        overall_analysis.warnings.extend(variant_analysis.warnings);
                    }
                }
                Err(e) => {
                    // Log the failure but continue with other queries for now
                    println!(
                        "cargo:warning=Could not analyze query '{}': {}. Continuing with other queries.",
                        variant_name, e
                    );
                }
            }
        }

        Ok(overall_analysis)
    }

    /// Analyze a single SQL query variant
    async fn analyze_single_query(
        client: &tokio_postgres::Client,
        sql: &str,
        query_name: &str,
    ) -> Result<QueryAnalysis> {
        // Convert named parameters ${param} to positional parameters $1, $2, etc.
        let (converted_sql, param_names) =
            crate::type_extraction::convert_named_params_to_positional(sql);

        // Use EXPLAIN to get the query execution plan
        let explain_sql = format!("EXPLAIN (FORMAT TEXT, ANALYZE false) {}", converted_sql);

        let mut analysis = QueryAnalysis {
            has_sequential_scan: false,
            cost_estimate: None,
            warnings: Vec::new(),
        };

        // Execute EXPLAIN query with appropriate parameters
        let query_result = if !param_names.is_empty() {
            // Try to prepare the statement to get parameter types and create appropriate dummy values
            match client.prepare(&converted_sql).await {
                Ok(statement) => {
                    let param_types = statement.params();

                    // We'll handle enum types properly by getting their actual values

                    let mut dummy_params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> =
                        Vec::new();
                    let mut special_params: Vec<(usize, String, String)> = Vec::new(); // (param_index, type_name, value) - for enums and numeric

                    // Create dummy values based on parameter types
                    for param_type in param_types {
                        use tokio_postgres::types::Type;

                        // Check if this is an enum type and get actual enum values
                        if let Ok(Some(enum_info)) =
                            crate::type_extraction::get_enum_type_info(client, param_type.oid())
                                .await
                        {
                            // For enum types, we'll handle them specially by modifying the query
                            // Store enum info for later query modification
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
                            // For numeric types, we'll also handle them by modifying the query
                            special_params.push((
                                dummy_params.len(),
                                "numeric".to_string(),
                                "0".to_string(),
                            ));
                            dummy_params.push(Box::new("NUMERIC_PLACEHOLDER".to_string()));
                            continue;
                        }

                        // Handle built-in PostgreSQL types
                        let dummy_value: Box<dyn tokio_postgres::types::ToSql + Sync> =
                            match param_type {
                                &Type::BOOL => Box::new(false),
                                &Type::INT2 => Box::new(0i16),
                                &Type::INT4 => Box::new(0i32),
                                &Type::INT8 => Box::new(0i64),
                                &Type::FLOAT4 => Box::new(0.0f32),
                                &Type::FLOAT8 => Box::new(0.0f64),
                                &Type::TEXT
                                | &Type::VARCHAR
                                | &Type::CHAR
                                | &Type::BPCHAR
                                | &Type::NAME
                                | &Type::UNKNOWN => Box::new("dummy".to_string()),
                                &Type::BYTEA => Box::new(vec![0u8]),
                                &Type::JSON | &Type::JSONB => Box::new(serde_json::Value::Null),
                                &Type::TIMESTAMPTZ => Box::new(chrono::Utc::now()),
                                &Type::TIMESTAMP => Box::new(
                                    chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
                                ),
                                &Type::DATE => {
                                    Box::new(chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap())
                                }
                                &Type::TIME | &Type::TIMETZ => {
                                    Box::new(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                                }
                                &Type::UUID => Box::new(uuid::Uuid::nil()),
                                _ => {
                                    // Handle other types by name
                                    match param_type.name() {
                                        "numeric" => Box::new("0".to_string()), // PostgreSQL accepts string for numeric
                                        _ => Box::new("dummy".to_string()), // Fallback for unknown types
                                    }
                                }
                            };
                        dummy_params.push(dummy_value);
                    }

                    // Handle enum parameters by modifying the query to cast enum values
                    let (final_explain_sql, filtered_params) = if special_params.is_empty() {
                        // No enum parameters, use original approach
                        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                            dummy_params.iter().map(|p| p.as_ref()).collect();
                        (explain_sql.clone(), param_refs)
                    } else {
                        // Replace special parameters (enums and numeric) with cast values and adjust remaining parameters
                        let mut modified_sql = converted_sql.clone();
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
                        let mut sorted_special_params = special_params.clone();
                        sorted_special_params.sort_by(|a, b| b.0.cmp(&a.0));

                        for (param_index, param_type, param_value) in sorted_special_params {
                            let old_placeholder = format!("${}", param_index + 1);
                            let cast_value = if param_type == "numeric" {
                                // For numeric, cast as numeric literal
                                format!("{}::numeric", param_value)
                            } else {
                                // For enums, cast with quoted value
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

                        let final_sql =
                            format!("EXPLAIN (FORMAT TEXT, ANALYZE false) {}", modified_sql);
                        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                            param_mapping
                                .iter()
                                .map(|&i| dummy_params[i].as_ref())
                                .collect();

                        (final_sql, param_refs)
                    };

                    client.query(&final_explain_sql, &filtered_params).await
                }
                Err(e) => {
                    // If prepare fails, it might be due to complex syntax, skip analysis
                    return Err(anyhow::anyhow!(
                        "Failed to prepare statement for analysis: {}",
                        e
                    ));
                }
            }
        } else {
            // No parameters, execute directly
            client.query(&explain_sql, &[]).await
        };

        match query_result {
            Ok(rows) => {
                // PostgreSQL returns EXPLAIN as text lines
                for row in rows {
                    let plan_line: String = row.get(0);
                    if plan_line.contains("Seq Scan") {
                        analysis.has_sequential_scan = true;

                        // Extract table name from the plan line
                        // Format is usually "Seq Scan on table_name"
                        if let Some(on_pos) = plan_line.find(" on ") {
                            let after_on = &plan_line[on_pos + 4..];
                            let table_name =
                                after_on.split_whitespace().next().unwrap_or("unknown");

                            let scan_detail = format!("Sequential scan on table '{}'", table_name);
                            analysis.warnings.push(scan_detail);

                            println!(
                                "cargo:warning=Query '{}' performs sequential scan on table '{}' - consider adding indexes",
                                query_name, table_name
                            );
                        }
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("EXPLAIN failed: {}", e));
            }
        }

        Ok(analysis)
    }

    /// Check if query analysis should be performed for this query
    fn should_analyze_query(&self, query: &QueryDefinition) -> bool {
        // Check per-query setting first
        if let Some(analyze_query) = query.analyze_query {
            return analyze_query;
        }

        // Fall back to global setting
        if let Some(ref defaults) = self.defaults {
            return defaults.analyze_queries;
        }

        // Default is false (no analysis)
        false
    }

    /// Check if generated code is up to date by comparing file hash
    fn is_generated_code_up_to_date<P: AsRef<Path>, Q: AsRef<Path>>(
        yaml_path: P,
        generated_file: Q,
    ) -> Result<bool> {
        use std::fs;

        // If generated file doesn't exist, we need to regenerate
        if !generated_file.as_ref().exists() {
            return Ok(false);
        }

        // Calculate current YAML hash
        let yaml_path_str = yaml_path
            .as_ref()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("YAML path contains invalid UTF-8"))?;
        let current_hash = Self::calculate_file_hash(yaml_path_str)?;

        // Read first line of generated file to check for hash comment
        let generated_content = fs::read_to_string(generated_file)?;
        let first_line = generated_content.lines().next().unwrap_or("");

        // Look for hash comment pattern: // AUTOMODEL_HASH: <hash>
        if let Some(hash_comment) = first_line.strip_prefix("// AUTOMODEL_HASH: ") {
            if let Ok(stored_hash) = hash_comment.trim().parse::<u64>() {
                return Ok(stored_hash == current_hash);
            }
        }

        // No valid hash found, need to regenerate
        Ok(false)
    }

    /// Create a new AutoModel instance by loading queries from a YAML file
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = parse_yaml_file(path).await?;

        Ok(Self {
            queries: config.queries,
            defaults: config.defaults,
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
        self.generate_code_for_module_with_hash(database_url, module, None)
            .await
    }

    /// Generate Rust code for queries in a specific module with optional hash header
    /// If module is None, generates code for queries without a module specified
    /// If yaml_hash is provided, adds hash comment at the top for caching
    pub async fn generate_code_for_module_with_hash(
        &self,
        database_url: &str,
        module: Option<&str>,
        yaml_hash: Option<u64>,
    ) -> Result<String> {
        let mut generated_code = String::new();

        // Add hash comment at the top if provided
        if let Some(hash) = yaml_hash {
            generated_code.push_str(&format!("// AUTOMODEL_HASH: {}\n", hash));
            generated_code.push_str(
                "// This file was automatically generated by AutoModel. Do not edit manually.\n\n",
            );
        }

        // Filter queries for this module
        let module_queries: Vec<&QueryDefinition> = self
            .queries
            .iter()
            .filter(|q| q.module.as_deref() == module)
            .collect();

        if module_queries.is_empty() {
            return Ok(generated_code);
        }

        // Collect type information for all queries in this module
        let mut type_infos = Vec::new();
        for query in &module_queries {
            let type_info =
                extract_query_types(database_url, &query.sql, query.types.as_ref()).await?;
            type_infos.push(type_info);

            // Analyze query performance if enabled
            let should_analyze = self.should_analyze_query(query);
            if should_analyze {
                println!("cargo:info=Analyzing query '{}'", query.name);
                let _analysis =
                    Self::analyze_query_performance(database_url, &query.sql, &query.name).await?;
                // Analysis warnings are printed in the analyze_query_performance function
            } else {
                println!(
                    "cargo:info=Skipping analysis for query '{}' (disabled)",
                    query.name
                );
            }
        }

        // Check if any query has output types (needs Row trait for try_get method)
        let needs_row_import = type_infos.iter().any(|ti| !ti.output_types.is_empty());
        if needs_row_import {
            generated_code.push_str("use sqlx::Row;\n\n");
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
            let function_code =
                generate_function_code_without_enums(query, type_info, self.defaults.as_ref())?;
            generated_code.push_str(&function_code);
            generated_code.push('\n');
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
    /// - Check if AUTOMODEL_DATABASE_URL environment variable is set
    /// - Calculate hash of YAML file and check if generated code is up to date
    /// - If generated code is up to date, skip database connection entirely
    /// - If not up to date and DATABASE_URL is set, regenerate code
    /// - If not up to date and no DATABASE_URL, fail the build
    /// - Organize functions into modules based on the `module` field in queries
    /// - Generate separate .rs files for each module and a main mod.rs that includes them
    /// - Add hash comments to generated files for future caching
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

        // Calculate hash of YAML file for caching
        let yaml_hash = Self::calculate_file_hash(yaml_file)?;

        // Load current YAML to determine modules for cleanup (even if cached)
        let automodel_for_modules = AutoModel::new(yaml_file).await?;
        let modules = automodel_for_modules.get_modules();
        let module_set: std::collections::HashSet<_> = modules.iter().collect();

        // Create output directory if it doesn't exist
        fs::create_dir_all(output_path)?;

        // Always run cleanup to remove unused module files, even if using cache
        Self::cleanup_unused_module_files(output_path, &module_set)?;

        // Check if generated code is up to date
        let mod_file_path = output_path.join("mod.rs");
        let code_up_to_date =
            Self::is_generated_code_up_to_date(yaml_file, &mod_file_path).unwrap_or(false);

        if code_up_to_date {
            println!("cargo:info=Generated code is up to date, skipping database connection");
            return Ok(());
        }

        // Generated code is out of date, need to regenerate
        println!("cargo:info=YAML file changed or generated code missing, regeneration required");

        // Check for database URL (try AUTOMODEL_DATABASE_URL first, then fall back to DATABASE_URL)
        let database_url = env::var("AUTOMODEL_DATABASE_URL")
            .or_else(|_| env::var("DATABASE_URL"))
            .map_err(|_| {
                eprintln!("cargo:error=AUTOMODEL_DATABASE_URL (or DATABASE_URL) environment variable must be set for code generation");
                eprintln!("cargo:error=Set AUTOMODEL_DATABASE_URL to your PostgreSQL connection string");
                std::io::Error::new(std::io::ErrorKind::NotFound, "AUTOMODEL_DATABASE_URL environment variable not set")
            })?;

        println!("cargo:info=Database URL found, generating database functions...");

        // Create AutoModel instance and load queries from YAML file
        let automodel = AutoModel::new(yaml_file).await?;

        let mut mod_declarations = Vec::new();

        // Generate code for queries without a module (main mod.rs content)
        // Don't add hash to main module code since we'll add it to mod.rs directly
        let main_module_code = automodel
            .generate_code_for_module(&database_url, None)
            .await?;

        // Generate separate files for each named module
        for module in &modules {
            let module_code = automodel
                .generate_code_for_module_with_hash(&database_url, Some(module), Some(yaml_hash))
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

        // Add hash comment at the top for caching
        mod_content.push_str(&format!("// AUTOMODEL_HASH: {}\n", yaml_hash));
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

        // Add the main module code (functions without a specific module)
        // Skip if it's empty or just the hash header
        let trimmed_main_code = main_module_code.trim();
        if !trimmed_main_code.is_empty()
            && !trimmed_main_code.starts_with("// AUTOMODEL_HASH:")
            && trimmed_main_code
                .lines()
                .any(|line| !line.starts_with("//") && !line.trim().is_empty())
        {
            mod_content.push_str(&main_module_code);
        }

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

        Ok(())
    }
}
