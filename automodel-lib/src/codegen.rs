use crate::config::{ExpectedResult, QueryDefinition};
use crate::type_extraction::{
    convert_named_params_to_positional, generate_input_params_with_names, generate_result_struct,
    generate_return_type, parse_parameter_names_from_sql, OutputColumn, QueryTypeInfo,
};
use anyhow::Result;

/// Generate a JSON wrapper helper for custom types
pub fn generate_json_wrapper_helper() -> String {
    r#"
// JSON wrapper for custom types that implement Serialize/Deserialize
struct JsonWrapper<T>(T);

impl<T> JsonWrapper<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    fn new(value: T) -> Self {
        Self(value)
    }
    
    fn into_inner(self) -> T {
        self.0
    }
}

impl<T> FromSql<'_> for JsonWrapper<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        let json_value = serde_json::Value::from_sql(ty, raw)?;
        let value = T::deserialize(json_value)?;
        Ok(JsonWrapper(value))
    }

    fn accepts(ty: &Type) -> bool {
        matches!(*ty, Type::JSON | Type::JSONB)
    }
}

impl<T> ToSql for JsonWrapper<T>
where
    T: Serialize + std::fmt::Debug,
{
    fn to_sql(&self, ty: &Type, out: &mut bytes::BytesMut) -> Result<tokio_postgres::types::IsNull, Box<dyn Error + Sync + Send>> {
        let json_value = serde_json::to_value(&self.0)?;
        json_value.to_sql(ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        matches!(*ty, Type::JSON | Type::JSONB)
    }

    tokio_postgres::types::to_sql_checked!();
}

impl<T> std::fmt::Debug for JsonWrapper<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("JsonWrapper").field(&self.0).finish()
    }
}
"#.to_string()
}

/// Generate Rust function code for a SQL query
pub fn generate_function_code(
    query: &QueryDefinition,
    type_info: &QueryTypeInfo,
) -> Result<String> {
    let mut code = String::new();

    // Generate result struct if needed
    if let Some(struct_def) = generate_result_struct(&query.name, &type_info.output_types) {
        code.push_str(&struct_def);
        code.push('\n');
    }

    // Generate function documentation
    if let Some(description) = &query.description {
        code.push_str(&format!("/// {}\n", description));
    }

    // Handle multiline SQL comments properly
    let sql_lines: Vec<&str> = query.sql.trim().lines().collect();
    if sql_lines.len() == 1 {
        code.push_str(&format!("/// Generated from SQL: {}\n", sql_lines[0]));
    } else {
        code.push_str("/// Generated from SQL:\n");
        for line in sql_lines {
            code.push_str(&format!("/// {}\n", line.trim()));
        }
    }

    // Generate function signature
    let param_names = parse_parameter_names_from_sql(&query.sql);
    let input_params = generate_input_params_with_names(&type_info.input_types, &param_names);
    let base_return_type = if type_info.output_types.len() > 1 {
        format!("{}Result", to_pascal_case(&query.name))
    } else {
        generate_return_type(&type_info.output_types)
    };

    // Generate function signature
    let params_str = if input_params.is_empty() {
        "client: &tokio_postgres::Client".to_string()
    } else {
        format!("client: &tokio_postgres::Client, {}", input_params)
    };

    let final_return_type = if type_info.output_types.is_empty() {
        "()".to_string()
    } else {
        // Adjust return type based on expect field
        match query.expect {
            ExpectedResult::ExactlyOne => base_return_type.clone(),
            ExpectedResult::PossibleOne => {
                // For PossibleOne, we need to wrap the non-nullable version in Option<>
                let non_nullable_type = if type_info.output_types.len() == 1 {
                    // For single column, get the inner type without Option<>
                    let rust_type = &type_info.output_types[0].rust_type.rust_type;
                    if rust_type.starts_with("Option<") && rust_type.ends_with('>') {
                        // If it's already Option<T>, for PossibleOne we just use Option<T>
                        rust_type.clone()
                    } else {
                        // If it's T, for PossibleOne we use Option<T>
                        format!("Option<{}>", rust_type)
                    }
                } else {
                    // For multi-column, use Option<StructName>
                    format!("Option<{}>", base_return_type)
                };
                non_nullable_type
            }
            ExpectedResult::AtLeastOne => format!("Vec<{}>", base_return_type),
            ExpectedResult::Multiple => format!("Vec<{}>", base_return_type),
        }
    };

    code.push_str(&format!(
        "pub async fn {}({}) -> Result<{}, tokio_postgres::Error> {{\n",
        query.name, params_str, final_return_type
    ));

    // Generate function body
    code.push_str(&generate_function_body(
        query,
        type_info,
        &base_return_type,
    )?);

    code.push_str("}\n");

    Ok(code)
}

