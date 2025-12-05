use crate::{
    types_extractor::{OutputColumn, RustType},
    utils::{to_pascal_case, to_snake_case},
};

/// Generate function parameter list with custom parameter names
pub fn generate_input_params_with_names(
    input_types: &[RustType],
    param_names: &[String],
) -> String {
    if input_types.is_empty() {
        return String::new();
    }

    // Build a map of unique parameter names to their types
    let mut unique_params: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut param_order: Vec<String> = Vec::new();

    for (i, rust_type) in input_types.iter().enumerate() {
        let default_name = format!("param_{}", i + 1);
        let raw_param_name = param_names.get(i).unwrap_or(&default_name);

        // Strip the ? suffix for optional parameters when generating function parameter names
        let clean_param_name = if raw_param_name.ends_with('?') {
            raw_param_name.trim_end_matches('?').to_string()
        } else {
            raw_param_name.clone()
        };

        // Only add if we haven't seen this parameter name before
        if !unique_params.contains_key(&clean_param_name) {
            let final_type = if rust_type.is_nullable || rust_type.is_optional {
                format!("Option<{}>", rust_type.rust_type)
            } else {
                rust_type.rust_type.clone()
            };
            unique_params.insert(clean_param_name.clone(), final_type);
            param_order.push(clean_param_name);
        }
    }

    // Generate the parameter list in the order we first encountered each parameter
    param_order
        .iter()
        .map(|param_name| {
            let param_type = unique_params.get(param_name).unwrap();
            format!("{}: {}", param_name, param_type)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Generate function parameter for multiunzip pattern
/// Returns a single parameter that is a Vec of structs containing all the input types
/// For example: items: Vec<InsertUsersBatchRecord>
pub fn generate_multiunzip_param(query_name: &str, param_name: &str) -> String {
    let struct_name = format!("{}Record", to_pascal_case(query_name));
    format!("{}: Vec<{}>", param_name, struct_name)
}

/// Generate return type for single column results or empty results
pub fn generate_return_type(output_column: Option<&OutputColumn>) -> String {
    match output_column {
        None => "()".to_string(),
        Some(col) => {
            if col.rust_type.is_nullable {
                format!("Option<{}>", col.rust_type.rust_type)
            } else {
                col.rust_type.rust_type.clone()
            }
        }
    }
}

/// Generate Rust enum definition from enum type info
pub fn generate_enum_definition(
    enum_variants: &[String],
    enum_name: &str,
    pg_type_name: &str,
) -> String {
    let mut enum_def = format!(
        "#[derive(Debug, Clone, PartialEq, Eq)]\npub enum {} {{\n",
        enum_name
    );

    for variant in enum_variants {
        let variant_name = to_pascal_case(variant);
        enum_def.push_str(&format!("    {},\n", variant_name));
    }

    enum_def.push_str("}\n\n");

    // Add FromStr implementation for converting from database strings
    enum_def.push_str(&format!(
        r#"impl std::str::FromStr for {} {{
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {{
        match s {{
"#,
        enum_name
    ));

    for variant in enum_variants {
        let variant_name = to_pascal_case(variant);
        enum_def.push_str(&format!(
            "            \"{}\" => Ok({}::{}),\n",
            variant, enum_name, variant_name
        ));
    }

    enum_def.push_str(&format!(
        r#"            _ => Err(format!("Invalid {} variant: {{}}", s)),
        }}
    }}
}}

"#,
        enum_name
    ));

    // Add Display implementation for converting to database strings
    enum_def.push_str(&format!(
        r#"impl std::fmt::Display for {} {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
        let s = match self {{
"#,
        enum_name
    ));

    for variant in enum_variants {
        let variant_name = to_pascal_case(variant);
        enum_def.push_str(&format!(
            "            {}::{} => \"{}\",\n",
            enum_name, variant_name, variant
        ));
    }

    enum_def.push_str(&format!(
        r#"        }};
        write!(f, "{{}}", s)
    }}
}}

"#
    ));

    // Add SQLx Type implementation for enum
    enum_def.push_str(&format!(
        r#"impl sqlx::Type<sqlx::Postgres> for {} {{
    fn type_info() -> sqlx::postgres::PgTypeInfo {{
        sqlx::postgres::PgTypeInfo::with_name("{}")
    }}
}}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for {} {{
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {{
        let s = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        s.parse().map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)) as Box<dyn std::error::Error + Send + Sync + 'static>)
    }}
}}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for {} {{
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync + 'static>> {{
        <&str as sqlx::Encode<sqlx::Postgres>>::encode(&self.to_string(), buf)
    }}
}}

