# AutoModel

A Rust library for automatically generating typed functions from YAML-defined SQL queries using PostgreSQL and sea-query.

## Features

- 📝 Define SQL queries in YAML files with names and descriptions
- 🔌 Connect to PostgreSQL databases
- 🔍 Automatically extract input and output types from prepared statements
- 🛠️ Generate Rust functions with proper type signatures
- ✅ Support for all common PostgreSQL types
- 🏗️ Generate result structs for multi-column queries

## CLI Usage

AutoModel also provides a command-line interface for easy code generation:

### Install the CLI

```bash
cargo install --git https://github.com/yourusername/automodel --bin automodel-cli
```

### Use the CLI

```bash
# Generate Rust code from YAML queries
automodel-cli postgresql://localhost/mydb queries.yaml

# This will create a queries.rs file with all generated functions
```

### CLI Example

```bash
$ automodel-cli postgresql://localhost/test examples/user_queries.yaml

AutoModel Code Generator
=======================
Database URL: postgresql://localhost/test
YAML file: examples/user_queries.yaml

Loading queries from YAML file...
✓ Successfully loaded 7 queries
  1. get_user_by_id: Retrieve a user by their ID
  2. list_active_users: List all active users ordered by name
  3. create_user: Create a new user and return the generated ID
  4. update_user_email: Update a user's email address
  5. delete_user: Delete a user by ID
  6. get_user_posts: Get all posts by a specific user
  7. count_users_by_status: Count users grouped by their active status

Connecting to database and generating code...
✓ Successfully generated Rust code
✓ Generated code written to: examples/user_queries.rs

You can now include this file in your Rust project:
  mod user_queries;
```

## Library Usage

### Quick Start

#### 1. Add to your Cargo.toml

```toml
[dependencies]
automodel = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
tokio-postgres = "0.7"
```

#### 2. Create a YAML file with your queries

```yaml
# queries.yaml
queries:
  - name: get_user_by_id
    sql: "SELECT id, name, email FROM users WHERE id = $1"
    description: "Retrieve a user by their ID"
    
  - name: create_user
    sql: "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id"
    description: "Create a new user and return the generated ID"
```

#### 3. Generate Rust functions

```rust
use automodel::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = "postgresql://localhost/mydb";
    let mut automodel = AutoModel::new(database_url.to_string());
    
    // Load queries from YAML
    automodel.load_queries_from_file("queries.yaml").await?;
    
    // Generate Rust code
    let generated_code = automodel.generate_code().await?;
    
    // Write to file or use directly
    tokio::fs::write("src/generated.rs", generated_code).await?;
    
    Ok(())
}
```

#### 4. Use the generated functions

The generated code will look like this:

```rust
/// Retrieve a user by their ID
/// Generated from SQL: SELECT id, name, email FROM users WHERE id = $1
#[derive(Debug, Clone)]
pub struct GetUserByIdResult {
    pub id: i32,
    pub name: String,
    pub email: String,
}

pub async fn get_user_by_id(client: &tokio_postgres::Client, param_1: i32) -> Result<GetUserByIdResult, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT id, name, email FROM users WHERE id = $1").await?;
    let row = client.query_one(&stmt, &[&param_1]).await?;
    Ok(GetUserByIdResult {
        id: row.get::<_, i32>(0),
        name: row.get::<_, String>(1),
        email: row.get::<_, String>(2),
    })
}

/// Create a new user and return the generated ID
/// Generated from SQL: INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id
pub async fn create_user(client: &tokio_postgres::Client, param_1: String, param_2: String) -> Result<i32, tokio_postgres::Error> {
    let stmt = client.prepare("INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id").await?;
    let row = client.query_one(&stmt, &[&param_1, &param_2]).await?;
    Ok(row.get::<_, i32>(0))
}
```

## YAML Schema

```yaml
queries:
  - name: function_name          # Required: Valid Rust function name
    sql: "SELECT ..."           # Required: SQL query with $1, $2, etc. for parameters
    description: "..."          # Optional: Function documentation
    tags: ["tag1", "tag2"]      # Optional: Tags for organization

metadata:                       # Optional
  version: "1.0"
  description: "Query collection description"
  author: "Your Name"
```

## Supported PostgreSQL Types

| PostgreSQL Type | Rust Type |
|----------------|-----------|
| `BOOL` | `bool` |
| `INT2` | `i16` |
| `INT4` | `i32` |
| `INT8` | `i64` |
| `FLOAT4` | `f32` |
| `FLOAT8` | `f64` |
| `TEXT`, `VARCHAR` | `String` |
| `BYTEA` | `Vec<u8>` |
| `TIMESTAMP` | `chrono::NaiveDateTime` |
| `TIMESTAMPTZ` | `chrono::DateTime<chrono::Utc>` |
| `DATE` | `chrono::NaiveDate` |
| `TIME` | `chrono::NaiveTime` |
| `UUID` | `uuid::Uuid` |
| `JSON`, `JSONB` | `serde_json::Value` |
| `INET` | `std::net::IpAddr` |
| `NUMERIC` | `rust_decimal::Decimal` |

All types support `Option<T>` for nullable columns.

## Examples

See the `examples/` directory for:

- `user_queries.yaml` - Sample query definitions
- `schema.sql` - Database schema for testing
- `basic_usage.rs` - Complete usage example

## Running Examples

1. Set up a PostgreSQL database:
```bash
createdb test_automodel
psql test_automodel < examples/schema.sql
```

2. Set the database URL:
```bash
export DATABASE_URL="postgresql://localhost/test_automodel"
```

3. Run the example:
```bash
cargo run --example basic_usage
```

## Requirements

- PostgreSQL database
- Rust 1.70+
- tokio runtime

## License

MIT License - see LICENSE file for details.