/// Generate the function body
fn generate_function_body(
    query: &QueryDefinition,
    type_info: &QueryTypeInfo,
    return_type: &str,
) -> Result<String> {
    let mut body = String::new();

    // Convert named parameters to positional parameters for the generated SQL
    let (converted_sql, param_names) = convert_named_params_to_positional(&query.sql);

    // Prepare the statement
    body.push_str(&format!(
        "    let stmt = client.prepare(\"{}\").await?;\n",
        escape_sql_string(&converted_sql)
    ));

    // Prepare parameters - use meaningful names if available
    let param_refs = if type_info.input_types.is_empty() {
        "&[]".to_string()
    } else {
        let params: Vec<String> = if param_names.is_empty() {
            // Fallback to generic param names
            (1..=type_info.input_types.len())
                .map(|i| format!("&param_{}", i))
                .collect()
        } else {
            // Use the meaningful parameter names
            param_names
                .iter()
                .map(|name| format!("&{}", name))
                .collect()
        };
        format!("&[{}]", params.join(", "))
    };

    if type_info.output_types.is_empty() {
        // For queries that don't return data (INSERT, UPDATE, DELETE)
        body.push_str(&format!(
            "    client.execute(&stmt, {}).await?;\n",
            param_refs
        ));
        body.push_str("    Ok(())\n");
    } else if type_info.output_types.len() == 1 {
        // For queries that return a single column
        match query.expect {
            ExpectedResult::ExactlyOne => {
                body.push_str(&format!(
                    "    let row = client.query_one(&stmt, {}).await?;\n",
                    param_refs
                ));
            }
            ExpectedResult::PossibleOne => {
                body.push_str(&format!(
                    "    let rows = client.query(&stmt, {}).await?;\n",
                    param_refs
                ));
                body.push_str(
                    "    let extracted_value = if let Some(row) = rows.into_iter().next() {\n",
                );
            }
            ExpectedResult::AtLeastOne => {
                body.push_str(&format!(
                    "    let rows = client.query(&stmt, {}).await?;\n",
                    param_refs
                ));
                body.push_str("    if rows.is_empty() {\n");
                body.push_str("        // Simulate the same error that query_one would produce\n");
                body.push_str(
                    "        let _ = client.query_one(\"SELECT 1 WHERE FALSE\", &[]).await?;\n",
                );
                body.push_str("    }\n");
                body.push_str("    let result = rows.into_iter().map(|row| {\n");
            }
            ExpectedResult::Multiple => {
                body.push_str(&format!(
                    "    let rows = client.query(&stmt, {}).await?;\n",
                    param_refs
                ));
                body.push_str("    let result = rows.into_iter().map(|row| {\n");
            }
        }

        let output_col = &type_info.output_types[0];
        let value_extraction = if output_col.rust_type.needs_json_wrapper {
            // Use JSON wrapper for custom types
            let inner_type = if output_col.rust_type.is_nullable {
                // Extract the inner type from Option<CustomType>
                let rust_type = &output_col.rust_type.rust_type;
                if rust_type.starts_with("Option<") && rust_type.ends_with('>') {
                    &rust_type[7..rust_type.len() - 1]
                } else {
                    rust_type
                }
            } else {
                &output_col.rust_type.rust_type
            };

            if output_col.rust_type.is_nullable {
                // For nullable types, just extract normally
                format!(
                    "row.get::<_, Option<JsonWrapper<{}>>>(0).map(|wrapper| wrapper.into_inner())",
                    inner_type
                )
            } else {
                format!("row.get::<_, JsonWrapper<{}>>(0).into_inner()", inner_type)
            }
        } else {
            // For non-JSON wrapper types, just extract normally
            format!("row.get::<_, {}>(0)", output_col.rust_type.rust_type)
        };

        match query.expect {
            ExpectedResult::ExactlyOne => {
                body.push_str(&format!("    Ok({})\n", value_extraction));
            }
            ExpectedResult::PossibleOne => {
                if output_col.rust_type.is_nullable {
                    // For nullable columns, return the value directly (it's already Option<T>)
                    body.push_str(&format!("        {}\n", value_extraction));
                } else {
                    // For non-nullable columns, wrap in Some()
                    body.push_str(&format!("        Some({})\n", value_extraction));
                }
                body.push_str("    } else {\n");
                body.push_str("        None\n");
                body.push_str("    };\n");
                body.push_str("    Ok(extracted_value)\n");
            }
            ExpectedResult::AtLeastOne | ExpectedResult::Multiple => {
                body.push_str(&format!("        {}\n", value_extraction));
                body.push_str("    }).collect();\n");
                body.push_str("    Ok(result)\n");
            }
        }
    } else {
        // For queries that return multiple columns
        match query.expect {
            ExpectedResult::ExactlyOne => {
                body.push_str(&format!(
                    "    let row = client.query_one(&stmt, {}).await?;\n",
                    param_refs
                ));
            }
            ExpectedResult::PossibleOne => {
                body.push_str(&format!(
                    "    let rows = client.query(&stmt, {}).await?;\n",
                    param_refs
                ));
                body.push_str(
                    "    let extracted_value = if let Some(row) = rows.into_iter().next() {\n",
                );
            }
            ExpectedResult::AtLeastOne => {
                body.push_str(&format!(
                    "    let rows = client.query(&stmt, {}).await?;\n",
                    param_refs
                ));
                body.push_str("    if rows.is_empty() {\n");
                body.push_str("        // Simulate the same error that query_one would produce\n");
                body.push_str(
                    "        let _ = client.query_one(\"SELECT 1 WHERE FALSE\", &[]).await?;\n",
                );
                body.push_str("    }\n");
                body.push_str("    let result = rows.into_iter().map(|row| {\n");
            }
            ExpectedResult::Multiple => {
                body.push_str(&format!(
                    "    let rows = client.query(&stmt, {}).await?;\n",
                    param_refs
                ));
                body.push_str("    let result = rows.into_iter().map(|row| {\n");
            }
        }

        let struct_creation = generate_struct_creation(return_type, &type_info.output_types);

        match query.expect {
            ExpectedResult::ExactlyOne => {
                body.push_str(&format!("    Ok({})\n", struct_creation));
            }
            ExpectedResult::PossibleOne => {
                body.push_str(&format!("        Some({})\n", struct_creation));
                body.push_str("    } else {\n");
                body.push_str("        None\n");
                body.push_str("    };\n");
                body.push_str("    Ok(extracted_value)\n");
            }
            ExpectedResult::AtLeastOne | ExpectedResult::Multiple => {
                body.push_str(&format!("        {}\n", struct_creation));
                body.push_str("    }).collect();\n");
                body.push_str("    Ok(result)\n");
            }
        }
    }

    Ok(body)
}

