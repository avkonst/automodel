# AutoModel Workspace

A Rust workspace for automatically generating typed functions from YAML-defined SQL queries using PostgreSQL.

## Project Structure

This is a Cargo workspace with three main components:

- **`automodel-lib/`** - The core library for generating typed functions from SQL queries
- **`automodel-cli/`** - Command-line interface with advanced features  
- **`example-app/`** - An example application that demonstrates build-time code generation

## Features

- 📝 Define SQL queries in YAML files with names and descriptions
- 🔌 Connect to PostgreSQL databases  
- 🔍 Automatically extract input and output types from prepared statements
- 🛠️ Generate Rust functions with proper type signatures at build time
- ✅ Support for all common PostgreSQL types including custom enums
- 🏗️ Generate result structs for multi-column queries
- ⚡ Build-time code generation with automatic regeneration when YAML changes
- 🎯 Advanced CLI with dry-run and flexible output options
- 📊 Built-in query performance analysis with sequential scan detection

## Quick Start

### 1. Clone and Build

```bash
git clone <repository-url>
cd automodel
cargo build
```

### 2. CLI Usage

The CLI tool provides several commands for different workflows:

#### Generate code

```bash
# Basic generation
cargo run -p automodel-cli -- generate -d postgresql://localhost/mydb -f queries.yaml

# Generate with custom output file
cargo run -p automodel-cli -- generate -d postgresql://localhost/mydb -f queries.yaml -o src/db_functions.rs

# Dry run (see generated code without writing files)
cargo run -p automodel-cli -- generate -d postgresql://localhost/mydb -f queries.yaml --dry-run
```

#### Query Performance Analysis

```bash
# Analysis is performed automatically during code generation (if analysis is enabled in the queries.yaml configuration file)
cargo run -p automodel-cli -- generate -d postgresql://localhost/mydb -f queries.yaml
```

#### CLI Help

```bash
# General help
cargo run -p automodel-cli -- --help

# Subcommand help
cargo run -p automodel-cli -- generate --help
```

### 3. Run the Example App

```bash
cd example-app
cargo run
```

The example app demonstrates:
- Build-time code generation via `build.rs`
- Automatic regeneration when YAML files change
- How to use generated functions in your application

## Library Usage (automodel-lib)

### Add to your Cargo.toml

```toml
[dependencies]
automodel-lib = { path = "../automodel-lib" }  # or from crates.io when published

[build-dependencies]  
automodel-lib = { path = "../automodel-lib" }
tokio = { version = "1.0", features = ["rt"] }
anyhow = "1.0"
```

### Create a build.rs for automatic code generation

```rust
use automodel::AutoModel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    AutoModel::generate_at_build_time("queries.yaml", "src/generated").await?;

    Ok(())
}
```

### Create queries.yaml

```yaml
queries:
  - name: get_user_by_id
    sql: "SELECT id, name, email FROM users WHERE id = ${id}"
    description: "Retrieve a user by their ID"
    
  - name: create_user
    sql: "INSERT INTO users (name, email) VALUES (${name}, ${email}) RETURNING id"
    description: "Create a new user and return the generated ID"
```

### Use the generated functions

```rust
mod generated;

use tokio_postgres::Client;

async fn example(client: &Client) -> Result<(), tokio_postgres::Error> {
    // The functions are generated at build time with proper types!
    let user = generated::get_user_by_id(client, 1).await?;
    let new_id = generated::create_user(client, "John".to_string(), "john@example.com".to_string()).await?;
    Ok(())
}
```

## Configuration Options

AutoModel uses YAML files to define SQL queries and their associated metadata. Here's a comprehensive guide to all configuration options:

### Root Configuration Structure

```yaml
# Default configuration for telemetry and analysis (optional)
defaults:
  telemetry:
    level: debug           # Global telemetry level
    include_sql: true      # Include SQL in spans globally
  ensure_indexes: true     # Enable query performance analysis globally

# List of query definitions
queries:
  - name: query_name
    sql: "SELECT ..."
    # ... other query options
```

### Default Configuration

The `defaults` section configures global settings for telemetry and analysis:

```yaml
defaults:
  telemetry:
    level: debug           # none | info | debug | trace (default: none)
    include_sql: true      # true | false (default: false)
  ensure_indexes: true     # true | false (default: false)
```

**Telemetry Levels:**
- `none` - No instrumentation
- `info` - Basic span creation with function name
- `debug` - Include SQL query in span (if include_sql is true)
- `trace` - Include both SQL query and parameters in span

**Query Analysis Features:**
- **Sequential scan detection**: Automatically detects queries that perform full table scans
- **Warnings during build**: Identifies queries that might benefit from indexing

### Query Configuration

Each query in the `queries` array supports these options:

#### Required Fields

```yaml
- name: get_user_by_id                    # Function name (must be valid Rust identifier)
  sql: "SELECT id, name FROM users WHERE id = ${id}"  # SQL query with named parameters
```

