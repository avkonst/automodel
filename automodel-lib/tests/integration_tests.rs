use automodel::*;

#[tokio::test]
async fn test_yaml_parsing() {
    let yaml_content = r#"
queries:
  - name: get_user
    sql: "SELECT id, name FROM users WHERE id = $1"
    description: "Get a user by ID"
  - name: list_users
    sql: "SELECT id, name FROM users"
    description: "List all users"
metadata:
  version: "1.0"
"#;

    let config = parse_yaml_string(yaml_content).unwrap();
    assert_eq!(config.queries.len(), 2);
    assert_eq!(config.queries[0].name, "get_user");
    assert_eq!(config.queries[1].name, "list_users");
}

#[tokio::test]
async fn test_query_validation() {
    let valid_queries = vec![
        QueryDefinition {
            name: "valid_name".to_string(),
            sql: "SELECT 1".to_string(),
            description: None,
            tags: None,
        },
        QueryDefinition {
            name: "another_valid_name".to_string(),
            sql: "SELECT 2".to_string(),
            description: None,
            tags: None,
        },
    ];

    assert!(validate_query_names(&valid_queries).is_ok());

    let invalid_queries = vec![
        QueryDefinition {
            name: "123invalid".to_string(),
            sql: "SELECT 1".to_string(),
            description: None,
            tags: None,
        },
    ];

    assert!(validate_query_names(&invalid_queries).is_err());
}

#[test]
fn test_type_conversions() {
    use automodel::type_extraction::{generate_input_params, generate_return_type, RustType, OutputColumn};

    let input_types = vec![
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

    let params = generate_input_params(&input_types);
    assert_eq!(params, "param_1: i32, param_2: String");

    let output_types = vec![
        OutputColumn {
            name: "id".to_string(),
            rust_type: RustType {
                rust_type: "i32".to_string(),
                is_nullable: false,
                pg_type: "INT4".to_string(),
                needs_json_wrapper: false,
            },
        },
    ];

    let return_type = generate_return_type(&output_types);
    assert_eq!(return_type, "i32");
}

#[test]
fn test_code_generation() {
    use automodel::code_generation::generate_function_code;
    use automodel::type_extraction::{QueryTypeInfo, RustType, OutputColumn};

    let query = QueryDefinition {
        name: "get_count".to_string(),
        sql: "SELECT COUNT(*) FROM users".to_string(),
        description: Some("Count users".to_string()),
        tags: None,
    };

    let type_info = QueryTypeInfo {
        input_types: vec![],
        output_types: vec![
            OutputColumn {
                name: "count".to_string(),
                rust_type: RustType {
                    rust_type: "i64".to_string(),
                    is_nullable: false,
                    pg_type: "INT8".to_string(),
                    needs_json_wrapper: false,
                },
            },
        ],
    };

    let code = generate_function_code(&query, &type_info).unwrap();
    assert!(code.contains("pub async fn get_count"));
    assert!(code.contains("client: &tokio_postgres::Client"));
    assert!(code.contains("-> Result<i64, tokio_postgres::Error>"));
}
