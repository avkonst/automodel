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

/// Generate Rust function code for a SQL query without enum definitions
/// (assumes enums are already defined elsewhere in the module)
pub fn generate_function_code_without_enums(
    query: &QueryDefinition,
    type_info: &QueryTypeInfo,
) -> Result<String> {
    let mut code = String::new();

    // Generate result struct if needed (but no enums)
    if let Some(struct_def) = generate_result_struct(&query.name, &type_info.output_types) {
        code.push_str(&struct_def);
        code.push('\n');
    }

    // Generate function documentation
    let sql_lines: Vec<&str> = query.sql.lines().collect();
    if let Some(description) = &query.description {
        code.push_str(&format!("/// {}\n", description));
    }

    if sql_lines.len() == 1 {
        code.push_str(&format!("/// Generated from SQL: {}\n", sql_lines[0]));
    } else {
        code.push_str("/// Generated from SQL:\n");
        for line in sql_lines {
            code.push_str(&format!("/// {}\n", line.trim()));
        }
    }

    // Generate function signature
    // Extract clean parameter names directly from the SQL for function signature
    let original_param_names = parse_parameter_names_from_sql(&query.sql);
    let clean_param_names: Vec<String> = original_param_names
        .iter()
        .map(|name| {
            if name.ends_with('?') {
                name.trim_end_matches('?').to_string()
            } else {
                name.clone()
            }
        })
        .collect();

    let input_params = generate_input_params_with_names(&type_info.input_types, &clean_param_names);
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

    let return_type = match query.expect {
        ExpectedResult::ExactlyOne => {
            format!("Result<{}, tokio_postgres::Error>", base_return_type)
        }
        ExpectedResult::PossibleOne => {
            format!(
                "Result<Option<{}>, tokio_postgres::Error>",
                base_return_type
            )
        }
        ExpectedResult::AtLeastOne | ExpectedResult::Multiple => {
            format!("Result<Vec<{}>, tokio_postgres::Error>", base_return_type)
        }
    };

    code.push_str(&format!(
        "pub async fn {}({}) -> {} {{\n",
        query.name, params_str, return_type
    ));

    // Generate function body
    let function_body = generate_function_body(query, type_info, &base_return_type)?;
    code.push_str(&function_body);

    code.push_str("}\n");

    Ok(code)
}

/// Generate struct creation code for multi-column results

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
            // Use the meaningful parameter names, mapping each SQL parameter to its function parameter
            // Don't deduplicate here - each SQL positional parameter needs its value
            param_names
                .iter()
                .map(|name| {
                    // Strip the ? suffix for optional parameters to get the function parameter name
                    let clean_name = if name.ends_with('?') {
                        name.trim_end_matches('?').to_string()
                    } else {
                        name.clone()
                    };
                    format!("&{}", clean_name)
                })
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
            let inner_type = &output_col.rust_type.rust_type;

            if output_col.rust_type.is_nullable {
                // For nullable types, extract as Option<JsonWrapper<T>>
                format!(
                    "row.get::<_, Option<JsonWrapper<{}>>>(0).map(|opt| opt.map(|wrapper| wrapper.into_inner())).flatten()",
                    inner_type
                )
            } else {
                format!("row.get::<_, JsonWrapper<{}>>(0).into_inner()", inner_type)
            }
        } else {
            // For non-JSON wrapper types, extract based on nullability
            let extraction_type = if output_col.rust_type.is_nullable {
                format!("Option<{}>", output_col.rust_type.rust_type)
            } else {
                output_col.rust_type.rust_type.clone()
            };
            format!("row.get::<_, {}>(0)", extraction_type)
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
            let inner_type = &col.rust_type.rust_type;

            if col.rust_type.is_nullable {
                creation.push_str(&format!(
                    "        {}: row.get::<_, Option<JsonWrapper<{}>>>({}).map(|opt| opt.map(|wrapper| wrapper.into_inner())).flatten(),\n",
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
            let extraction_type = if col.rust_type.is_nullable {
                format!("Option<{}>", col.rust_type.rust_type)
            } else {
                col.rust_type.rust_type.clone()
            };
            creation.push_str(&format!(
                "        {}: row.get::<_, {}>({}),\n",
                to_snake_case(&col.name),
                extraction_type,
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
