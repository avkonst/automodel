use crate::query_definition::QueryDefinition;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;

/// Generate SQL query variants for analysis by handling conditional syntax
/// Returns list of (sql, variant_label) tuples
fn generate_query_variants(sql: &str) -> Vec<(String, String)> {
    let mut variants = Vec::new();

    // First variant: remove all conditional blocks #[...]
    let base_query = remove_conditional_blocks(sql);
    if !base_query.trim().is_empty() {
        variants.push((base_query, "base".to_string()));
    }

    // Additional variants: include each conditional block separately
    let conditional_variants = extract_conditional_variants(sql);
    for (i, variant_sql) in conditional_variants.into_iter().enumerate() {
        variants.push((variant_sql, format!("variant {}", i + 1)));
    }

    variants
}

/// Remove all conditional blocks #[...] from SQL
fn remove_conditional_blocks(sql: &str) -> String {
    let mut result = sql.to_string();

    // Remove #[...] blocks using simple string replacement
    while let Some(start) = result.find("#[") {
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

    while let Some(start) = sql[pos..].find("#[") {
        let start_pos = pos + start;
        if let Some(end) = sql[start_pos..].find("]") {
            let end_pos = start_pos + end + 1;
            let conditional_content = &sql[start_pos + 2..end_pos - 1]; // Remove #[ and ]

            // Create variant with this conditional block included
            let mut variant = sql.to_string();
            variant.replace_range(start_pos..end_pos, conditional_content);

            // Remove any remaining conditional blocks from this variant
            variant = remove_conditional_blocks(&variant);

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

/// Validates that a module name is a valid Rust identifier
fn validate_module_name(module_name: &str) -> Result<(), String> {
    if module_name.is_empty() {
        return Err("Module name cannot be empty".to_string());
    }

    // Reuse existing validation logic
    if !is_valid_rust_identifier(module_name) {
        // Check specific error cases to provide better error messages
        let first_char = module_name.chars().next().unwrap();
        if !first_char.is_ascii_alphabetic() && first_char != '_' {
            return Err(format!(
                "Module name '{}' must start with a letter or underscore",
                module_name
            ));
        }

        // Check for invalid characters
        for ch in module_name.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '_' {
                return Err(format!(
                    "Module name '{}' contains invalid character '{}'. Only letters, numbers, and underscores are allowed",
                    module_name, ch
                ));
            }
        }

        // If we get here, it must be a reserved keyword
        if is_rust_keyword(module_name) {
            return Err(format!(
                "Module name '{}' is a reserved Rust keyword and cannot be used",
                module_name
            ));
        }

        // Fallback error (should not happen with current logic)
        return Err(format!(
            "Module name '{}' is not a valid Rust identifier",
            module_name
        ));
    }

    Ok(())
}

/// Check if a string is a valid Rust identifier
fn is_valid_rust_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();

    // First character must be a letter or underscore
    if !first.is_alphabetic() && first != '_' {
        return false;
    }

    // Remaining characters must be alphanumeric or underscore
    for c in chars {
        if !c.is_alphanumeric() && c != '_' {
            return false;
        }
    }

    // Check if it's a Rust keyword
    !is_rust_keyword(name)
}

/// Check if a string is a Rust keyword
fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
    )
}

