use crate::config::QueryDefinition;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;

/// Public function to scan SQL files from a queries directory
pub async fn scan_sql_files_from_path(queries_dir: &Path) -> Result<Vec<QueryDefinition>> {
    scan_sql_files(queries_dir).await
}

/// Public function to calculate hash of all SQL files in a queries directory
pub fn calculate_queries_dir_hash(queries_dir: &Path) -> Result<u64, std::io::Error> {
    use sha2::{Digest, Sha256};
    use std::fs;

    let mut hasher = Sha256::new();

    if queries_dir.exists() && queries_dir.is_dir() {
        // Collect all SQL files and sort them for deterministic hashing
        let mut sql_files = Vec::new();
        for module_entry in fs::read_dir(queries_dir)? {
            let module_entry = module_entry?;
            let module_path = module_entry.path();

            if module_path.is_dir() {
                for sql_entry in fs::read_dir(&module_path)? {
                    let sql_entry = sql_entry?;
                    let sql_path = sql_entry.path();

                    if sql_path.extension().and_then(|e| e.to_str()) == Some("sql") {
                        sql_files.push(sql_path);
                    }
                }
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

    Ok(hash_u64)
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
/// UPDATE users SET profile = ${profile} WHERE id = ${user_id}
/// ```
async fn parse_sql_file(path: &Path, module: &str, name: &str) -> Result<QueryDefinition> {
    let content = fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read SQL file: {}", path.display()))?;

    parse_sql_string(&content, module, name)
}

/// Parse SQL string with embedded YAML metadata
fn parse_sql_string(content: &str, module: &str, name: &str) -> Result<QueryDefinition> {
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
    #[derive(serde::Deserialize)]
    struct QueryMetadata {
        description: Option<String>,
        expect: Option<crate::config::ExpectedResult>,
        types: Option<std::collections::HashMap<String, String>>,
        telemetry: Option<crate::config::QueryTelemetryConfig>,
        ensure_indexes: Option<bool>,
        multiunzip: Option<bool>,
        conditions_type: Option<crate::config::ConditionsType>,
        parameters_type: Option<crate::config::ParametersType>,
        return_type: Option<String>,
        error_type: Option<String>,
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

    Ok(QueryDefinition {
        name: name.to_string(),
        sql,
        description: metadata.description,
        module: module.to_string(),
        expect: metadata.expect.unwrap_or_default(),
        types: metadata.types,
        telemetry: metadata.telemetry,
        ensure_indexes: metadata.ensure_indexes,
        multiunzip: metadata.multiunzip,
        conditions_type: metadata.conditions_type,
        parameters_type: metadata.parameters_type,
        return_type: metadata.return_type,
        error_type: metadata.error_type,
    })
}

/// Scan for SQL files in a queries directory and load them as QueryDefinitions
/// Directory structure: queries/{module}/{query_name}.sql
async fn scan_sql_files(queries_dir: &Path) -> Result<Vec<QueryDefinition>> {
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

        let query_def = parse_sql_file(&sql_path, &module_name, &query_name).await?;
        queries.push(query_def);
    }

    Ok(queries)
}