#### Optional Fields

```yaml
- name: get_user_by_id
  sql: "SELECT id, name FROM users WHERE id = ${id}"
  
  # Optional description (becomes function documentation)
  description: "Retrieve a user by their ID"
  
  # Optional module name (generates code in separate module)
  module: "users"                         # Must be valid Rust module name
  
  # Expected result behavior (default: exactly_one)
  expect: "exactly_one"                   # exactly_one | possible_one | at_least_one | multiple
  
  # Custom type mappings for fields
  types:
    "profile": "crate::models::UserProfile"     # Input/output field type override
    "users.profile": "crate::models::UserProfile"  # Table-qualified field override
  
  # Per-query telemetry configuration
  telemetry:
    level: trace                          # Override global telemetry level
    include_params: ["id", "name"]       # Specific parameters to include in spans
    include_sql: false                    # Override global SQL inclusion
  
  # Per-query analysis configuration
  ensure_indexes: true                     # Override global analysis setting for this query
```

### Expected Result Types

Controls how the query is executed and what it returns:

```yaml
expect: "exactly_one"    # fetch_one() -> Result<T, Error> - Fails if 0 or >1 rows
expect: "possible_one"   # fetch_optional() -> Result<Option<T>, Error> - 0 or 1 row
expect: "at_least_one"   # fetch_all() -> Result<Vec<T>, Error> - Fails if 0 rows
expect: "multiple"       # fetch_all() -> Result<Vec<T>, Error> - 0 or more rows (default for collections)
```

### Custom Type Mappings

Override PostgreSQL-to-Rust type mappings for specific fields:

```yaml
types:
  # For input parameters and output fields with this name
  "profile": "crate::models::UserProfile"
  
  # For output fields from specific table (when using JOINs)
  "users.profile": "crate::models::UserProfile"
  "posts.metadata": "crate::models::PostMetadata"
  
  # Custom enum types
  "status": "UserStatus"
  "category": "crate::enums::Category"
```

**Note:** Custom types must implement appropriate serialization traits:
- **Input parameters:** `serde::Serialize` (for JSON serialization)
- **Output fields:** `serde::Deserialize` (for JSON deserialization)

### Named Parameters

Use `${parameter_name}` syntax in SQL queries:

```yaml
sql: "SELECT * FROM users WHERE id = ${user_id} AND status = ${status}"
```

**Optional Parameters:**
Add `?` suffix for optional parameters that become `Option<T>`:

```yaml
sql: "SELECT * FROM posts WHERE user_id = ${user_id} AND (${category?} IS NULL OR category = ${category?})"
```

### Per-Query Telemetry Configuration

Override global telemetry settings for specific queries:

```yaml
telemetry:
  # Override global level for this query
  level: trace                    # none | info | debug | trace
  
  # Specify which parameters to include in spans
  include_params: ["user_id", "email"]   # Only these parameters will be logged
  include_params: []                      # Empty array = skip all parameters
  # If not specified, all parameters are skipped by default
  
  # Override SQL inclusion for this query
  include_sql: true               # true | false
```

### Per-Query Analysis Configuration

Override global analysis settings for specific queries:

```yaml
ensure_indexes: true               # true | false - Enable/disable analysis for this query
```

### Module Organization

Organize generated functions into modules:

```yaml
queries:
  - name: get_user
    module: "users"               # Generated in src/generated/users.rs
    
  - name: get_post  
    module: "posts"               # Generated in src/generated/posts.rs
    
  - name: health_check
    # No module specified          # Generated in src/generated/mod.rs
```

### Complete Example

```yaml
# Global configuration
defaults:
  telemetry:
    level: debug
    include_sql: false
  ensure_indexes: true           # Enable query performance analysis

queries:
  # Simple query with custom type
  - name: get_user_profile
    sql: "SELECT id, name, profile FROM users WHERE id = ${user_id}"
    description: "Get user profile with custom JSON type"
    module: "users"
    expect: "possible_one"
    types:
      "profile": "crate::models::UserProfile"
    telemetry:
      level: trace
      include_params: ["user_id"]
      include_sql: true
    ensure_indexes: true           # Enable analysis for this specific query
  
  # Query with optional parameter
  - name: search_posts
    sql: "SELECT * FROM posts WHERE user_id = ${user_id} AND (${category?} IS NULL OR category = ${category?})"
    description: "Search posts with optional category filter"
    module: "posts"
    expect: "multiple"
    types:
      "category": "PostCategory"
      "metadata": "crate::models::PostMetadata"
    ensure_indexes: true           # Check for sequential scans on posts table
  
  - name: create_sessions_table
    sql: "CREATE TABLE IF NOT EXISTS sessions (id UUID PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT NOW())"
    description: "Create sessions table"
    module: "setup"
    ensure_indexes: false # force DDL query to be skipped from analysis
  
  # Bulk operation with minimal telemetry
  - name: cleanup_old_sessions
    sql: "DELETE FROM sessions WHERE created_at < ${cutoff_date}"
    description: "Remove sessions older than cutoff date"
    module: "admin" 
    expect: "exactly_one"
    telemetry:
      include_params: []          # Skip all parameters for privacy
      include_sql: false
```

