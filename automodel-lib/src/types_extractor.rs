use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tokio_postgres::types::Type as PgType;
use tokio_postgres::Statement;

use crate::utils::to_pascal_case;

// Global cache for enum type information to avoid repeated database queries
static ENUM_CACHE: OnceLock<Mutex<HashMap<u32, Option<EnumTypeInfo>>>> = OnceLock::new();

/// Constraint information extracted from database schema
#[derive(Debug, Clone)]
pub struct ConstraintInfo {
    /// Constraint name
    pub name: String,
    /// Constraint type: unique, primary_key, foreign_key, check, not_null
    #[allow(unused)]
    pub constraint_type: String,
    /// Table name
    pub table_name: String,
}

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
    /// Whether this type is optional (conditional) parameter in a query
    pub is_optional: bool,
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
    client: &tokio_postgres::Client,
    sql: &str,
    field_type_mappings: Option<&HashMap<String, String>>,
) -> Result<QueryTypeInfo> {
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

/// Extract constraint information from tables involved in a prepared statement
/// This analyzes the statement to identify affected tables and retrieves their constraints
pub async fn extract_constraints_from_statement(
    client: &tokio_postgres::Client,
    statement: &Statement,
    sql: &str,
) -> Result<Vec<ConstraintInfo>> {
    let mut constraints = Vec::new();
    let mut table_oids = HashSet::new();

    // Collect table OIDs from input parameters (for INSERT/UPDATE)
    for param in statement.params() {
        // Skip non-table types
        if let Some(oid) = get_table_oid_from_type(param) {
            table_oids.insert(oid);
        }
    }

    // Collect table OIDs from output columns (for SELECT/RETURNING)
    for column in statement.columns() {
        if let Some(oid) = column.table_oid() {
            table_oids.insert(oid);
        }
    }

    // Fallback: parse SQL to extract table names for DDL statements
    if table_oids.is_empty() {
        let extracted_tables = extract_table_names_from_sql(sql);
        for table_name in extracted_tables {
            if let Some(oid) = get_table_oid_by_name(client, &table_name).await? {
                table_oids.insert(oid);
            }
        }
    }

    // Query constraints for each table
    for table_oid in table_oids {
        let table_constraints = query_table_constraints(client, table_oid).await?;
        constraints.extend(table_constraints);
    }

    Ok(constraints)
}

/// Get table OID from a PostgreSQL type (if it's a composite type)
fn get_table_oid_from_type(_pg_type: &PgType) -> Option<u32> {
    // For now, we can't easily determine table OID from type alone
    // This would require more complex analysis
    None
}

/// Extract table names from SQL using simple pattern matching
fn extract_table_names_from_sql(sql: &str) -> Vec<String> {
    let mut tables = Vec::new();
    let sql_upper = sql.to_uppercase();

    // Simple regex-like patterns for common SQL operations
    let patterns = [
        ("INSERT INTO ", " "),
        ("UPDATE ", " SET"),
        ("FROM ", " "),
        ("JOIN ", " ON"),
    ];

    for (start_pattern, end_pattern) in patterns {
        if let Some(start_pos) = sql_upper.find(start_pattern) {
            let remaining = &sql[start_pos + start_pattern.len()..];
            if let Some(end_pos) = remaining.to_uppercase().find(end_pattern) {
                let table_part = remaining[..end_pos].trim();
                // Extract just the table name (handle schema.table)
                let table_name = table_part
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .split('(')
                    .next()
                    .unwrap_or("")
                    .trim();
                if !table_name.is_empty() && !tables.contains(&table_name.to_string()) {
                    tables.push(table_name.to_string());
                }
            }
        }
    }

    tables
}

/// Get table OID by table name
async fn get_table_oid_by_name(
    client: &tokio_postgres::Client,
    table_name: &str,
) -> Result<Option<u32>> {
    let parts: Vec<&str> = table_name.split('.').collect();
    match parts.as_slice() {
        // Case 1: schema-qualified name: look only there
        [schema, name] => {
            let row = client
                .query_opt(
                    r#"
                    SELECT c.oid
                    FROM pg_class AS c
                    JOIN pg_namespace AS n ON n.oid = c.relnamespace
                    WHERE c.relname = $1
                      AND n.nspname = $2
                    "#,
                    &[&*name, &*schema],
                )
                .await?;
            Ok(row.map(|r| r.get(0)))
        }
        _ => Ok(None),
    }
}

/// Query all constraints for a given table OID
async fn query_table_constraints(
    client: &tokio_postgres::Client,
    table_oid: u32,
) -> Result<Vec<ConstraintInfo>> {
    let mut constraints = Vec::new();

    // Query unique and primary key constraints
    let rows = client
        .query(
            r#"
            SELECT 
                c.conname as constraint_name,
                c.contype::text as constraint_type,
                t.relname as table_name,
                array_agg(a.attname ORDER BY u.attposition) as column_names
            FROM pg_constraint c
            JOIN pg_class t ON c.conrelid = t.oid
            JOIN LATERAL unnest(c.conkey) WITH ORDINALITY AS u(attnum, attposition) ON true
            JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = u.attnum
            WHERE c.conrelid = $1 
                AND c.contype IN ('u', 'p', 'f', 'c')
            GROUP BY c.conname, c.contype, t.relname, c.confrelid, c.confkey
            "#,
            &[&table_oid],
        )
        .await?;

    for row in rows {
        let constraint_name: String = row.get(0);
        let constraint_type_str: String = row.get(1);
        let constraint_type_char: char = constraint_type_str.chars().next().unwrap_or('?');
        let table_name: String = row.get(2);

        let constraint_type = match constraint_type_char {
            'u' => "unique",
            'p' => "primary_key",
            'f' => "foreign_key",
            'c' => "check",
            _ => "other",
        }
        .to_string();

        constraints.push(ConstraintInfo {
            name: constraint_name,
            constraint_type,
            table_name,
        });
    }

    // Query foreign key references separately
    let fk_rows = client
        .query(
            r#"
            SELECT 
                c.conname as constraint_name,
                t.relname as table_name,
                array_agg(a.attname ORDER BY u.attposition) as column_names,
                ft.relname as referenced_table,
                array_agg(fa.attname ORDER BY fu.attposition) as referenced_columns
            FROM pg_constraint c
            JOIN pg_class t ON c.conrelid = t.oid
            JOIN pg_class ft ON c.confrelid = ft.oid
            JOIN LATERAL unnest(c.conkey) WITH ORDINALITY AS u(attnum, attposition) ON true
            JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = u.attnum
            JOIN LATERAL unnest(c.confkey) WITH ORDINALITY AS fu(attnum, attposition) ON true
            JOIN pg_attribute fa ON fa.attrelid = ft.oid AND fa.attnum = fu.attnum
            WHERE c.conrelid = $1 AND c.contype = 'f'
            GROUP BY c.conname, t.relname, ft.relname
            "#,
            &[&table_oid],
        )
        .await?;

    for row in fk_rows {
        let constraint_name: String = row.get(0);
        let table_name: String = row.get(1);

        constraints.push(ConstraintInfo {
            name: constraint_name,
            constraint_type: "foreign_key".to_string(),
            table_name,
        });
    }

    // Query NOT NULL constraints
    let nn_rows = client
        .query(
            r#"
            SELECT 
                a.attname as column_name,
                t.relname as table_name
            FROM pg_attribute a
            JOIN pg_class t ON a.attrelid = t.oid
            WHERE a.attrelid = $1 
                AND a.attnotnull = true
                AND a.attnum > 0
                AND NOT a.attisdropped
            "#,
            &[&table_oid],
        )
        .await?;

    for row in nn_rows {
        let column_name: String = row.get(0);
        let table_name: String = row.get(1);

        constraints.push(ConstraintInfo {
            name: format!("{}_{}_not_null", table_name, column_name),
            constraint_type: "not_null".to_string(),
            table_name,
        });
    }

    Ok(constraints)
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
                    is_nullable: false,
                    is_optional: is_optional_param,
                    needs_json_wrapper: true, // Custom input parameters need JSON serialization
                    enum_variants: None,
                    pg_type_name: None,
                };
            } else if is_optional_param {
                // If it's an optional parameter but no custom type, mark as nullable
                rust_type.is_nullable = false;
                rust_type.is_optional = true;
            }
        } else if is_optional_param {
            // If no mappings and it's optional parameter, mark as nullable
            rust_type.is_nullable = false;
            rust_type.is_optional = true;
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

/// Get enum type information from PostgreSQL system catalogs with caching
pub async fn get_enum_type_info(
    client: &tokio_postgres::Client,
    type_oid: u32,
) -> Result<Option<EnumTypeInfo>> {
    // Initialize cache if not already done
    let cache = ENUM_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    // Check cache first
    {
        let cache_lock = cache.lock().await;
        if let Some(cached_result) = cache_lock.get(&type_oid) {
            return Ok(cached_result.clone());
        }
    }

    // Not in cache, query the database
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

    let result = if let Some(row) = rows.first() {
        let type_name: String = row.get(0);
        let enum_values: Vec<String> = row.get(1);

        Some(EnumTypeInfo {
            type_name,
            variants: enum_values,
        })
    } else {
        None
    };

    // Cache the result
    {
        let mut cache_lock = cache.lock().await;
        cache_lock.insert(type_oid, result.clone());
    }

    Ok(result)
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
                    is_optional: false,
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
    let base_type = match *pg_type {
        // Boolean & Numeric Types
        PgType::BOOL => "bool",
        PgType::BOOL_ARRAY => "Vec<bool>",
        PgType::CHAR => "i8",
        PgType::CHAR_ARRAY => "Vec<i8>",
        PgType::INT2 => "i16",
        PgType::INT2_ARRAY => "Vec<i16>",
        PgType::INT4 => "i32",
        PgType::INT4_ARRAY => "Vec<i32>",
        PgType::INT8 => "i64",
        PgType::INT8_ARRAY => "Vec<i64>",
        PgType::FLOAT4 => "f32",
        PgType::FLOAT4_ARRAY => "Vec<f32>",
        PgType::FLOAT8 => "f64",
        PgType::FLOAT8_ARRAY => "Vec<f64>",
        PgType::NUMERIC => "rust_decimal::Decimal",
        PgType::NUMERIC_ARRAY => "Vec<rust_decimal::Decimal>",
        PgType::REGPROC => "u32",
        PgType::OID => "u32",
        PgType::TID => "(u32, u32)",
        PgType::XID => "u32",
        PgType::CID => "u32",
        PgType::XID8 => "u64",

        // String & Text Types
        PgType::NAME => "String",
        PgType::NAME_ARRAY => "Vec<String>",
        PgType::TEXT => "String",
        PgType::TEXT_ARRAY => "Vec<String>",
        PgType::BPCHAR => "String",
        PgType::BPCHAR_ARRAY => "Vec<String>",
        PgType::VARCHAR => "String",
        PgType::VARCHAR_ARRAY => "Vec<String>",
        PgType::XML => "String",
        PgType::XML_ARRAY => "Vec<String>",
        PgType::JSON => "serde_json::Value",
        PgType::JSON_ARRAY => "Vec<serde_json::Value>",
        PgType::JSONB => "serde_json::Value",
        PgType::JSONB_ARRAY => "Vec<serde_json::Value>",
        PgType::JSONPATH => "String",

        // Binary & Bit Types
        PgType::BYTEA => "Vec<u8>",
        PgType::BYTEA_ARRAY => "Vec<Vec<u8>>",
        PgType::BIT => "bit_vec::BitVec",
        PgType::BIT_ARRAY => "Vec<bit_vec::BitVec>",
        PgType::VARBIT => "bit_vec::BitVec",
        PgType::VARBIT_ARRAY => "Vec<bit_vec::BitVec>",

        // Date & Time Types
        PgType::DATE => "chrono::NaiveDate",
        PgType::DATE_ARRAY => "Vec<chrono::NaiveDate>",
        PgType::TIME => "chrono::NaiveTime",
        PgType::TIME_ARRAY => "Vec<chrono::NaiveTime>",
        PgType::TIMESTAMP => "chrono::NaiveDateTime",
        PgType::TIMESTAMP_ARRAY => "Vec<chrono::NaiveDateTime>",
        PgType::TIMESTAMPTZ => "chrono::DateTime<chrono::Utc>",
        PgType::TIMESTAMPTZ_ARRAY => "Vec<chrono::DateTime<chrono::Utc>>",
        PgType::INTERVAL => "sqlx::postgres::types::PgInterval",
        PgType::INTERVAL_ARRAY => "Vec<sqlx::postgres::types::PgInterval>",
        PgType::TIMETZ => "sqlx::postgres::types::PgTimeTz",
        PgType::TIMETZ_ARRAY => "Vec<sqlx::postgres::types::PgTimeTz>",

        // Range Types
        PgType::INT4_RANGE => "sqlx::postgres::types::PgRange<i32>",
        PgType::INT4_RANGE_ARRAY => "Vec<sqlx::postgres::types::PgRange<i32>>",
        PgType::INT8_RANGE => "sqlx::postgres::types::PgRange<i64>",
        PgType::INT8_RANGE_ARRAY => "Vec<sqlx::postgres::types::PgRange<i64>>",
        PgType::NUM_RANGE => "sqlx::postgres::types::PgRange<rust_decimal::Decimal>",
        PgType::NUM_RANGE_ARRAY => "Vec<sqlx::postgres::types::PgRange<rust_decimal::Decimal>>",
        PgType::TS_RANGE => "sqlx::postgres::types::PgRange<chrono::NaiveDateTime>",
        PgType::TS_RANGE_ARRAY => "Vec<sqlx::postgres::types::PgRange<chrono::NaiveDateTime>>",
        PgType::TSTZ_RANGE => "sqlx::postgres::types::PgRange<chrono::DateTime<chrono::Utc>>",
        PgType::TSTZ_RANGE_ARRAY => {
            "Vec<sqlx::postgres::types::PgRange<chrono::DateTime<chrono::Utc>>>"
        }
        PgType::DATE_RANGE => "sqlx::postgres::types::PgRange<chrono::NaiveDate>",
        PgType::DATE_RANGE_ARRAY => "Vec<sqlx::postgres::types::PgRange<chrono::NaiveDate>>",

        // Multirange Types - sqlx doesn't support multirange types natively, use JSON
        PgType::INT4MULTI_RANGE => "serde_json::Value",
        PgType::INT4MULTI_RANGE_ARRAY => "serde_json::Value",
        PgType::INT8MULTI_RANGE => "serde_json::Value",
        PgType::INT8MULTI_RANGE_ARRAY => "serde_json::Value",
        PgType::NUMMULTI_RANGE => "serde_json::Value",
        PgType::NUMMULTI_RANGE_ARRAY => "serde_json::Value",
        PgType::TSMULTI_RANGE => "serde_json::Value",
        PgType::TSMULTI_RANGE_ARRAY => "serde_json::Value",
        PgType::TSTZMULTI_RANGE => "serde_json::Value",
        PgType::TSTZMULTI_RANGE_ARRAY => "serde_json::Value",
        PgType::DATEMULTI_RANGE => "serde_json::Value",
        PgType::DATEMULTI_RANGE_ARRAY => "serde_json::Value",

        // Network & Address Types
        PgType::CIDR => "std::net::IpAddr",
        PgType::CIDR_ARRAY => "Vec<std::net::IpAddr>",
        PgType::INET => "std::net::IpAddr",
        PgType::INET_ARRAY => "Vec<std::net::IpAddr>",
        PgType::MACADDR => "mac_address::MacAddress",
        PgType::MACADDR_ARRAY => "Vec<mac_address::MacAddress>",

        // Geometric Types
        PgType::POINT => "sqlx::postgres::types::PgPoint",
        PgType::POINT_ARRAY => "Vec<sqlx::postgres::types::PgPoint>",
        PgType::LSEG => "sqlx::postgres::types::PgLseg",
        PgType::LSEG_ARRAY => "Vec<sqlx::postgres::types::PgLseg>",
        PgType::PATH => "sqlx::postgres::types::PgPath",
        PgType::PATH_ARRAY => "Vec<sqlx::postgres::types::PgPath>",
        PgType::BOX => "sqlx::postgres::types::PgBox",
        PgType::BOX_ARRAY => "Vec<sqlx::postgres::types::PgBox>",
        PgType::POLYGON => "sqlx::postgres::types::PgPolygon",
        PgType::POLYGON_ARRAY => "Vec<sqlx::postgres::types::PgPolygon>",
        PgType::CIRCLE => "sqlx::postgres::types::PgCircle",
        PgType::CIRCLE_ARRAY => "Vec<sqlx::postgres::types::PgCircle>",
        PgType::LINE => "sqlx::postgres::types::PgLine",
        PgType::LINE_ARRAY => "Vec<sqlx::postgres::types::PgLine>",

        // Special & System Types
        PgType::TSQUERY => "String",
        PgType::TSQUERY_ARRAY => "Vec<String>",
        PgType::REGCONFIG => "u32",
        PgType::REGDICTIONARY => "u32",
        PgType::REGNAMESPACE => "u32",
        PgType::REGROLE => "u32",
        PgType::REGCOLLATION => "u32",
        PgType::ACLITEM => "String",
        PgType::PG_NDISTINCT => "String",
        PgType::PG_DEPENDENCIES => "String",
        PgType::PG_BRIN_BLOOM_SUMMARY => "String",
        PgType::PG_BRIN_MINMAX_MULTI_SUMMARY => "String",
        PgType::PG_MCV_LIST => "String",
        PgType::PG_SNAPSHOT => "String",
        PgType::TXID_SNAPSHOT => "String",
        PgType::UUID => "uuid::Uuid",
        PgType::UUID_ARRAY => "Vec<uuid::Uuid>",

        PgType::PG_LSN => "u64",
        PgType::PG_LSN_ARRAY => "Vec<u64>",

        // Pseudo-types, handlers, and unknowns: map to serde_json::Value
        PgType::UNKNOWN
        | PgType::RECORD
        | PgType::ANY
        | PgType::ANYARRAY
        | PgType::VOID
        | PgType::TRIGGER
        | PgType::LANGUAGE_HANDLER
        | PgType::INTERNAL
        | PgType::ANYELEMENT
        | PgType::RECORD_ARRAY
        | PgType::ANYNONARRAY
        | PgType::FDW_HANDLER
        | PgType::TSM_HANDLER
        | PgType::ANYENUM => "serde_json::Value",

        // Enum types and fallback
        _ => {
            // Check if this is an enum type by trying to get enum info
            if let Some(enum_info) = get_enum_type_info(client, pg_type.oid()).await? {
                let enum_name = to_pascal_case(&enum_info.type_name);
                return Ok(RustType {
                    rust_type: enum_name,
                    is_nullable,
                    is_optional: false,
                    needs_json_wrapper: false,
                    enum_variants: Some(enum_info.variants),
                    pg_type_name: Some(enum_info.type_name),
                });
            }
            return Ok(RustType {
                rust_type: format!("/* Unknown type: {} */ String", pg_type.name()),
                is_nullable,
                is_optional: false,
                needs_json_wrapper: false,
                enum_variants: None,
                pg_type_name: None,
            });
        }
    };

    Ok(RustType {
        rust_type: base_type.to_string(),
        is_nullable,
        is_optional: false,
        needs_json_wrapper: false,
        enum_variants: None,
        pg_type_name: None,
    })
}


/// Parse SQL to extract meaningful parameter names from named parameters
pub fn parse_parameter_names_from_sql(sql: &str) -> Vec<String> {
    // Look for named parameters in the format #{param_name}
    let mut param_names = Vec::new();
    let mut chars = sql.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '#' {
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
        if ch == '#' {
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
                            result.base_sql.push_str(&format!("#[{}]", block_content));
                            break;
                        } else {
                            block_content.push(inner_ch);
                        }
                    } else {
                        block_content.push(inner_ch);
                    }
                }
            } else if let Some(&'{') = chars.peek() {
                // Found regular parameter #{param}
                chars.next(); // consume '{'
                let mut param_name = String::new();

                while let Some(inner_ch) = chars.next() {
                    if inner_ch == '}' {
                        if !param_name.is_empty() {
                            result.all_parameters.push(param_name.clone());
                            result.base_sql.push_str("#{");
                            result.base_sql.push_str(&param_name);
                            result.base_sql.push('}');
                        }
                        break;
                    } else {
                        param_name.push(inner_ch);
                    }
                }
            } else {
                // Regular # character
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

    // Replace conditional blocks #[...] with their inner content
    for block in &parsed_sql.conditional_blocks {
        let conditional_block = format!("#[{}]", block.sql_content);
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
        if ch == '#' {
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