/// Generate struct creation code for multi-column results
fn generate_struct_creation(struct_name: &str, output_types: &[OutputColumn]) -> String {
    let mut creation = format!("{} {{\n", struct_name);

    for (i, col) in output_types.iter().enumerate() {
        if col.rust_type.needs_json_wrapper {
            // Use JSON wrapper for custom types
            let inner_type = if col.rust_type.is_nullable {
                // Extract the inner type from Option<CustomType>
                let rust_type = &col.rust_type.rust_type;
                if rust_type.starts_with("Option<") && rust_type.ends_with('>') {
                    &rust_type[7..rust_type.len() - 1]
                } else {
                    rust_type
                }
            } else {
                &col.rust_type.rust_type
            };

            if col.rust_type.is_nullable {
                creation.push_str(&format!(
                    "        {}: row.get::<_, Option<JsonWrapper<{}>>>({}).map(|wrapper| wrapper.into_inner()),\n",
                    to_snake_case(&col.name),
                    inner_type,
                    i
                ));
            } else {
                creation.push_str(&format!(
                    "        {}: row.get::<_, JsonWrapper<{}>>({})).into_inner(),\n",
                    to_snake_case(&col.name),
                    inner_type,
                    i
                ));
            }
        } else {
            creation.push_str(&format!(
                "        {}: row.get::<_, {}>({}),\n",
                to_snake_case(&col.name),
                col.rust_type.rust_type,
                i
            ));
        }
    }

    creation.push_str("    }");
    creation
}

