use crate::query_config::QueryDefinition;
use crate::type_extraction::{QueryTypeInfo, generate_input_params, generate_return_type, generate_result_struct};
use anyhow::Result;

/// Generate Rust function code for a SQL query
pub fn generate_function_code(query: &QueryDefinition, type_info: &QueryTypeInfo) -> Result<String> {
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
    code.push_str(&format!("/// Generated from SQL: {}\n", query.sql.trim()));
    
    // Generate function signature
    let input_params = generate_input_params(&type_info.input_types);
    let return_type = if type_info.output_types.len() > 1 {
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
        return_type.clone()
    };

    code.push_str(&format!(
        "pub async fn {}({}) -> Result<{}, tokio_postgres::Error> {{\n",
        query.name,
        params_str,
        final_return_type
    ));

    // Generate function body
    code.push_str(&generate_function_body(query, type_info, &return_type)?);
    
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
    
    // Prepare the statement
    body.push_str(&format!(
        "    let stmt = client.prepare(\"{}\").await?;\n",
        escape_sql_string(&query.sql)
    ));

    // Prepare parameters
    let param_refs = if type_info.input_types.is_empty() {
        "&[]".to_string()
    } else {
        let params: Vec<String> = (1..=type_info.input_types.len())
            .map(|i| format!("&param_{}", i))
            .collect();
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
        body.push_str(&format!(
            "    let row = client.query_one(&stmt, {}).await?;\n",
            param_refs
        ));
        body.push_str(&format!(
            "    Ok(row.get::<_, {}>(0))\n",
            type_info.output_types[0].rust_type.rust_type
        ));
    } else {
        // For queries that return multiple columns
        body.push_str(&format!(
            "    let row = client.query_one(&stmt, {}).await?;\n",
            param_refs
        ));
        body.push_str(&format!("    Ok({} {{\n", return_type));
        
        for (i, col) in type_info.output_types.iter().enumerate() {
            body.push_str(&format!(
                "        {}: row.get::<_, {}>({}),\n",
                to_snake_case(&col.name),
                col.rust_type.rust_type,
                i
            ));
        }
        
        body.push_str("    })\n");
    }

    Ok(body)
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
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
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
pub fn generate_module_code(queries: &[QueryDefinition], type_infos: &[QueryTypeInfo]) -> Result<String> {
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
    use crate::type_extraction::{RustType, OutputColumn};

    #[test]
    fn test_escape_sql_string() {
        assert_eq!(escape_sql_string(r#"SELECT "name" FROM users"#), r#"SELECT \"name\" FROM users"#);
        assert_eq!(escape_sql_string("SELECT *\nFROM users"), "SELECT *\\nFROM users");
        assert_eq!(escape_sql_string("SELECT * FROM users\r\n"), "SELECT * FROM users\\r\\n");
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
            tags: None,
        };

        let type_info = QueryTypeInfo {
            input_types: vec![RustType {
                rust_type: "i32".to_string(),
                is_nullable: false,
                pg_type: "INT4".to_string(),
            }],
            output_types: vec![
                OutputColumn {
                    name: "id".to_string(),
                    rust_type: RustType {
                        rust_type: "i32".to_string(),
                        is_nullable: false,
                        pg_type: "INT4".to_string(),
                    },
                },
                OutputColumn {
                    name: "name".to_string(),
                    rust_type: RustType {
                        rust_type: "String".to_string(),
                        is_nullable: false,
                        pg_type: "TEXT".to_string(),
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
