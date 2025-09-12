use anyhow::{Context, Result};
use std::collections::HashMap;
use tokio_postgres::types::Type as PgType;
use tokio_postgres::{NoTls, Statement};

/// Information about a SQL query's input and output types
#[derive(Debug, Clone)]
pub struct QueryTypeInfo {
    /// Input parameter types
    pub input_types: Vec<RustType>,
    /// Output column types and names
    pub output_types: Vec<OutputColumn>,
}

/// Represents a Rust type mapping from PostgreSQL types
#[derive(Debug, Clone)]
pub struct RustType {
    /// The Rust type name (e.g., "i32", "String", "Option<i64>")
    pub rust_type: String,
    /// Whether this type is nullable
    pub is_nullable: bool,
    /// The original PostgreSQL type
    pub pg_type: String,
    /// Whether this is a custom type that needs JSON wrapper
    pub needs_json_wrapper: bool,
}

/// Represents an output column with its name and type
#[derive(Debug, Clone)]
pub struct OutputColumn {
    /// Column name
    pub name: String,
    /// Rust type information
    pub rust_type: RustType,
}

/// Extract type information from a prepared SQL statement
pub async fn extract_query_types(
    database_url: &str,
    sql: &str,
    field_type_mappings: Option<&HashMap<String, String>>,
) -> Result<QueryTypeInfo> {
    // Create database connection
    let (client, connection) = tokio_postgres::connect(database_url, NoTls)
        .await
        .with_context(|| format!("Failed to connect to database: {}", database_url))?;

    // Spawn the connection in the background
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Database connection error: {}", e);
        }
    });

    // Convert named parameters to positional parameters for PostgreSQL
    let (converted_sql, _param_names) = convert_named_params_to_positional(sql);

    let statement = client.prepare(&converted_sql).await.with_context(|| {
        format!(
            "Failed to prepare statement for type extraction: {}",
            converted_sql
        )
    })?;

    // Extract types
    let input_types = extract_input_types(&statement)?;
    let output_types = extract_output_types(&client, &statement, field_type_mappings).await?;

    Ok(QueryTypeInfo {
        input_types,
        output_types,
    })
}

/// Extract input parameter types from a prepared statement
fn extract_input_types(statement: &Statement) -> Result<Vec<RustType>> {
    let params = statement.params();
    let mut input_types = Vec::new();

    for param_type in params {
        let rust_type = pg_type_to_rust_type(param_type, false)?;
        input_types.push(rust_type);
    }

    Ok(input_types)
}

/// Get nullability information for columns by querying PostgreSQL system catalogs
async fn get_column_nullability(
    client: &tokio_postgres::Client,
    columns: &[tokio_postgres::Column],
) -> Result<Vec<bool>> {
    let mut nullability = Vec::new();

    for column in columns {
        let table_oid = column.table_oid();
        let column_id = column.column_id();

        let is_nullable = if let (Some(table_oid), Some(column_id)) = (table_oid, column_id) {
            // Query pg_attribute to get the actual NOT NULL constraint
            let rows = client
                .query(
                    "SELECT attnotnull FROM pg_attribute WHERE attrelid = $1 AND attnum = $2",
                    &[&table_oid, &column_id],
                )
                .await?;

            if let Some(row) = rows.first() {
                let attnotnull: bool = row.get(0);
                !attnotnull // attnotnull=true means NOT NULL, so nullable=false
            } else {
                // Fallback: if we can't find the column info, assume nullable
                true
            }
        } else {
            // No table/column info available (computed column, function result, etc.)
            // Assume nullable for safety
            true
        };

        nullability.push(is_nullable);
    }

    Ok(nullability)
}