/// Parse SQL file with embedded YAML metadata in comments
/// Expected format:
/// ```sql
/// -- @automodel
/// --    description: Update user profile
/// --    expect: exactly_one
/// --    types:
/// --      profile: "crate::models::UserProfile"
/// -- @end
///
/// UPDATE users SET profile = #{profile} WHERE id = #{user_id}
/// ```
async fn parse_sql_file(
    path: &Path,
    module: &str,
    name: &str,
    defaults: crate::DefaultsConfig,
) -> Result<QueryDefinition> {
    let content = fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read SQL file: {}", path.display()))?;

    let mut in_metadata = false;
    let mut yaml_lines = Vec::new();
    let mut sql_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "-- @automodel" {
            in_metadata = true;
            continue;
        }

        if trimmed == "-- @end" {
            in_metadata = false;
            continue;
        }

        if in_metadata {
            // Remove leading "-- " or "--" from the line, but preserve indentation after that
            if let Some(yaml_content) = trimmed.strip_prefix("--") {
                // If there's a space after --, remove it, but keep the rest of the spacing
                let yaml_content = if yaml_content.starts_with(' ') {
                    &yaml_content[1..]
                } else {
                    yaml_content
                };
                yaml_lines.push(yaml_content);
            }
        } else if !trimmed.starts_with("--")
            || trimmed.starts_with("-- ") && !trimmed.trim_start_matches("-- ").trim().is_empty()
        {
            // Include SQL lines (skip empty comment lines outside metadata)
            sql_lines.push(line);
        }
    }

    // Parse the YAML metadata
    let yaml_str = yaml_lines.join("\n");

    // Create a temporary QueryDefinition with minimal info
    #[derive(Default, serde::Deserialize)]
    struct TelemetryMetadata {
        #[serde(default)]
        pub level: Option<crate::query_definition::TelemetryLevel>,
        #[serde(default)]
        pub include_params: Option<Vec<String>>,
        #[serde(default)]
        pub include_sql: Option<bool>,
    }

    // Create a temporary QueryDefinition with minimal info
    #[derive(serde::Deserialize)]
    struct QueryMetadata {
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        expect: Option<crate::query_definition::ExpectedResult>,
        #[serde(default)]
        types: Option<std::collections::HashMap<String, String>>,
        #[serde(default)]
        telemetry: TelemetryMetadata,
        #[serde(default)]
        ensure_indexes: Option<bool>,
        #[serde(default)]
        multiunzip: Option<bool>,
        #[serde(default)]
        conditions_type: Option<crate::query_definition::ConditionsType>,
        #[serde(default)]
        parameters_type: Option<crate::query_definition::ParametersType>,
        #[serde(default)]
        return_type: Option<String>,
        #[serde(default)]
        error_type: Option<String>,
        #[serde(default)]
        conditions_type_derives: Vec<String>,
        #[serde(default)]
        parameters_type_derives: Vec<String>,
        #[serde(default)]
        return_type_derives: Vec<String>,
        #[serde(default)]
        error_type_derives: Vec<String>,
    }

    let metadata: QueryMetadata = if yaml_str.trim().is_empty() {
        // No metadata provided, use defaults
        serde_yaml::from_str("{}").unwrap()
    } else {
        serde_yaml::from_str(&yaml_str).with_context(|| {
            format!(
                "Failed to parse YAML metadata in SQL file for query '{}'",
                name
            )
        })?
    };

    // Combine SQL lines and trim
    let sql = sql_lines.join("\n").trim().to_string();

    if sql.is_empty() {
        anyhow::bail!("SQL file contains no SQL query for '{}'", name);
    }

    // Generate SQL variants and convert to positional parameters at parse time
    let sql_variants_raw = generate_query_variants(&sql);
    let sql_variants: Vec<(String, Vec<String>, String)> = sql_variants_raw
        .into_iter()
        .map(|(variant_sql, variant_label)| {
            let (converted_sql, param_names) =
                crate::types_extractor::convert_named_params_to_positional(&variant_sql);
            (converted_sql, param_names, variant_label)
        })
        .collect();

    Ok(QueryDefinition {
        name: name.to_string(),
        sql,
        sql_variants,
        description: metadata.description,
        module: module.to_string(),
        expect: metadata.expect.unwrap_or_default(),
        types: metadata.types,
        telemetry: crate::query_definition::QueryTelemetryConfig {
            level: metadata.telemetry.level.unwrap_or(defaults.telemetry.level),
            include_params: metadata.telemetry.include_params,
            include_sql: metadata
                .telemetry
                .include_sql
                .unwrap_or(defaults.telemetry.include_sql),
        },
        ensure_indexes: metadata.ensure_indexes.unwrap_or(defaults.ensure_indexes),
        multiunzip: metadata.multiunzip.unwrap_or(false),
        conditions_type: metadata.conditions_type.unwrap_or_default(),
        parameters_type: metadata.parameters_type.unwrap_or_default(),
        return_type: metadata.return_type,
        error_type: metadata.error_type,
        conditions_type_derives: metadata.conditions_type_derives,
        parameters_type_derives: metadata.parameters_type_derives,
        return_type_derives: metadata.return_type_derives,
        error_type_derives: metadata.error_type_derives,
    })
}

/// Scan for SQL files in a queries directory and load them as QueryDefinitions
/// Directory structure: queries/{module}/{query_name}.sql
pub async fn scan_sql_files(
    queries_dir: &Path,
    defaults: crate::DefaultsConfig,
) -> Result<Vec<QueryDefinition>> {
    let mut queries = Vec::new();

    // Check if queries directory exists
    if !queries_dir.exists() {
        return Ok(queries);
    }

    // Collect all SQL file paths first, then sort them
    let mut all_sql_files = Vec::new();

    // Read all module directories
    let mut module_dirs = fs::read_dir(queries_dir).await.with_context(|| {
        format!(
            "Failed to read queries directory: {}",
            queries_dir.display()
        )
    })?;

    while let Some(module_entry) = module_dirs.next_entry().await? {
        let module_path = module_entry.path();

        if !module_path.is_dir() {
            continue;
        }

        let module_name = module_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid module directory name"))?
            .to_string();

        // Validate module name
        validate_module_name(&module_name).map_err(|e| {
            anyhow::anyhow!("Invalid module directory name '{}': {}", module_name, e)
        })?;

        // Read all SQL files in the module directory
        let mut sql_files_in_module = fs::read_dir(&module_path).await.with_context(|| {
            format!("Failed to read module directory: {}", module_path.display())
        })?;

        while let Some(sql_entry) = sql_files_in_module.next_entry().await? {
            let sql_path = sql_entry.path();

            if sql_path.extension().and_then(|e| e.to_str()) != Some("sql") {
                continue;
            }

            all_sql_files.push((sql_path, module_name.clone()));
        }
    }

    // Sort SQL files by their full path to ensure consistent ordering
    all_sql_files.sort_by(|a, b| a.0.cmp(&b.0));

    // Now process the sorted files
    for (sql_path, module_name) in all_sql_files {
        let file_stem = sql_path
            .file_stem()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid SQL file name"))?;

        // Strip numeric prefix if present (e.g., "01_query_name" -> "query_name")
        let query_name = if let Some(underscore_pos) = file_stem.find('_') {
            let (prefix, name) = file_stem.split_at(underscore_pos);
            // Check if prefix is all digits
            if prefix.chars().all(|c| c.is_ascii_digit()) {
                name.trim_start_matches('_').to_string()
            } else {
                file_stem.to_string()
            }
        } else {
            file_stem.to_string()
        };

        // Validate query name
        if !is_valid_rust_identifier(&query_name) {
            anyhow::bail!(
                "SQL file name '{}' is not a valid Rust function name. Use only alphanumeric characters and underscores, and start with a letter or underscore.",
                query_name
            );
        }

        let query_def =
            parse_sql_file(&sql_path, &module_name, &query_name, defaults.clone()).await?;
        queries.push(query_def);
    }

    Ok(queries)
}
