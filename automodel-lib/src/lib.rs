mod codegen;
mod query_definition;
mod query_definition_rt;
mod sqlfile_parser;
mod types_extractor;
mod utils;

use query_definition::*;
use query_definition_rt::*;
use sqlfile_parser::*;
use types_extractor::*;

use anyhow::Result;
use std::path::Path;

pub use query_definition::TelemetryLevel;

use crate::codegen::generate_root_module;

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
    ///     AutoModel::generate(|| {
    ///         if std::env::var("CI").is_err() {
    ///             std::env::var("AUTOMODEL_DATABASE_URL").map_err(|_| {
    ///                 "AUTOMODEL_DATABASE_URL environment variable must be set for code generation"
    ///             })
    ///         } else {
    ///             Err("Detecting not up to date AutoModel generated code in CI environment")
    ///         }
    ///     }, "queries.yaml", "src/generated").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn generate<F>(
        database_url_cb: F,
        queries_dir: &str,
        output_dir: &str,
        defaults: crate::DefaultsConfig,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce() -> Result<String, String>,
    {
        use sha2::{Digest, Sha256};
        use std::fs;

        println!("cargo:rerun-if-changed={}", output_dir);

        let output_path = Path::new(output_dir);
        let mod_file = output_path.join("mod.rs");
        println!("cargo:rerun-if-changed={}", mod_file.display());

        let mut hasher = Sha256::new();
        hasher.update(env!("CARGO_PKG_VERSION").as_bytes());

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

            // Output warnings from file even when skipping build
            let warn_file = output_path.join("automodel.warn");
            if warn_file.exists() {
                if let Ok(warn_content) = fs::read_to_string(&warn_file) {
                    for warning in warn_content.lines() {
                        if !warning.is_empty() {
                            println!("cargo:warning={}", warning);
                        }
                    }
                }
            }

            return Ok(());
        }

        let database_url = database_url_cb().map_err(|e| {
            println!("cargo:error={}", e);
            std::io::Error::new(std::io::ErrorKind::NotConnected, e)
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
        use std::time::Duration;

        let output_path = Path::new(output_dir);
        let modules = self.get_modules();

        // Create output directory
        fs::create_dir_all(output_path)?;

        Self::cleanup_unused_files(output_path, &modules)?;

        // Parse connection string and configure timeouts
        let mut config: tokio_postgres::Config = database_url.parse()?;
        config.connect_timeout(Duration::from_secs(10));

        // Connect with NoTls - users should add ?sslmode=disable to their connection string
        // For TLS support, the dependency on postgres-native-tls or tokio-postgres-rustls would be needed
        let (client, connection) = config.connect(tokio_postgres::NoTls).await?;

        // Spawn the connection task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        // Temporarily disable sequential scans to force index usage in analysis
        // This helps detect queries that would benefit from indexes even with empty/small tables
        client.execute("SET enable_seqscan = false", &[]).await?;

        // Enforce queries with full path, including schemas
        client.execute("SET search_path TO ''", &[]).await?;

        // PHASE 1: Analyze all queries and collect information
        let analyzed_queries = self.analyze_all_queries(&client).await?;

        // Collect all warnings
        let mut all_warnings = Vec::new();

        // PHASE 2: Generate code from analyzed queries (no DB access)
        for module in &modules {
            let (module_code, module_warnings) =
                crate::codegen::generate_code_for_module(&analyzed_queries, module)?;
            let module_file = output_path.join(format!("{}.rs", module));
            fs::write(&module_file, &module_code)?;

            // Output warnings for this module
            for warning in &module_warnings {
                println!("cargo:warning={}", warning);
            }
            all_warnings.extend(module_warnings);
        }

        // Create the main mod.rs file
        let mod_file = output_path.join("mod.rs");
        let mod_content = generate_root_module(&modules, source_hash);
        fs::write(&mod_file, &mod_content)?;

        // Write all warnings to automodel.warn file only if there are warnings
        let warn_file = output_path.join("automodel.warn");
        if !all_warnings.is_empty() {
            let warn_content = all_warnings.join("\n");
            fs::write(&warn_file, &warn_content)?;
        } else {
            // Delete the file if it exists from a previous run with warnings
            let _ = fs::remove_file(&warn_file);
        }

        Ok(())
    }

    /// PHASE 1: Analyze all queries and extract complete information
    /// This phase interacts with the database to collect all needed information
    async fn analyze_all_queries(
        &self,
        client: &tokio_postgres::Client,
    ) -> Result<Vec<QueryDefinitionRuntime>> {
        use futures::stream::{self, StreamExt};

        // Process queries in parallel batches of 40
        let analyzed_queries: Vec<QueryDefinitionRuntime> = stream::iter(&self.queries)
            .map(|query| async move {
                println!("cargo:info=Analyzing query '{}'", query.name);

                // Extract type information (input/output types, parsed SQL)
                let type_info =
                    extract_query_types(client, &query.sql, query.types.as_ref()).await?;

                // Analyze query with EXPLAIN to detect mutation and optionally get performance data
                // EXPLAIN fails on mutations (INSERT/UPDATE/DELETE), so we use that to detect them
                // This also pre-computes EXPLAIN params during the analysis phase
                let analysis_result = Self::analyze_query_with_explain(client, query).await?;

                let analyzed_query = QueryDefinitionRuntime::new(
                    query.clone(),
                    type_info,
                    analysis_result.is_mutation,
                    analysis_result.constraints,
                    analysis_result.performance_analysis,
                    analysis_result.explain_params,
                );

                Ok::<_, anyhow::Error>(analyzed_query)
            })
            .buffered(40) // Process up to 40 queries in parallel while preserving order
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

        Ok(analyzed_queries)
    }

    /// Analyze query with EXPLAIN to detect mutations and optionally collect performance data
    /// - First checks SQL keywords to quickly identify obvious mutations
    /// - For potential read-only queries: runs EXPLAIN to verify and optionally collect performance
    /// - If EXPLAIN fails on what looks like a SELECT: treat as mutation (edge case)
    async fn analyze_query_with_explain(
        client: &tokio_postgres::Client,
        query: &QueryDefinition,
    ) -> Result<QueryAnalysisResult> {
        // Quick keyword-based detection first
        let sql_upper = query.sql.to_uppercase();
        let sql_trimmed = sql_upper.trim();

        let mutation_keywords = [
            "INSERT", "UPDATE", "DELETE", "TRUNCATE", "DROP", "CREATE", "ALTER",
        ];

        let is_obvious_mutation = mutation_keywords.iter().any(|kw| {
            sql_trimmed.starts_with(kw)
                || (sql_trimmed.starts_with("WITH") && sql_upper.contains(&format!(") {}", kw)))
        });

        if is_obvious_mutation {
            // This is clearly a mutation - extract constraints
            // Use first variant (base query) for constraint extraction
            let (converted_sql, _param_names, _label) = &query.sql_variants[0];
            let constraints = match client.prepare(converted_sql).await {
                Ok(statement) => {
                    match extract_constraints_from_statement(client, &statement, &query.sql).await {
                        Ok(constraints) => constraints,
                        Err(_e) => {
                            // Silently skip constraint extraction errors
                            Vec::new()
                        }
                    }
                }
                Err(e) => {
                    println!(
                        "cargo:info=Failed to prepare statement for constraint extraction for query '{}': {}",
                        query.name, e
                    );
                    Vec::new()
                }
            };

            return Ok(QueryAnalysisResult {
                is_mutation: true,
                performance_analysis: None,
                constraints,
                explain_params: Vec::new(),
            });
        }

        // Pre-compute EXPLAIN parameters for all variants
        let mut explain_params = Vec::new();
        for (converted_sql, param_names, _variant_label) in &query.sql_variants {
            if param_names.is_empty() {
                explain_params.push(None);
            } else {
                match client.prepare(converted_sql).await {
                    Ok(statement) => {
                        let param_types = statement.params();
                        match Self::prepare_explain_params_for_variant(
                            client,
                            converted_sql,
                            param_types,
                        )
                        .await
                        {
                            Ok(params) => explain_params.push(Some(params)),
                            Err(_) => explain_params.push(None),
                        }
                    }
                    Err(_) => explain_params.push(None),
                }
            }
        }

        // Looks like a SELECT or read-only query - verify with EXPLAIN
        let explain_result = if query.ensure_indexes {
            // Run full performance analysis (which includes EXPLAIN)
            Self::analyze_query_performance(client, query, &explain_params).await
        } else {
            // Just run a simple EXPLAIN to verify it's read-only
            Self::detect_mutation_via_explain(client, query, &explain_params).await
        };

        match explain_result {
            Ok(perf_analysis) => {
                // EXPLAIN succeeded - confirmed as read-only query
                let performance = if query.ensure_indexes {
                    Some(perf_analysis)
                } else {
                    None
                };
                Ok(QueryAnalysisResult {
                    is_mutation: false,
                    performance_analysis: performance,
                    constraints: Vec::new(),
                    explain_params,
                })
            }
            Err(_) => {
                // EXPLAIN failed on what looked like a SELECT - treat as mutation (edge case)
                // Warning will be collected in performance analysis
                Ok(QueryAnalysisResult {
                    is_mutation: true,
                    performance_analysis: None,
                    constraints: Vec::new(),
                    explain_params,
                })
            }
        }
    }

    /// Detect if query is a mutation by attempting EXPLAIN (lightweight version)
    /// Returns PerformanceAnalysis with minimal data if EXPLAIN succeeds, otherwise returns error
    async fn detect_mutation_via_explain(
        client: &tokio_postgres::Client,
        query: &QueryDefinition,
        explain_params: &[Option<ExplainParams>],
    ) -> Result<PerformanceAnalysis> {
        // Use first variant (base query) for detection
        let (_converted_sql, param_names, _label) = &query.sql_variants[0];

        // Try EXPLAIN with pre-computed parameters
        let explain_result = if !param_names.is_empty() {
            if let Some(params) = &explain_params[0] {
                if params.special_params.is_empty() {
                    // No special params, use dummy params for all parameters
                    let (converted_sql, _param_names, _label) = &query.sql_variants[0];
                    match client.prepare(converted_sql).await {
                        Ok(statement) => {
                            let param_types = statement.params();
                            let (dummy_params, _) =
                                crate::types_extractor::create_dummy_params(client, param_types)
                                    .await?;
                            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                                dummy_params.iter().map(|p| p.as_ref()).collect();
                            client.query(params.explain_sql.as_str(), &param_refs).await
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    // Has special params - some are inlined, others need dummy values
                    // Prepare to get param types for non-special params
                    let (converted_sql, _param_names, _label) = &query.sql_variants[0];
                    match client.prepare(converted_sql).await {
                        Ok(statement) => {
                            let param_types = statement.params();
                            let (all_dummy_params, _) =
                                crate::types_extractor::create_dummy_params(client, param_types)
                                    .await?;

                            // Filter to only non-special params
                            let mut non_special_dummy_params = Vec::new();
                            for (i, dummy_param) in all_dummy_params.iter().enumerate() {
                                if !params.special_params.contains(&i) {
                                    non_special_dummy_params.push(dummy_param.as_ref());
                                }
                            }

                            client
                                .query(params.explain_sql.as_str(), &non_special_dummy_params)
                                .await
                        }
                        Err(e) => Err(e),
                    }
                }
            } else {
                // Pre-computation failed - cannot run EXPLAIN
                return Err(anyhow::anyhow!(
                    "Statement preparation failed, cannot run EXPLAIN"
                ));
            }
        } else {
            // No parameters, execute directly
            let (converted_sql, _, _) = &query.sql_variants[0];
            let explain_sql = format!("EXPLAIN (FORMAT TEXT, ANALYZE false) {}", converted_sql);
            client.query(&explain_sql, &[]).await
        };

        match explain_result {
            Ok(_) => {
                // EXPLAIN succeeded, so it's a read-only query
                Ok(PerformanceAnalysis {
                    query_name: query.name.clone(),
                    has_sequential_scan: false,
                    sequential_scan_tables: Vec::new(),
                    warnings: Vec::new(),
                    query_plan: None,
                })
            }
            Err(e) => {
                // EXPLAIN failed, likely a mutation query
                Err(anyhow::anyhow!("EXPLAIN failed (likely mutation): {}", e))
            }
        }
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

    /// Prepare EXPLAIN parameters for a single query variant (done once during Phase 1)
    /// Returns ExplainParams to be stored and reused
    async fn prepare_explain_params_for_variant(
        client: &tokio_postgres::Client,
        converted_sql: &str,
        param_types: &[tokio_postgres::types::Type],
    ) -> Result<ExplainParams> {
        let (_dummy_params, special_params) =
            crate::types_extractor::create_dummy_params(client, param_types).await?;

        // Build the EXPLAIN query with special param replacements
        let explain_sql = if special_params.is_empty() {
            format!("EXPLAIN (FORMAT TEXT, ANALYZE false) {}", converted_sql)
        } else {
            // Replace special parameters with casted values and renumber remaining params
            let mut modified_sql = converted_sql.to_string();

            // Replace special parameters from highest index to lowest (to avoid renumbering issues)
            for (param_idx, type_name, value) in special_params.iter().rev() {
                let param_placeholder = format!("${}", param_idx + 1);
                // Don't quote numeric values
                let casted_value = if type_name == "numeric" {
                    format!("{}::{}", value, type_name)
                } else {
                    format!("'{}'::{}", value, type_name)
                };
                modified_sql = modified_sql.replace(&param_placeholder, &casted_value);
            }

            // Renumber remaining parameters
            // Build a mapping of non-special parameter indices
            let mut param_mapping = Vec::new();
            for i in 0..param_types.len() {
                if !special_params.iter().any(|(idx, _, _)| *idx == i) {
                    param_mapping.push(i);
                }
            }

            // Renumber from highest to lowest to avoid conflicts
            for (new_num, &original_idx) in param_mapping.iter().enumerate().rev() {
                let old_placeholder = format!("${}", original_idx + 1);
                let new_placeholder = format!("${}", new_num + 1);
                if old_placeholder != new_placeholder {
                    // Use a temporary placeholder to avoid conflicts
                    let temp_placeholder = format!("__PARAM_{}__", new_num + 1);
                    modified_sql = modified_sql.replace(&old_placeholder, &temp_placeholder);
                }
            }

            // Replace temporary placeholders with final ones
            for (new_num, _) in param_mapping.iter().enumerate() {
                let temp_placeholder = format!("__PARAM_{}__", new_num + 1);
                let final_placeholder = format!("${}", new_num + 1);
                modified_sql = modified_sql.replace(&temp_placeholder, &final_placeholder);
            }

            format!("EXPLAIN (FORMAT TEXT, ANALYZE false) {}", modified_sql)
        };

        Ok(ExplainParams {
            explain_sql,
            special_params: special_params
                .into_iter()
                .map(|(param_index, _, _)| param_index)
                .collect(),
        })
    }
    /// Analyze query performance using EXPLAIN (full analysis with query plan)
    async fn analyze_query_performance(
        client: &tokio_postgres::Client,
        query: &QueryDefinition,
        explain_params: &[Option<ExplainParams>],
    ) -> Result<PerformanceAnalysis> {
        let mut has_sequential_scan = false;
        let mut sequential_scan_tables = Vec::new();
        let mut warnings = Vec::new();
        let mut full_query_plan = String::new();

        // Analyze each variant from pre-processed sql_variants
        for (i, (converted_sql, param_names, variant_label)) in
            query.sql_variants.iter().enumerate()
        {
            let variant_name = format!("{} ({})", query.name, variant_label);

            let (variant_has_seq_scan, variant_tables, variant_warnings, variant_plan) =
                Self::analyze_single_query(
                    client,
                    converted_sql,
                    param_names,
                    &variant_name,
                    explain_params.get(i).and_then(|p| p.as_ref()),
                )
                .await?;

            if variant_has_seq_scan {
                has_sequential_scan = true;
                sequential_scan_tables.extend(variant_tables);
            }
            warnings.extend(variant_warnings);

            // Append variant plan to full plan
            if i > 0 {
                full_query_plan.push_str("\n\n");
            }
            if query.sql_variants.len() > 1 {
                full_query_plan.push_str(&format!("=== {} ===\n", variant_name));
            }
            full_query_plan.push_str(&variant_plan);
        }

        Ok(PerformanceAnalysis {
            query_name: query.name.clone(),
            has_sequential_scan,
            sequential_scan_tables,
            warnings,
            query_plan: if full_query_plan.is_empty() {
                None
            } else {
                Some(full_query_plan)
            },
        })
    }

    /// Analyze a single SQL query variant
    /// sql: already converted to positional parameters ($1, $2, etc.)
    /// param_names: list of parameter names in order
    /// explain_params: pre-computed EXPLAIN SQL and special params
    async fn analyze_single_query(
        client: &tokio_postgres::Client,
        sql: &str,
        param_names: &[String],
        query_name: &str,
        explain_params: Option<&ExplainParams>,
    ) -> Result<(bool, Vec<String>, Vec<String>, String)> {
        let mut has_sequential_scan = false;
        let mut sequential_scan_tables = Vec::new();
        let mut warnings = Vec::new();
        let mut query_plan_lines = Vec::new();

        // Execute EXPLAIN query with appropriate parameters
        let query_result = if !param_names.is_empty() {
            if let Some(params) = explain_params {
                if params.special_params.is_empty() {
                    // No special params, use dummy params
                    match client.prepare(sql).await {
                        Ok(statement) => {
                            let param_types = statement.params();
                            let (dummy_params, _) =
                                crate::types_extractor::create_dummy_params(client, param_types)
                                    .await?;
                            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                                dummy_params.iter().map(|p| p.as_ref()).collect();
                            client.query(params.explain_sql.as_str(), &param_refs).await
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!(
                                "Failed to prepare statement for analysis: {}",
                                e
                            ));
                        }
                    }
                } else {
                    // Has special params - some are inlined, others need dummy values
                    match client.prepare(sql).await {
                        Ok(statement) => {
                            let param_types = statement.params();
                            let (all_dummy_params, _) =
                                crate::types_extractor::create_dummy_params(client, param_types)
                                    .await?;

                            // Filter to only non-special params
                            let mut non_special_dummy_params = Vec::new();
                            for (i, dummy_param) in all_dummy_params.iter().enumerate() {
                                if !params.special_params.contains(&i) {
                                    non_special_dummy_params.push(dummy_param.as_ref());
                                }
                            }

                            client
                                .query(params.explain_sql.as_str(), &non_special_dummy_params)
                                .await
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!(
                                "Failed to prepare statement for analysis: {}",
                                e
                            ));
                        }
                    }
                }
            } else {
                // Pre-computation failed, try to prepare on-the-fly
                match client.prepare(sql).await {
                    Ok(statement) => {
                        let param_types = statement.params();
                        let (dummy_params, special_params) =
                            crate::types_extractor::create_dummy_params(client, param_types)
                                .await?;

                        if special_params.is_empty() {
                            // No special params, use dummy params directly
                            let explain_sql =
                                format!("EXPLAIN (FORMAT TEXT, ANALYZE false) {}", sql);
                            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                                dummy_params.iter().map(|p| p.as_ref()).collect();
                            client.query(&explain_sql, &param_refs).await
                        } else {
                            // Has special params, inline them
                            let mut modified_sql = sql.to_string();
                            for (param_idx, type_name, value) in special_params.iter().rev() {
                                let param_placeholder = format!("${}", param_idx + 1);
                                let casted_value = if type_name == "numeric" {
                                    format!("{}::{}", value, type_name)
                                } else {
                                    format!("'{}'::{}", value, type_name)
                                };
                                modified_sql =
                                    modified_sql.replace(&param_placeholder, &casted_value);
                            }
                            let explain_sql =
                                format!("EXPLAIN (FORMAT TEXT, ANALYZE false) {}", modified_sql);
                            client.query(&explain_sql, &[]).await
                        }
                    }
                    Err(_) => {
                        return Ok((
                            false,
                            Vec::new(),
                            vec![format!("Query '{}' had EXPLAIN failed", query_name)],
                            String::new(),
                        ));
                    }
                }
            }
        } else {
            // No parameters, execute directly
            let explain_sql = format!("EXPLAIN (FORMAT TEXT, ANALYZE false) {}", sql);
            client.query(&explain_sql, &[]).await
        };

        let Ok(rows) = query_result else {
            let warning = format!("Query '{}' had EXPLAIN failed", query_name);
            return Ok((false, Vec::new(), vec![warning], String::new()));
        };

        // PostgreSQL returns EXPLAIN as text lines
        for row in rows {
            let plan_line: String = row.get(0);
            query_plan_lines.push(plan_line.clone());

            // Check for sequential scans
            if plan_line.contains("Seq Scan") {
                has_sequential_scan = true;

                // Extract table name from the plan line
                // Format is usually "Seq Scan on table_name"
                if let Some(on_pos) = plan_line.find(" on ") {
                    let after_on = &plan_line[on_pos + 4..];
                    let table_name = after_on.split_whitespace().next().unwrap_or("unknown");

                    sequential_scan_tables.push(table_name.to_string());

                    let warning = format!(
                        "Query '{}' performs sequential scan on table '{}'",
                        query_name, table_name
                    );
                    warnings.push(warning);
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

                        let warning = format!(
                            "Query '{}' uses filtering on table '{}' - verify appropriate indexes exist",
                            query_name, table_name
                        );
                        warnings.push(warning);
                    }
                }
            }
        }

        let query_plan = query_plan_lines.join("\n");
        Ok((
            has_sequential_scan,
            sequential_scan_tables,
            warnings,
            query_plan,
        ))
    }
}