"#,
        enum_name, pg_type_name, enum_name, enum_name
    ));

    enum_def
}

/// Generate a result struct with a custom struct name
pub fn generate_result_struct_with_name(
    struct_name: &str,
    output_types: &[OutputColumn],
) -> Option<String> {
    if output_types.len() <= 1 {
        return None;
    }

    let mut struct_def = format!("#[derive(Debug, Clone)]\npub struct {} {{\n", struct_name);

    for col in output_types {
        let field_type = if col.rust_type.is_nullable {
            format!("Option<{}>", col.rust_type.rust_type)
        } else {
            col.rust_type.rust_type.clone()
        };
        struct_def.push_str(&format!(
            "    pub {}: {},\n",
            to_snake_case(&col.name),
            field_type
        ));
    }

    struct_def.push_str("}\n");
    Some(struct_def)
}

/// Generate an input struct for multiunzip pattern
/// Creates a struct with fields matching the parameter names and types
pub fn generate_multiunzip_input_struct(
    query_name: &str,
    param_names: &[String],
    input_types: &[RustType],
) -> Option<String> {
    if input_types.is_empty() {
        return None;
    }

    let struct_name = format!("{}Record", to_pascal_case(query_name));
    let mut struct_def = format!("#[derive(Debug, Clone)]\npub struct {} {{\n", struct_name);

    for (i, param_name) in param_names.iter().enumerate() {
        if let Some(rust_type) = input_types.get(i) {
            // Extract base type from Vec<T> for array parameters
            let base_type =
                if rust_type.rust_type.starts_with("Vec<") && rust_type.rust_type.ends_with('>') {
                    &rust_type.rust_type[4..rust_type.rust_type.len() - 1]
                } else {
                    &rust_type.rust_type
                };

            let field_type = if rust_type.is_nullable || rust_type.is_optional {
                format!("Option<{}>", base_type)
            } else {
                base_type.to_string()
            };

            struct_def.push_str(&format!(
                "    pub {}: {},\n",
                to_snake_case(param_name),
                field_type
            ));
        }
    }

    struct_def.push_str("}\n");
    Some(struct_def)
}

/// Generate struct for conditions_type pattern
/// This creates a struct with ONLY the conditional parameters (those ending with '?')
/// preserving their nullable types to support setting values to NULL (e.g., age: Some(40) -> None)
pub fn generate_conditional_diff_struct(
    struct_name: &str,
    param_names: &[String],
    input_types: &[RustType],
) -> Option<String> {
    if input_types.is_empty() {
        return None;
    }

    let mut code = String::new();

    code.push_str("#[derive(Debug, Clone, PartialEq)]\n");
    code.push_str(&format!("pub struct {} {{\n", struct_name));

    // Build a map of unique parameter names to their types
    // Only include conditional parameters (those with '?')
    let mut unique_params: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut param_order: Vec<String> = Vec::new();

    for (i, rust_type) in input_types.iter().enumerate() {
        let default_name = format!("param_{}", i + 1);
        let raw_param_name = param_names.get(i).unwrap_or(&default_name);

        // Only process conditional parameters (those ending with '?')
        if raw_param_name.ends_with('?') {
            // Strip the ? suffix
            let clean_param_name = raw_param_name.trim_end_matches('?').to_string();

            // Only add if we haven't seen this parameter name before
            if !unique_params.contains_key(&clean_param_name) {
                // For conditions_type, preserve nullable types to support setting values to NULL
                let final_type = if rust_type.is_nullable {
                    format!("Option<{}>", rust_type.rust_type)
                } else {
                    rust_type.rust_type.clone()
                };
                unique_params.insert(clean_param_name.clone(), final_type);
                param_order.push(clean_param_name);
            }
        }
    }

    // If no conditional parameters found, don't generate the struct
    if param_order.is_empty() {
        return None;
    }

    // Generate struct fields
    for param_name in &param_order {
        let param_type = unique_params.get(param_name).unwrap();
        code.push_str(&format!("    pub {}: {},\n", param_name, param_type));
    }

    code.push_str("}\n");

    Some(code)
}