/// Extract output column types from a prepared statement
async fn extract_output_types(
    client: &tokio_postgres::Client,
    statement: &Statement,
    field_type_mappings: Option<&HashMap<String, String>>,
) -> Result<Vec<OutputColumn>> {
    let columns = statement.columns();
    let mut output_types = Vec::new();

    // Get nullability information for all columns
    let nullability_info = get_column_nullability(client, &columns).await?;

    for (i, column) in columns.iter().enumerate() {
        let column_name = column.name();
        let is_nullable = nullability_info.get(i).copied().unwrap_or(true); // Default to nullable if unknown
        let base_rust_type = pg_type_to_rust_type(column.type_(), is_nullable)?;

        // Check if there's a custom type mapping for this field
        // Note: Since we only have the column name here, we can't determine the exact table
        // For now, we'll check for exact column name matches in the mappings
        let rust_type = if let Some(mappings) = field_type_mappings {
            // Look for any mapping that ends with the column name
            let custom_type = mappings
                .iter()
                .find(|(key, _)| key.ends_with(&format!(".{}", column_name)))
                .map(|(_, rust_type)| rust_type.clone());

            if let Some(custom_type) = custom_type {
                RustType {
                    rust_type: if base_rust_type.is_nullable {
                        format!("Option<{}>", custom_type)
                    } else {
                        custom_type
                    },
                    is_nullable: base_rust_type.is_nullable,
                    pg_type: base_rust_type.pg_type,
                    needs_json_wrapper: true, // Custom types need JSON wrapper
                }
            } else {
                base_rust_type
            }
        } else {
            base_rust_type
        };

        output_types.push(OutputColumn {
            name: column_name.to_string(),
            rust_type,
        });
    }

    Ok(output_types)
}

/// Convert PostgreSQL type to Rust type
fn pg_type_to_rust_type(pg_type: &PgType, is_nullable: bool) -> Result<RustType> {
    let base_type = match *pg_type {
        PgType::BOOL => "bool",
        PgType::CHAR => "i8",
        PgType::INT2 => "i16",
        PgType::INT4 => "i32",
        PgType::INT8 => "i64",
        PgType::OID => "u32",
        PgType::FLOAT4 => "f32",
        PgType::FLOAT8 => "f64",
        PgType::TEXT | PgType::VARCHAR | PgType::BPCHAR => "String",
        PgType::BYTEA => "Vec<u8>",
        PgType::TIMESTAMP => "chrono::NaiveDateTime",
        PgType::TIMESTAMPTZ => "chrono::DateTime<chrono::Utc>",
        PgType::DATE => "chrono::NaiveDate",
        PgType::TIME => "chrono::NaiveTime",
        PgType::UUID => "uuid::Uuid",
        PgType::JSON | PgType::JSONB => "serde_json::Value",
        PgType::INET => "std::net::IpAddr",
        PgType::NUMERIC => "rust_decimal::Decimal",
        _ => {
            // For unknown types, use a generic approach
            return Ok(RustType {
                rust_type: format!("/* Unknown type: {} */ String", pg_type.name()),
                is_nullable,
                pg_type: pg_type.name().to_string(),
                needs_json_wrapper: false,
            });
        }
    };

    let rust_type = if is_nullable {
        format!("Option<{}>", base_type)
    } else {
        base_type.to_string()
    };

    Ok(RustType {
        rust_type,
        is_nullable,
        pg_type: pg_type.name().to_string(),
        needs_json_wrapper: false, // Standard types don't need JSON wrapper
    })
}

