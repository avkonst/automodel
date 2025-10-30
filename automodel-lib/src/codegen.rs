use crate::config::{ExpectedResult, QueryDefinition, TelemetryConfig, TelemetryLevel};
use crate::type_extraction::{
    convert_named_params_to_positional, generate_input_params_with_names, generate_result_struct,
    generate_return_type, parse_parameter_names_from_sql, OutputColumn, QueryTypeInfo,
};
use anyhow::Result;

/// Generate tracing::instrument attribute for a function
fn generate_tracing_attribute(
    query: &QueryDefinition,
    param_names: &[String],
    telemetry_level: &TelemetryLevel,
    global_telemetry: Option<&TelemetryConfig>,
) -> String {
    use std::collections::HashSet;

    if *telemetry_level == TelemetryLevel::None {
        return String::new();
    }

    let mut attributes = Vec::new();

    // No need for explicit span name - tracing::instrument will use the function name automatically

    // Add instrumentation level
    let level = match telemetry_level {
        TelemetryLevel::Info => "info",
        TelemetryLevel::Debug => "debug",
        TelemetryLevel::Trace => "trace",
        TelemetryLevel::None => unreachable!(),
    };
    attributes.push(format!("level = \"{}\"", level));

    // Determine parameter skipping strategy
    let mut skip_params = HashSet::new();
    skip_params.insert("executor".to_string());

    // Parameter inclusion logic (independent of telemetry level)
    if let Some(query_telemetry) = &query.telemetry {
        if let Some(include_params) = &query_telemetry.include_params {
            if include_params.is_empty() {
                // Empty include_params list means skip all parameters
                skip_params.extend(param_names.iter().cloned());
            } else {
                let included: Vec<String> = include_params
                    .iter()
                    .filter(|param| param_names.contains(param))
                    .cloned()
                    .collect();

                if !included.is_empty() {
                    // Skip parameters not in the include list
                    for param in param_names {
                        if !included.contains(param) {
                            skip_params.insert(param.clone());
                        }
                    }
                } else {
                    // No valid included parameters, skip all
                    skip_params.extend(param_names.iter().cloned());
                }
            }
        } else {
            // No include_params specified, skip all parameters
            skip_params.extend(param_names.iter().cloned());
        }
    } else {
        // No query telemetry, skip all parameters
        skip_params.extend(param_names.iter().cloned());
    }

    // Check if we should use skip_all (all params including query params are skipped)
    let total_params = param_names.len() + 1; // +1 for executor
    let should_use_skip_all = param_names.len() > 0 && skip_params.len() == total_params;

    // Generate skip attribute
    if should_use_skip_all {
        attributes.push("skip_all".to_string());
    } else if skip_params.len() > 1 {
        // More than just executor
        let mut skip_vec: Vec<_> = skip_params.into_iter().collect();
        skip_vec.sort(); // Sort for consistent output
        attributes.push(format!("skip({})", skip_vec.join(", ")));
    } else {
        attributes.push("skip(executor)".to_string());
    }

    // Determine whether to include SQL based on configuration (default false)
    let should_include_sql = if let Some(query_telemetry) = &query.telemetry {
        if let Some(include_sql) = query_telemetry.include_sql {
            include_sql
        } else {
            // Fall back to global configuration
            global_telemetry
                .map(|config| config.include_sql)
                .unwrap_or(false)
        }
    } else {
        // No query telemetry, use global configuration
        global_telemetry
            .map(|config| config.include_sql)
            .unwrap_or(false)
    };

    if should_include_sql {
        let escaped_sql = query
            .sql
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        attributes.push(format!("fields(sql = \"{}\")", escaped_sql));
    }

    format!("#[tracing::instrument({})]\n", attributes.join(", "))
}

/// Generate an indented raw string literal with proper formatting
fn generate_indented_raw_string_literal(sql: &str) -> String {
    // Find a delimiter that doesn't appear in the SQL
    let mut delimiter_count = 0;
    let delimiter = loop {
        let delimiter = "#".repeat(delimiter_count);
        let pattern = format!("\"{}\"", delimiter);
        if !sql.contains(&pattern) {
            break delimiter;
        }
        delimiter_count += 1;
    };

    // Add proper indentation to each line of SQL
    let indented_sql = sql
        .lines()
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                line.to_string() // First line doesn't need extra indentation
            } else {
                format!("        {}", line) // Subsequent lines get 8 spaces of indentation
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "        r{delimiter}\"{indented_sql}\"{delimiter}",
        delimiter = delimiter,
        indented_sql = indented_sql
    )
}