/// Generate function parameters for conditions_type pattern
/// Returns: "old: &QueryNameParams, new: &QueryNameParams, non_conditional_params..."
pub fn generate_conditional_diff_params(
    query_name: &str,
    param_names: &[String],
    input_types: &[RustType],
    struct_name_override: Option<&str>,
) -> String {
    let struct_name = if let Some(override_name) = struct_name_override {
        override_name.to_string()
    } else {
        format!("{}Params", to_pascal_case(query_name))
    };

    // Separate conditional and non-conditional parameters
    let mut non_conditional_params = Vec::new();

    for (i, param_name) in param_names.iter().enumerate() {
        // Only include non-conditional parameters (those without '?')
        if !param_name.ends_with('?') {
            if let Some(rust_type) = input_types.get(i) {
                let final_type = if rust_type.is_nullable {
                    format!("Option<{}>", rust_type.rust_type)
                } else {
                    rust_type.rust_type.clone()
                };
                non_conditional_params.push(format!("{}: {}", param_name, final_type));
            }
        }
    }

    // Build parameter string - old and new structs, then non-conditional params
    let mut params = vec![
        format!("old: &{}", struct_name),
        format!("new: &{}", struct_name),
    ];

    params.extend(non_conditional_params);

    params.join(", ")
}

/// Generate a struct for structured parameters pattern
/// Returns: "pub struct QueryNameParams { pub param1: Type1, pub param2: Type2, ... }"
pub fn generate_structured_params_struct(
    query_name: &str,
    param_names: &[String],
    input_types: &[RustType],
) -> Option<String> {
    if input_types.is_empty() {
        return None;
    }

    let struct_name = format!("{}Params", to_pascal_case(query_name));
    let mut code = String::new();

    code.push_str("#[derive(Debug, Clone)]\n");
    code.push_str(&format!("pub struct {} {{\n", struct_name));

    // Build a map of unique parameter names to their types
    let mut unique_params: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut param_order: Vec<String> = Vec::new();

    for (i, rust_type) in input_types.iter().enumerate() {
        let default_name = format!("param_{}", i + 1);
        let param_name = param_names.get(i).unwrap_or(&default_name);

        // Clean parameter name (remove '?' if present for conditional params)
        let clean_param_name = param_name.trim_end_matches('?').to_string();

        // Only add if we haven't seen this parameter name before
        if !unique_params.contains_key(&clean_param_name) {
            // Use the type as-is (including Option wrapper if nullable)
            let final_type = if rust_type.is_nullable {
                format!("Option<{}>", rust_type.rust_type)
            } else {
                rust_type.rust_type.clone()
            };
            unique_params.insert(clean_param_name.clone(), final_type);
            param_order.push(clean_param_name);
        }
    }

    // Generate struct fields
    for param_name in &param_order {
        let param_type = unique_params.get(param_name).unwrap();
        code.push_str(&format!("    pub {}: {},\n", param_name, param_type));
    }

    code.push_str("}\n");

    Some(code)
}

/// Generate function parameters for parameters_type pattern
/// Returns: "params: &QueryNameParams" or "params: &OverrideName" if override is provided
pub fn generate_structured_params_signature(
    query_name: &str,
    struct_name_override: Option<&str>,
) -> String {
    let struct_name = if let Some(override_name) = struct_name_override {
        override_name.to_string()
    } else {
        format!("{}Params", to_pascal_case(query_name))
    };
    format!("params: &{}", struct_name)
}