/// Generate function parameter list from input types
pub fn generate_input_params(input_types: &[RustType]) -> String {
    if input_types.is_empty() {
        return String::new();
    }

    input_types
        .iter()
        .enumerate()
        .map(|(i, rust_type)| format!("param_{}: {}", i + 1, rust_type.rust_type))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Generate function parameter list with custom parameter names
pub fn generate_input_params_with_names(
    input_types: &[RustType],
    param_names: &[String],
) -> String {
    if input_types.is_empty() {
        return String::new();
    }

    input_types
        .iter()
        .enumerate()
        .map(|(i, rust_type)| {
            let default_name = format!("param_{}", i + 1);
            let param_name = param_names.get(i).unwrap_or(&default_name);
            format!("{}: {}", param_name, rust_type.rust_type)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse SQL to extract meaningful parameter names from named parameters
pub fn parse_parameter_names_from_sql(sql: &str) -> Vec<String> {
    // Look for named parameters in the format ${param_name}
    let mut param_names = Vec::new();
    let mut chars = sql.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            if let Some(&'{') = chars.peek() {
                chars.next(); // consume the '{'
                let mut param_name = String::new();

                // Read until we find the closing brace
                while let Some(inner_ch) = chars.next() {
                    if inner_ch == '}' {
                        if !param_name.is_empty() {
                            param_names.push(param_name);
                        }
                        break;
                    } else {
                        param_name.push(inner_ch);
                    }
                }
            }
        }
    }

    // If no named parameters found, fall back to counting positional parameters
    if param_names.is_empty() {
        let param_count = sql.matches('$').count();
        param_names = (1..=param_count).map(|i| format!("param_{}", i)).collect();
    }

    param_names
}

/// Convert SQL with named parameters ${param} to positional parameters $1, $2, etc.
pub fn convert_named_params_to_positional(sql: &str) -> (String, Vec<String>) {
    let mut param_names = Vec::new();
    let mut result_sql = String::new();
    let mut chars = sql.chars().peekable();
    let mut param_counter = 1;

    while let Some(ch) = chars.next() {
        if ch == '$' {
            if let Some(&'{') = chars.peek() {
                chars.next(); // consume the '{'
                let mut param_name = String::new();

                // Read until we find the closing brace
                while let Some(inner_ch) = chars.next() {
                    if inner_ch == '}' {
                        if !param_name.is_empty() {
                            param_names.push(param_name);
                            result_sql.push_str(&format!("${}", param_counter));
                            param_counter += 1;
                        }
                        break;
                    } else {
                        param_name.push(inner_ch);
                    }
                }
            } else {
                // Regular $ character, just pass it through
                result_sql.push(ch);
            }
        } else {
            result_sql.push(ch);
        }
    }

    // If no named parameters were found, return original SQL
    if param_names.is_empty() {
        (sql.to_string(), Vec::new())
    } else {
        (result_sql, param_names)
    }
}

/// Generate return type from output types
pub fn generate_return_type(output_types: &[OutputColumn]) -> String {
    if output_types.is_empty() {
        return "()".to_string();
    }

    if output_types.len() == 1 {
        return output_types[0].rust_type.rust_type.clone();
    }

    // For multiple columns, generate a tuple
    let types: Vec<String> = output_types
        .iter()
        .map(|col| col.rust_type.rust_type.clone())
        .collect();

    format!("({})", types.join(", "))
}

/// Generate struct definition for query result if there are multiple columns
pub fn generate_result_struct(query_name: &str, output_types: &[OutputColumn]) -> Option<String> {
    if output_types.len() <= 1 {
        return None;
    }

    let struct_name = format!("{}Result", to_pascal_case(query_name));
    let mut struct_def = format!("#[derive(Debug, Clone)]\npub struct {} {{\n", struct_name);

    for col in output_types {
        struct_def.push_str(&format!(
            "    pub {}: {},\n",
            to_snake_case(&col.name),
            col.rust_type.rust_type
        ));
    }

    struct_def.push_str("}\n");
    Some(struct_def)
}

/// Convert string to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect()
}

/// Convert string to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    let mut prev_was_upper = false;

    while let Some(ch) = chars.next() {
        if ch.is_uppercase() {
            if !result.is_empty() && !prev_was_upper {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch.is_lowercase() {
                        result.push('_');
                    }
                }
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
            prev_was_upper = true;
        } else {
            result.push(ch);
            prev_was_upper = false;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("get_user"), "GetUser");
        assert_eq!(to_pascal_case("list_all_users"), "ListAllUsers");
        assert_eq!(to_pascal_case("simple"), "Simple");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("userId"), "user_id");
        assert_eq!(to_snake_case("firstName"), "first_name");
        assert_eq!(to_snake_case("ID"), "id"); // Fixed expectation
        assert_eq!(to_snake_case("simple"), "simple");
    }

    #[test]
    fn test_generate_input_params() {
        let types = vec![
            RustType {
                rust_type: "i32".to_string(),
                is_nullable: false,
                pg_type: "INT4".to_string(),
                needs_json_wrapper: false,
            },
            RustType {
                rust_type: "String".to_string(),
                is_nullable: false,
                pg_type: "TEXT".to_string(),
                needs_json_wrapper: false,
            },
        ];

        let params = generate_input_params(&types);
        assert_eq!(params, "param_1: i32, param_2: String");
    }

    #[test]
    fn test_generate_return_type() {
        let single_col = vec![OutputColumn {
            name: "id".to_string(),
            rust_type: RustType {
                rust_type: "i32".to_string(),
                is_nullable: false,
                pg_type: "INT4".to_string(),
                needs_json_wrapper: false,
            },
        }];

        assert_eq!(generate_return_type(&single_col), "i32");

        let multi_col = vec![
            OutputColumn {
                name: "id".to_string(),
                rust_type: RustType {
                    rust_type: "i32".to_string(),
                    is_nullable: false,
                    pg_type: "INT4".to_string(),
                    needs_json_wrapper: false,
                },
            },
            OutputColumn {
                name: "name".to_string(),
                rust_type: RustType {
                    rust_type: "String".to_string(),
                    is_nullable: false,
                    pg_type: "TEXT".to_string(),
                    needs_json_wrapper: false,
                },
            },
        ];

        assert_eq!(generate_return_type(&multi_col), "(i32, String)");
    }
}
