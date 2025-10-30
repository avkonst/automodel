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
    /// Parsed SQL with conditional blocks (if any)
    pub parsed_sql: Option<ParsedSql>,
}

/// Represents a Rust type mapping from PostgreSQL types
#[derive(Debug, Clone)]
pub struct RustType {
    /// The Rust type name (e.g., "i32", "String", "Option<i64>")
    pub rust_type: String,
    /// Whether this type is nullable
    pub is_nullable: bool,
    /// Whether this is a custom type that needs JSON wrapper
    pub needs_json_wrapper: bool,
    /// If this is an enum type, contains the enum variants
    pub enum_variants: Option<Vec<String>>,
    /// If this is an enum type, contains the original PostgreSQL type name
    pub pg_type_name: Option<String>,
}

/// Information about a PostgreSQL enum type
#[derive(Debug, Clone)]
pub struct EnumTypeInfo {
    /// The name of the enum type
    pub type_name: String,
    /// The variants of the enum
    pub variants: Vec<String>,
}

/// Represents an output column with its name and type
#[derive(Debug, Clone)]
pub struct OutputColumn {
    /// Column name
    pub name: String,
    /// Rust type information
    pub rust_type: RustType,
}

/// Represents a conditional block in a SQL query
#[derive(Debug, Clone)]
pub struct ConditionalBlock {
    /// The SQL content inside the conditional block
    pub sql_content: String,
    /// Parameters referenced within this conditional block
    pub parameters: Vec<String>,
}

/// Parsed SQL with conditional blocks separated
#[derive(Debug, Clone)]
pub struct ParsedSql {
    /// Base SQL with conditional blocks removed and placeholders inserted
    pub base_sql: String,
    /// List of conditional blocks found in the SQL
    pub conditional_blocks: Vec<ConditionalBlock>,
    /// All parameter names found in the SQL (including those in conditional blocks)
    pub all_parameters: Vec<String>,
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

    // Parse SQL to handle conditional blocks
    let parsed_sql = parse_sql_with_conditionals(sql);

    // For validation, create SQL with all conditional blocks included
    let full_sql = reconstruct_full_sql(&parsed_sql);

    // Convert named parameters to positional parameters for PostgreSQL
    let (converted_sql, param_names) = convert_named_params_to_positional(&full_sql);

    let statement = client.prepare(&converted_sql).await.with_context(|| {
        format!(
            "Failed to prepare statement for type extraction: {}",
            converted_sql
        )
    })?;

    // Extract types
    let input_types =
        extract_input_types(&client, &statement, &param_names, field_type_mappings).await?;
    let output_types = extract_output_types(&client, &statement, field_type_mappings).await?;

    let has_conditionals = !parsed_sql.conditional_blocks.is_empty();

    Ok(QueryTypeInfo {
        input_types,
        output_types,
        parsed_sql: if has_conditionals {
            Some(parsed_sql)
        } else {
            None
        },
    })
}

