use crate::query_config::{QueryConfig, QueryDefinition};
use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;

/// Parse a YAML file containing SQL query definitions
pub async fn parse_yaml_file<P: AsRef<Path>>(path: P) -> Result<Vec<QueryDefinition>> {
    let content = fs::read_to_string(&path)
        .await
        .with_context(|| format!("Failed to read YAML file: {}", path.as_ref().display()))?;

    parse_yaml_string(&content)
}

/// Parse a YAML string containing SQL query definitions
pub fn parse_yaml_string(content: &str) -> Result<Vec<QueryDefinition>> {
    let config: QueryConfig = serde_yaml::from_str(content)
        .with_context(|| "Failed to parse YAML content")?;

    Ok(config.queries)
}

/// Validate that query names are valid Rust function names
pub fn validate_query_names(queries: &[QueryDefinition]) -> Result<()> {
    for query in queries {
        if !is_valid_rust_identifier(&query.name) {
            anyhow::bail!(
                "Query name '{}' is not a valid Rust function name. Use only alphanumeric characters and underscores, and start with a letter or underscore.",
                query.name
            );
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
        "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern" | "false"
            | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move"
            | "mut" | "pub" | "ref" | "return" | "self" | "Self" | "static" | "struct" | "super"
            | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while" | "async" | "await"
            | "dyn" | "abstract" | "become" | "box" | "do" | "final" | "macro" | "override"
            | "priv" | "typeof" | "unsized" | "virtual" | "yield" | "try"
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

        let queries = parse_yaml_string(yaml_content).unwrap();
        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].name, "get_user");
        assert_eq!(queries[1].name, "list_users");
    }
}
