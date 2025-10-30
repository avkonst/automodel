use crate::config::{ExpectedResult, QueryDefinition};
use crate::type_extraction::{
    convert_named_params_to_positional, generate_input_params_with_names, generate_result_struct,
    generate_return_type, parse_parameter_names_from_sql, OutputColumn, QueryTypeInfo,
};
use anyhow::Result;

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
    if let Some(description) = &query.description {
        code.push_str(&format!("/// {}\n", description));
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
        generate_return_type(type_info.output_types.first())
    };

    // Generate function signature
    let params_str = if input_params.is_empty() {
        "pool: &sqlx::PgPool".to_string()
    } else {
        format!("pool: &sqlx::PgPool, {}", input_params)
    };

    let return_type = match query.expect {
        ExpectedResult::ExactlyOne => {
            format!("Result<{}, sqlx::Error>", base_return_type)
        }
        ExpectedResult::PossibleOne => {
            format!("Result<Option<{}>, sqlx::Error>", base_return_type)
        }
        ExpectedResult::AtLeastOne | ExpectedResult::Multiple => {
            format!("Result<Vec<{}>, sqlx::Error>", base_return_type)
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

/// Generate the function body using SQLx
fn generate_function_body(
    query: &QueryDefinition,
    type_info: &QueryTypeInfo,
    return_type: &str,
) -> Result<String> {
    let mut body = String::new();

    // Convert named parameters to positional parameters for SQLx
    let (converted_sql, param_names) = convert_named_params_to_positional(&query.sql);

    // Build the SQLx query with parameter bindings
    body.push_str(&format!(
        "    let query = sqlx::query(\"{}\");\n",
        escape_sql_string(&converted_sql)
    ));

    // Add parameter bindings using method chaining
    if !type_info.input_types.is_empty() {
        if param_names.is_empty() {
            // Fallback to generic param names
            for i in 1..=type_info.input_types.len() {
                body.push_str(&format!("    let query = query.bind(param_{});\n", i));
            }
        } else {
            // Use meaningful parameter names from SQL
            for (i, name) in param_names.iter().enumerate() {
                let clean_name = if name.ends_with('?') {
                    name.trim_end_matches('?').to_string()
                } else {
                    name.clone()
                };

                // Use reference for String parameters to avoid move issues
                let param_type = &type_info.input_types[i].rust_type;
                if param_type == "String" {
                    body.push_str(&format!("    let query = query.bind(&{});\n", clean_name));
                } else {
                    body.push_str(&format!("    let query = query.bind({});\n", clean_name));
                }
            }
        }
    }

    if type_info.output_types.is_empty() {
        // For queries that don't return data (INSERT, UPDATE, DELETE)
        body.push_str("    query.execute(pool).await?;\n");
        body.push_str("    Ok(())\n");
    } else if type_info.output_types.len() == 1 {
        // For queries that return a single column
        match query.expect {
            ExpectedResult::ExactlyOne => {
                body.push_str("    let row = query.fetch_one(pool).await?;\n");
                let value_extraction =
                    generate_sqlx_value_extraction(&type_info.output_types[0], 0);
                body.push_str(&format!("    Ok({})\n", value_extraction));
            }
            ExpectedResult::PossibleOne => {
                body.push_str("    let row = query.fetch_optional(pool).await?;\n");
                body.push_str("    match row {\n");
                body.push_str("        Some(row) => {\n");
                let value_extraction =
                    generate_sqlx_value_extraction(&type_info.output_types[0], 0);
                if type_info.output_types[0].rust_type.is_nullable {
                    body.push_str(&format!("            Ok({})\n", value_extraction));
                } else {
                    body.push_str(&format!("            Ok(Some({}))\n", value_extraction));
                }
                body.push_str("        },\n");
                body.push_str("        None => Ok(None),\n");
                body.push_str("    }\n");
            }
            ExpectedResult::AtLeastOne => {
                body.push_str("    let rows = query.fetch_all(pool).await?;\n");
                body.push_str("    if rows.is_empty() {\n");
                body.push_str("        return Err(sqlx::Error::RowNotFound);\n");
                body.push_str("    }\n");
                body.push_str(
                    "    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {\n",
                );
                let value_extraction =
                    generate_sqlx_value_extraction(&type_info.output_types[0], 0);
                body.push_str(&format!("        Ok({})\n", value_extraction));
                body.push_str("    }).collect();\n");
                body.push_str("    result\n");
            }
            ExpectedResult::Multiple => {
                body.push_str("    let rows = query.fetch_all(pool).await?;\n");
                body.push_str(
                    "    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {\n",
                );
                let value_extraction =
                    generate_sqlx_value_extraction(&type_info.output_types[0], 0);
                body.push_str(&format!("        Ok({})\n", value_extraction));
                body.push_str("    }).collect();\n");
                body.push_str("    result\n");
            }
        }
    } else {
        // For queries that return multiple columns
        match query.expect {
            ExpectedResult::ExactlyOne => {
                body.push_str("    let row = query.fetch_one(pool).await?;\n");
                body.push_str("    let result: Result<_, sqlx::Error> = (|| {\n");
                let struct_creation =
                    generate_sqlx_struct_creation(return_type, &type_info.output_types);
                body.push_str(&format!("        Ok({})\n", struct_creation));
                body.push_str("    })();\n");
                body.push_str("    result\n");
            }
            ExpectedResult::PossibleOne => {
                body.push_str("    let row = query.fetch_optional(pool).await?;\n");
                body.push_str("    match row {\n");
                body.push_str("        Some(row) => {\n");
                body.push_str("            let result: Result<_, sqlx::Error> = (|| {\n");
                let struct_creation =
                    generate_sqlx_struct_creation(return_type, &type_info.output_types);
                body.push_str(&format!("                Ok({})\n", struct_creation));
                body.push_str("            })();\n");
                body.push_str("            result.map(Some)\n");
                body.push_str("        },\n");
                body.push_str("        None => Ok(None),\n");
                body.push_str("    }\n");
            }
            ExpectedResult::AtLeastOne => {
                body.push_str("    let rows = query.fetch_all(pool).await?;\n");
                body.push_str("    if rows.is_empty() {\n");
                body.push_str("        return Err(sqlx::Error::RowNotFound);\n");
                body.push_str("    }\n");
                body.push_str(
                    "    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {\n",
                );
                let struct_creation =
                    generate_sqlx_struct_creation(return_type, &type_info.output_types);
                body.push_str(&format!("        Ok({})\n", struct_creation));
                body.push_str("    }).collect();\n");
                body.push_str("    result\n");
            }
            ExpectedResult::Multiple => {
                body.push_str("    let rows = query.fetch_all(pool).await?;\n");
                body.push_str(
                    "    let result: Result<Vec<_>, sqlx::Error> = rows.iter().map(|row| {\n",
                );
                let struct_creation =
                    generate_sqlx_struct_creation(return_type, &type_info.output_types);
                body.push_str(&format!("        Ok({})\n", struct_creation));
                body.push_str("    }).collect();\n");
                body.push_str("    result\n");
            }
        }
    }

    Ok(body)
}

/// Generate SQLx value extraction for a single column
fn generate_sqlx_value_extraction(output_col: &OutputColumn, _index: usize) -> String {
    let column_name = &output_col.name;

    if output_col.rust_type.needs_json_wrapper {
        // For custom types, we need to extract as serde_json::Value and then deserialize
        let inner_type = &output_col.rust_type.rust_type;
        if output_col.rust_type.is_nullable {
            format!(
                "row.try_get::<Option<serde_json::Value>, _>(\"{}\")?.map(|v| serde_json::from_value::<{}>(v)).transpose()?",
                column_name, inner_type
            )
        } else {
            format!(
                "serde_json::from_value::<{}>(row.try_get::<serde_json::Value, _>(\"{}\")?)?,",
                inner_type, column_name
            )
        }
    } else {
        // For standard types, extract directly
        if output_col.rust_type.is_nullable {
            format!(
                "row.try_get::<Option<{}>, _>(\"{}\")?",
                output_col.rust_type.rust_type, column_name
            )
        } else {
            format!(
                "row.try_get::<{}, _>(\"{}\")?",
                output_col.rust_type.rust_type, column_name
            )
        }
    }
}

/// Generate SQLx struct creation code for multi-column results
fn generate_sqlx_struct_creation(struct_name: &str, output_types: &[OutputColumn]) -> String {
    let mut creation = format!("{} {{\n", struct_name);

    for (i, col) in output_types.iter().enumerate() {
        let field_name = to_snake_case(&col.name);
        let value_extraction = generate_sqlx_value_extraction(col, i);
        creation.push_str(&format!("        {}: {},\n", field_name, value_extraction));
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
