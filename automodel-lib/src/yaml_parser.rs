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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_rust_identifier() {
        assert!(is_valid_rust_identifier("valid_name"));
        assert!(is_valid_rust_identifier("_private"));
        assert!(is_valid_rust_identifier("camelCase"));
        assert!(is_valid_rust_identifier("snake_case"));
        assert!(is_valid_rust_identifier("PascalCase"));

        assert!(!is_valid_rust_identifier("123invalid"));
        assert!(!is_valid_rust_identifier("invalid-name"));
        assert!(!is_valid_rust_identifier("invalid.name"));
        assert!(!is_valid_rust_identifier("fn"));
        assert!(!is_valid_rust_identifier("let"));
        assert!(!is_valid_rust_identifier(""));
    }

    #[test]
    fn test_parse_yaml_string() {
        let yaml_content = r#"
queries:
  - name: get_user
    sql: "SELECT id, name, email FROM users WHERE id = $1"
    description: "Get a user by ID"
  - name: list_users
    sql: "SELECT id, name, email FROM users ORDER BY name"
    description: "List all users"
metadata:
  version: "1.0"
  description: "User management queries"
"#;

        let config = parse_yaml_string(yaml_content).unwrap();
        assert_eq!(config.queries.len(), 2);
        assert_eq!(config.queries[0].name, "get_user");
        assert_eq!(config.queries[1].name, "list_users");
    }

    #[test]
    fn test_module_validation() {
        // Valid module names
        let valid_yaml = r#"
queries:
  - name: get_user
    sql: "SELECT id FROM users"
    module: "users"
  - name: get_admin
    sql: "SELECT id FROM admins"
    module: "admin_module"
  - name: get_data
    sql: "SELECT * FROM data"
    module: "_private"
"#;
        let config = parse_yaml_string(valid_yaml).unwrap();
        assert!(validate_query_names(&config.queries).is_ok());

        // Invalid module names
        let invalid_cases = vec![
            ("123invalid", "starts with number"),
            ("invalid-name", "contains hyphen"),
            ("invalid.name", "contains dot"),
            ("fn", "reserved keyword"),
            ("mod", "reserved keyword"),
            ("", "empty name"),
        ];

        for (invalid_module, description) in invalid_cases {
            let invalid_yaml = format!(
                r#"
queries:
  - name: test_query
    sql: "SELECT 1"
    module: "{}"
"#,
                invalid_module
            );
            let config = parse_yaml_string(&invalid_yaml).unwrap();
            let result = validate_query_names(&config.queries);
            assert!(
                result.is_err(),
                "Expected error for {}: {}",
                description,
                invalid_module
            );
        }
    }
}