## Conditional Queries

AutoModel supports **conditional queries** that dynamically include or exclude SQL clauses based on parameter availability. This allows you to write flexible queries that adapt based on which optional parameters are provided.

### Conditional Syntax

Use the `$[...]` syntax to wrap optional SQL parts:

```yaml
- name: search_users
  sql: "SELECT id, name, email FROM users WHERE 1=1 $[AND name ILIKE ${name_pattern?}] $[AND age >= ${min_age?}] ORDER BY created_at DESC"
  description: "Search users with optional name and age filters"
```

**Key Components:**
- `$[AND name ILIKE ${name_pattern?}]` - Conditional block that includes the clause only if `name_pattern` is `Some`
- `${name_pattern?}` - Optional parameter (note the `?` suffix)
- The conditional block is removed entirely if the parameter is `None`

### Runtime SQL Examples

The same function generates different SQL based on parameter availability:

```rust
// Both parameters provided
search_users(executor, Some("%john%".to_string()), Some(25)).await?;
// SQL: "SELECT id, name, email FROM users WHERE 1=1 AND name ILIKE $1 AND age >= $2 ORDER BY created_at DESC"
// Params: ["%john%", 25]

// Only name pattern provided  
search_users(executor, Some("%john%".to_string()), None).await?;
// SQL: "SELECT id, name, email FROM users WHERE 1=1 AND name ILIKE $1 ORDER BY created_at DESC"
// Params: ["%john%"]

// Only age provided
search_users(executor, None, Some(25)).await?;
// SQL: "SELECT id, name, email FROM users WHERE 1=1 AND age >= $1 ORDER BY created_at DESC"  
// Params: [25]

// No optional parameters
search_users(executor, None, None).await?;
// SQL: "SELECT id, name, email FROM users WHERE 1=1 ORDER BY created_at DESC"
// Params: []
```

### Complex Conditional Queries

You can mix conditional and non-conditional parameters:

```yaml
- name: find_users_complex
  sql: "SELECT id, name, email, age FROM users WHERE name ILIKE ${name_pattern} $[AND age >= ${min_age?}] AND email IS NOT NULL $[AND created_at >= ${since?}] ORDER BY name"
  description: "Complex search with required name pattern and optional filters"
```

This generates a function with signature:
```rust
pub async fn find_users_complex(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    name_pattern: String,        // Required parameter
    min_age: Option<i32>,        // Optional parameter
    since: Option<chrono::DateTime<chrono::Utc>>  // Optional parameter
) -> Result<Vec<FindUsersComplexItem>, sqlx::Error>
```

### Best Practices

1. **Use `WHERE 1=1`** as a base condition when all WHERE clauses are conditional:
   ```yaml
   sql: "SELECT * FROM users WHERE 1=1 $[AND name = ${name?}] $[AND age > ${min_age?}]"
   ```

### Parameter Binding and Performance

- **Sequential parameter binding**: AutoModel automatically renumbers parameters to ensure sequential binding ($1, $2, $3, etc.)
- **No SQL parsing overhead**: Parameter renumbering happens at function generation time, not runtime
- **Prepared statement compatibility**: Generated SQL is fully compatible with PostgreSQL prepared statements
- **Type safety**: All parameter types are validated at compile time

### Limitations

- **No nested conditionals**: `$[...]` blocks cannot be nested inside other conditional blocks
- **Parameter uniqueness**: Each optional parameter can only be used once per conditional block

## CLI Features

### Commands

- **`generate`** - Generate Rust code from YAML definitions

### CLI Options

#### Generate Command
- `-d, --database-url <URL>` - Database connection URL
- `-f, --file <FILE>` - YAML file with query definitions
- `-o, --output <FILE>` - Custom output file path
- `-m, --module <NAME>` - Module name for generated code
- `--dry-run` - Preview generated code without writing files


## Examples

The `examples/` directory contains:

- `queries.yaml` - Sample query definitions
- `schema.sql` - Database schema for testing

## Workspace Commands

```bash
# Build everything
cargo build

# Test the library
cargo test -p automodel-lib

# Run the CLI tool
cargo run -p automodel-cli -- [args...]

# Run the example app
cargo run -p example-app

# Check specific package
cargo check -p automodel-lib
cargo check -p automodel-cli
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

## Requirements

- PostgreSQL database (for actual code generation)
- Rust 1.70+
- tokio runtime

## License

MIT License - see LICENSE file for details.