/// Extract input parameter types from a prepared statement
async fn extract_input_types(
    client: &tokio_postgres::Client,
    statement: &Statement,
    param_names: &[String],
    field_type_mappings: Option<&HashMap<String, String>>,
) -> Result<Vec<RustType>> {
    let params = statement.params();
    let mut input_types = Vec::new();

    for (i, param_type) in params.iter().enumerate() {
        // Check if this parameter has the optional suffix ?
        let param_name = param_names.get(i).map(|s| s.as_str()).unwrap_or("");
        let is_optional_param = param_name.ends_with('?');

        // Get clean parameter name (without ? suffix)
        let clean_param_name = if is_optional_param {
            &param_name[..param_name.len() - 1]
        } else {
            param_name
        };

        let mut rust_type = pg_type_to_rust_type(client, param_type, false).await?; // Always get base type

        // Check if there's a custom type mapping for this parameter
        if let Some(mappings) = field_type_mappings {
            // Look for any mapping that ends with the parameter name
            let custom_type = mappings
                .iter()
                .find(|(key, _)| {
                    // Match patterns like "table.field" or just "field"
                    key.ends_with(&format!(".{}", clean_param_name)) || key == &clean_param_name
                })
                .map(|(_, rust_type_name)| rust_type_name.clone());

            if let Some(custom_type) = custom_type {
                rust_type = RustType {
                    rust_type: custom_type,
                    is_nullable: is_optional_param,
                    needs_json_wrapper: true, // Custom input parameters need JSON serialization
                    enum_variants: None,
                    pg_type_name: None,
                };
            } else if is_optional_param {
                // If it's an optional parameter but no custom type, mark as nullable
                rust_type.is_nullable = true;
            }
        } else if is_optional_param {
            // If no mappings and it's optional parameter, mark as nullable
            rust_type.is_nullable = true;
        }

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

/// Get enum type information from PostgreSQL system catalogs
pub async fn get_enum_type_info(
    client: &tokio_postgres::Client,
    type_oid: u32,
) -> Result<Option<EnumTypeInfo>> {
    // Query pg_enum to get enum values for a specific type
    let rows = client
        .query(
            r#"
            SELECT t.typname, array_agg(e.enumlabel ORDER BY e.enumsortorder) as enum_values
            FROM pg_type t
            JOIN pg_enum e ON t.oid = e.enumtypid
            WHERE t.oid = $1
            GROUP BY t.typname
            "#,
            &[&type_oid],
        )
        .await?;

    if let Some(row) = rows.first() {
        let type_name: String = row.get(0);
        let enum_values: Vec<String> = row.get(1);

        Ok(Some(EnumTypeInfo {
            type_name,
            variants: enum_values,
        }))
    } else {
        Ok(None)
    }
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
        let base_rust_type = pg_type_to_rust_type(client, column.type_(), is_nullable).await?;

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
                    rust_type: custom_type, // Store base type without Option<>
                    is_nullable: base_rust_type.is_nullable,
                    needs_json_wrapper: true, // Custom types need JSON wrapper
                    enum_variants: None,
                    pg_type_name: None,
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
async fn pg_type_to_rust_type(
    client: &tokio_postgres::Client,
    pg_type: &PgType,
    is_nullable: bool,
) -> Result<RustType> {
    // Check if this is an enum type by trying to get enum info
    if let Some(enum_info) = get_enum_type_info(client, pg_type.oid()).await? {
        let enum_name = to_pascal_case(&enum_info.type_name);

        return Ok(RustType {
            rust_type: enum_name, // Store base enum type without Option<>
            is_nullable,
            needs_json_wrapper: false,
            enum_variants: Some(enum_info.variants),
            pg_type_name: Some(enum_info.type_name),
        });
    }

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
                needs_json_wrapper: false,
                enum_variants: None,
                pg_type_name: None,
            });
        }
    };

    Ok(RustType {
        rust_type: base_type.to_string(), // Store base type without Option<>
        is_nullable,
        needs_json_wrapper: false, // Standard types don't need JSON wrapper
        enum_variants: None,
        pg_type_name: None,
    })
}

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
            let final_type = if rust_type.is_nullable {
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

/// Parse SQL to extract conditional blocks and return structured information
pub fn parse_sql_with_conditionals(sql: &str) -> ParsedSql {
    let mut result = ParsedSql {
        base_sql: String::new(),
        conditional_blocks: Vec::new(),
        all_parameters: Vec::new(),
    };

    let mut chars = sql.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            if let Some(&'[') = chars.peek() {
                // Found start of conditional block
                chars.next(); // consume '['

                let mut block_content = String::new();
                let mut bracket_count = 1; // We already consumed one '['

                // Read until we find the matching ']'
                while let Some(inner_ch) = chars.next() {
                    if inner_ch == '[' {
                        bracket_count += 1;
                        block_content.push(inner_ch);
                    } else if inner_ch == ']' {
                        bracket_count -= 1;
                        if bracket_count == 0 {
                            // Found the end of this conditional block

                            // Extract parameters from this block
                            let block_params = parse_parameter_names_from_sql(&block_content);

                            // Add conditional block
                            result.conditional_blocks.push(ConditionalBlock {
                                sql_content: block_content.clone(),
                                parameters: block_params.clone(),
                            });

                            // Add parameters to our global list
                            result.all_parameters.extend(block_params);

                            // Keep the original conditional block syntax in base SQL
                            result.base_sql.push_str(&format!("$[{}]", block_content));
                            break;
                        } else {
                            block_content.push(inner_ch);
                        }
                    } else {
                        block_content.push(inner_ch);
                    }
                }
            } else if let Some(&'{') = chars.peek() {
                // Found regular parameter ${param}
                chars.next(); // consume '{'
                let mut param_name = String::new();

                while let Some(inner_ch) = chars.next() {
                    if inner_ch == '}' {
                        if !param_name.is_empty() {
                            result.all_parameters.push(param_name.clone());
                            result.base_sql.push_str("${");
                            result.base_sql.push_str(&param_name);
                            result.base_sql.push('}');
                        }
                        break;
                    } else {
                        param_name.push(inner_ch);
                    }
                }
            } else {
                // Regular $ character
                result.base_sql.push(ch);
            }
        } else {
            result.base_sql.push(ch);
        }
    }

    result
}

/// Reconstruct full SQL with all conditional blocks included for validation
fn reconstruct_full_sql(parsed_sql: &ParsedSql) -> String {
    let mut result = parsed_sql.base_sql.clone();

    // Replace conditional blocks $[...] with their inner content
    for block in &parsed_sql.conditional_blocks {
        let conditional_block = format!("$[{}]", block.sql_content);
        result = result.replace(&conditional_block, &block.sql_content);
    }

    result
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

/// Extract all unique enum types from input and output types
pub fn extract_enum_types(
    input_types: &[RustType],
    output_types: &[OutputColumn],
) -> Vec<(String, Vec<String>, String)> {
    let mut enum_types = std::collections::HashMap::new();

    // Check input types for enums
    for input_type in input_types {
        if let Some(ref variants) = input_type.enum_variants {
            if let Some(ref pg_type_name) = input_type.pg_type_name {
                enum_types.insert(
                    input_type.rust_type.clone(),
                    (variants.clone(), pg_type_name.clone()),
                );
            }
        }
    }

    // Check output types for enums
    for output_col in output_types {
        if let Some(ref variants) = output_col.rust_type.enum_variants {
            if let Some(ref pg_type_name) = output_col.rust_type.pg_type_name {
                enum_types.insert(
                    output_col.rust_type.rust_type.clone(),
                    (variants.clone(), pg_type_name.clone()),
                );
            }
        }
    }

    enum_types
        .into_iter()
        .map(|(rust_name, (variants, pg_name))| (rust_name, variants, pg_name))
        .collect()
}

pub fn generate_result_struct(query_name: &str, output_types: &[OutputColumn]) -> Option<String> {
    if output_types.len() <= 1 {
        return None;
    }

    let struct_name = format!("{}Item", to_pascal_case(query_name));
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