/// Escape SQL string for inclusion in Rust code
fn escape_sql_string(sql: &str) -> String {
    sql.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
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

    while let Some(ch) = chars.next() {
        if ch.is_uppercase() && !result.is_empty() {
            if let Some(&next_ch) = chars.peek() {
                if next_ch.is_lowercase() {
                    result.push('_');
                }
            }
        }
        result.push(ch.to_lowercase().next().unwrap_or(ch));
    }

    result
}

/// Generate a complete module with all functions
pub fn generate_module_code(
    queries: &[QueryDefinition],
    type_infos: &[QueryTypeInfo],
) -> Result<String> {
    let mut module_code = String::new();

    // Add module header
    module_code.push_str("// This file was auto-generated by automodel\n");
    module_code.push_str("// Do not edit manually\n\n");
    module_code.push_str("use tokio_postgres::{Client, Error};\n");
    module_code.push_str("use std::result::Result;\n\n");

    // Generate all functions
    for (query, type_info) in queries.iter().zip(type_infos.iter()) {
        let function_code = generate_function_code(query, type_info)?;
        module_code.push_str(&function_code);
        module_code.push('\n');
    }

    Ok(module_code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type_extraction::{OutputColumn, RustType};

    #[test]
    fn test_escape_sql_string() {
        assert_eq!(
            escape_sql_string(r#"SELECT "name" FROM users"#),
            r#"SELECT \"name\" FROM users"#
        );
        assert_eq!(
            escape_sql_string("SELECT *\nFROM users"),
            "SELECT *\\nFROM users"
        );
        assert_eq!(
            escape_sql_string("SELECT * FROM users\r\n"),
            "SELECT * FROM users\\r\\n"
        );
    }

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
        assert_eq!(to_snake_case("simple"), "simple");
    }

    #[test]
    fn test_generate_function_code() {
        let query = QueryDefinition {
            name: "get_user".to_string(),
            sql: "SELECT id, name FROM users WHERE id = $1".to_string(),
            description: Some("Get user by ID".to_string()),
            module: None,
            expect: ExpectedResult::ExactlyOne,
        };

        let type_info = QueryTypeInfo {
            input_types: vec![RustType {
                rust_type: "i32".to_string(),
                is_nullable: false,
                pg_type: "INT4".to_string(),
                needs_json_wrapper: false,
            }],
            output_types: vec![
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
            ],
        };

        let code = generate_function_code(&query, &type_info).unwrap();
        assert!(code.contains("pub async fn get_user"));
        assert!(code.contains("param_1: i32"));
        assert!(code.contains("GetUserResult"));
    }
}
