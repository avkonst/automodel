use crate::config::{Config, QueryDefinition};
use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;

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

/// Parse a YAML file and return the full configuration including queries and type mappings
pub async fn parse_yaml_file<P: AsRef<Path>>(path: P) -> Result<Config> {
    let content = fs::read_to_string(&path)
        .await
        .with_context(|| format!("Failed to read YAML file: {}", path.as_ref().display()))?;

    let config = parse_yaml_string(&content)?;

    // Validate query names during parsing
    validate_query_names(&config.queries)?;

    Ok(config)
}

/// Parse a YAML string and return the full configuration including queries and type mappings
pub fn parse_yaml_string(content: &str) -> Result<Config> {
    let config: Config =
        serde_yaml::from_str(content).with_context(|| "Failed to parse YAML content")?;

    Ok(config)
}

/// Validate that query names are valid Rust function names and module names are valid
pub fn validate_query_names(queries: &[QueryDefinition]) -> Result<()> {
    for query in queries {
        // Validate query name
        if !is_valid_rust_identifier(&query.name) {
            anyhow::bail!(
                "Query name '{}' is not a valid Rust function name. Use only alphanumeric characters and underscores, and start with a letter or underscore.",
                query.name
            );
        }

        // Validate module name if specified
        if let Some(module_name) = &query.module {
            if let Err(error) = validate_module_name(module_name) {
                anyhow::bail!("Invalid module name in query '{}': {}", query.name, error);
            }
        }
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
