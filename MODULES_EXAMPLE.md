# Module Organization Feature

AutoModel now supports organizing generated functions into modules for better code organization.

## Configuration

Add an optional `module` field to your query definitions in `queries.yaml`:

```yaml
queries:
  - name: get_current_time
    sql: "SELECT NOW() as current_time"
    description: "Get the current timestamp"
    module: "admin"  # This function will be generated in admin.rs

  - name: get_version
    sql: "SELECT version() as pg_version"
    description: "Get PostgreSQL version"
    module: "admin"  # This function will also be in admin.rs

  - name: insert_user
    sql: |
      INSERT INTO users (name, email, age, profile)
      VALUES (${name}, ${email}, ${age}, ${profile})
      RETURNING id, name, email, age, created_at
    description: "Insert a new user"
    module: "users"  # This function will be in users.rs

  - name: test_query
    sql: "SELECT 1 as test"
    description: "Test query"
    # No module specified - will be in mod.rs
```

## Module Name Validation

Module names must be valid Rust identifiers:
- Start with a letter (a-z, A-Z) or underscore (_)
- Contain only letters, numbers, and underscores
- Cannot be Rust reserved keywords (fn, mod, let, etc.)

**Valid module names:**
- `users`
- `admin_panel`
- `user_management`
- `_private`

**Invalid module names:**
- `123invalid` (starts with number)
- `invalid-name` (contains hyphen)
- `invalid.name` (contains dot)
- `fn` (reserved keyword)
- `mod` (reserved keyword)

## Generated Structure

AutoModel will generate the following file structure:

```
src/generated/
├── mod.rs          # Module declarations + functions without module
├── admin.rs        # Functions with module: "admin"
└── users.rs        # Functions with module: "users"
```

## Usage in Rust Code

```rust
mod generated;

use tokio_postgres::Client;

async fn example(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    // Admin functions
    let current_time = generated::admin::get_current_time(client).await?;
    let version = generated::admin::get_version(client).await?;
    
    // User functions
    let user = generated::users::insert_user(
        client, 
        "John".to_string(), 
        "john@example.com".to_string(), 
        30, 
        serde_json::json!({"role": "user"})
    ).await?;
    
    // Functions without module (in mod.rs)
    let test_result = generated::test_query(client).await?;
    
    Ok(())
}
```

## Benefits

1. **Better Organization**: Related functions are grouped together
2. **Namespace Separation**: Avoid function name conflicts
3. **Cleaner Imports**: Import only the modules you need
4. **Scalability**: Works well with large numbers of queries
5. **Backwards Compatible**: Queries without `module` field work as before
6. **Validation**: Invalid module names are caught at build time

## Multiple Functions Per Module

Multiple functions can belong to the same module:

```yaml
queries:
  - name: create_user
    module: "users"
  - name: update_user  
    module: "users"
  - name: delete_user
    module: "users"
  - name: get_admin_stats
    module: "admin"
  - name: get_system_info
    module: "admin"
```

This will create `users.rs` with 3 functions and `admin.rs` with 2 functions.

## Error Handling

If you use an invalid module name, AutoModel will provide a clear error message:

```
Error: Invalid module name in query 'get_user': Module name '123invalid' must start with a letter or underscore
```