/// Determine the effective telemetry level for a query
fn determine_telemetry_level(
    query: &QueryDefinition,
    global_telemetry: Option<&TelemetryConfig>,
) -> TelemetryLevel {
    // Query-specific telemetry overrides global settings
    if let Some(query_telemetry) = &query.telemetry {
        if let Some(level) = &query_telemetry.level {
            return level.clone();
        }
    }

    // Fall back to global telemetry level
    global_telemetry
        .map(|config| config.level.clone())
        .unwrap_or(TelemetryLevel::None)
}

/// Generate Rust function code for a SQL query without enum definitions
/// (assumes enums are already defined elsewhere in the module)
pub fn generate_function_code_without_enums(
    query: &QueryDefinition,
    type_info: &QueryTypeInfo,
    global_telemetry: Option<&TelemetryConfig>,
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

    // Determine effective telemetry configuration and generate attribute
    let effective_telemetry_level = determine_telemetry_level(query, global_telemetry);
    let tracing_attribute = generate_tracing_attribute(
        query,
        &clean_param_names,
        &effective_telemetry_level,
        global_telemetry,
    );
    code.push_str(&tracing_attribute);

    let input_params = generate_input_params_with_names(&type_info.input_types, &clean_param_names);
    let base_return_type = if type_info.output_types.len() > 1 {
        format!("{}Item", to_pascal_case(&query.name))
    } else {
        generate_return_type(type_info.output_types.first())
    };

    // Generate function signature
    let params_str = if input_params.is_empty() {
        "executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>".to_string()
    } else {
        format!(
            "executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>, {}",
            input_params
        )
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
    let raw_string = generate_indented_raw_string_literal(&converted_sql);
    body.push_str(&format!(
        "    let query = sqlx::query(\n{}\n    );\n",
        raw_string
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

                let rust_type_info = &type_info.input_types[i];
                let param_type = &rust_type_info.rust_type;

                // Check if this is a custom type that needs JSON serialization
                if rust_type_info.needs_json_wrapper {
                    // For custom types, serialize to JSON before binding
                    body.push_str(&format!(
                        "    let query = query.bind(serde_json::to_value(&{}).map_err(|e| sqlx::Error::Encode(Box::new(e)))?);\n", 
                        clean_name
                    ));
                } else if param_type == "String" {
                    // Use reference for String parameters to avoid move issues
                    body.push_str(&format!("    let query = query.bind(&{});\n", clean_name));
                } else {
                    body.push_str(&format!("    let query = query.bind({});\n", clean_name));
                }
            }
        }
    }

    if type_info.output_types.is_empty() {
        // For queries that don't return data (INSERT, UPDATE, DELETE)
        body.push_str("    query.execute(executor).await?;\n");
        body.push_str("    Ok(())\n");
    } else if type_info.output_types.len() == 1 {
        // For queries that return a single column
        match query.expect {
            ExpectedResult::ExactlyOne => {
                body.push_str("    let row = query.fetch_one(executor).await?;\n");
                let value_extraction =
                    generate_sqlx_value_extraction(&type_info.output_types[0], 0);
                body.push_str(&format!("    Ok({})\n", value_extraction));
            }
            ExpectedResult::PossibleOne => {
                body.push_str("    let row = query.fetch_optional(executor).await?;\n");
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
                body.push_str("    let rows = query.fetch_all(executor).await?;\n");
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
                body.push_str("    let rows = query.fetch_all(executor).await?;\n");
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
                body.push_str("    let row = query.fetch_one(executor).await?;\n");
                body.push_str("    let result: Result<_, sqlx::Error> = (|| {\n");
                let struct_creation =
                    generate_sqlx_struct_creation(return_type, &type_info.output_types);
                body.push_str(&format!("        Ok({})\n", struct_creation));
                body.push_str("    })();\n");
                body.push_str("    result\n");
            }
            ExpectedResult::PossibleOne => {
                body.push_str("    let row = query.fetch_optional(executor).await?;\n");
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
                body.push_str("    let rows = query.fetch_all(executor).await?;\n");
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
                body.push_str("    let rows = query.fetch_all(executor).await?;\n");
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
                "row.try_get::<Option<serde_json::Value>, _>(\"{}\")?
            .map(|v| serde_json::from_value::<{}>(v)
            .map_err(|e| sqlx::Error::Decode(Box::new(e))))
            .transpose()?",
                column_name, inner_type
            )
        } else {
            format!(
                "serde_json::from_value::<{}>(
            row.try_get::<serde_json::Value, _>(\"{}\")?)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?",
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
