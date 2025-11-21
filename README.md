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
- 📊 Built-in query performance analysis with sequential scan detection
- 🔄 Conditional queries with dynamic SQL based on optional parameters
- ♻️ Struct reuse and deduplication across queries
- 🔀 Diff-based conditional updates for precise change tracking
- 🎨 Custom struct naming for cleaner, domain-specific APIs

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

**Option 1: Using YAML files**

```rust
use automodel::AutoModel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    AutoModel::generate_at_build_time("queries.yaml", "src/generated").await?;

    Ok(())
}
```

**Option 2: Using the builder pattern (no YAML file)**

```rust
use automodel::{AutoModel, AutoModelBuilder, QueryBuilder, TelemetryLevel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let builder = AutoModelBuilder::new()
        .default_telemetry(TelemetryLevel::Debug)
        .query(
            QueryBuilder::new("get_user", "SELECT id, name FROM users WHERE id = ${id}")
                .module("users")
                .expect_one()
        )
        .query(QueryBuilder::from_file("get_user", "queries/get_user.sql"))
        .query(
            QueryBuilder::new("insert_user", "INSERT INTO users (name, email) VALUES (${name}, ${email})")
                .module("users")
                .error_type("UserError")
        );
    
    AutoModel::generate_from_builder_at_build_time(builder, "src/generated").await?;
    Ok(())
}
```

### Create queries.yaml (for YAML approach)

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
  module: "database"       # Default module for queries without explicit module (optional)
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
  
  # Batch insert optimization with UNNEST pattern
  multiunzip: false                        # Default: false. Enable for UNNEST-based batch inserts
  
  # Diff-based conditional parameters (for conditional queries with $[...])
  conditions_type: false                   # Default: false. Use old/new struct comparison instead of Option<T>
  
  # Structured parameters - all params as single struct
  parameters_type: false                   # Default: false. Group all parameters into one struct (ignored if conditions_type is true)
  
  # Custom return type name (for multi-column SELECT queries)
  return_type: "UserInfo"                  # Default: auto-generated. Custom name or reuse existing struct
  
  # Custom error type name (for mutation queries with constraint violations)
  error_type: "UserError"                  # Default: auto-generated. Custom name or reuse existing error type
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
) -> Result<Vec<FindUsersComplexItem>, super::ErrorReadOnly>
```

### Best Practices

1. **Use `WHERE 1=1`** as a base condition when all WHERE clauses are conditional:
   ```yaml
   sql: "SELECT * FROM users WHERE 1=1 $[AND name = ${name?}] $[AND age > ${min_age?}]"
   ```

### Conditional UPDATE Statements

Conditional syntax is also useful for UPDATE statements where you want to update only certain fields based on which parameters are provided:

```yaml
- name: update_user_fields
  sql: "UPDATE users SET updated_at = NOW() $[, name = ${name?}] $[, email = ${email?}] $[, age = ${age?}] WHERE id = ${user_id} RETURNING id, name, email, age, updated_at"
  description: "Update user fields conditionally - only updates fields that are provided (not None)"
  module: "users"
  expect: "exactly_one"
```

This generates a function that allows partial updates:

```rust
// Update only the name
update_user_fields(executor, user_id, Some("Jane Doe".to_string()), None, None).await?;
// SQL: "UPDATE users SET updated_at = NOW(), name = $1 WHERE id = $2 RETURNING ..."

// Update only the age  
update_user_fields(executor, user_id, None, None, Some(35)).await?;
// SQL: "UPDATE users SET updated_at = NOW(), age = $1 WHERE id = $2 RETURNING ..."

// Update multiple fields
update_user_fields(executor, user_id, Some("Jane".to_string()), Some("jane@example.com".to_string()), None).await?;
// SQL: "UPDATE users SET updated_at = NOW(), name = $1, email = $2 WHERE id = $3 RETURNING ..."

// Update all fields
update_user_fields(executor, user_id, Some("Janet".to_string()), Some("janet@example.com".to_string()), Some(40)).await?;
// SQL: "UPDATE users SET updated_at = NOW(), name = $1, email = $2, age = $3 WHERE id = $4 RETURNING ..."
```

**Note**: Always include at least one non-conditional SET clause (like `updated_at = NOW()`) to ensure the UPDATE statement is syntactically valid even when all optional parameters are `None`.

## Struct Configuration and Reuse

AutoModel provides four powerful configuration options that allow you to customize how structs and error types are generated and reused across queries: `parameters_type`, `conditions_type`, `return_type`, and `error_type`. These options enable you to eliminate code duplication, improve type safety, and create cleaner APIs.

### Overview

| Option | Purpose | Default | Accepts | Generates |
|--------|---------|---------|---------|-----------|
| `parameters_type` | Group query parameters into a struct | `false` | `true` or struct name | `{QueryName}Params` struct |
| `conditions_type` | Diff-based conditional parameters | `false` | `true` or struct name | `{QueryName}Params` struct with old/new comparison |
| `return_type` | Custom name for return type struct | auto | struct name or omit | Custom named or `{QueryName}Item` struct |
| `error_type` | Custom name for error constraint enum (mutations only) | auto | error type name or omit | Custom named or `{QueryName}Constraints` enum |

Any structure or error type generated can be referenced by other queries. AutoModel validates at build time that the types are compatible and constraints match exactly.

### parameters_type: Structured Parameters

Group all query parameters into a single struct instead of passing them individually. Makes function calls cleaner and enables parameter reuse.

**Basic Usage:**

```yaml
- name: insert_user_structured
  sql: "INSERT INTO users (name, email, age) VALUES (${name}, ${email}, ${age}) RETURNING id"
  parameters_type: true  # Generates InsertUserStructuredParams
```

**Generated Code:**

```rust
#[derive(Debug, Clone)]
pub struct InsertUserStructuredParams {
    pub name: String,
    pub email: String,
    pub age: i32,
}

pub async fn insert_user_structured(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    params: &InsertUserStructuredParams
) -> Result<i32, super::Error<InsertUserStructuredConstraints>>
```

**Usage:**

```rust
let params = InsertUserStructuredParams {
    name: "Alice".to_string(),
    email: "alice@example.com".to_string(),
    age: 30,
};
insert_user_structured(executor, &params).await?;
```

**Struct Reuse:**

Specify an existing struct name to reuse it across queries:

```yaml
queries:
  # First query generates the struct
  - name: get_user_by_id_and_email
    sql: "SELECT id, name, email FROM users WHERE id = ${id} AND email = ${email}"
    parameters_type: true  # Generates GetUserByIdAndEmailParams
  
  # Second query reuses the same struct
  - name: delete_user_by_id_and_email
    sql: "DELETE FROM users WHERE id = ${id} AND email = ${email} RETURNING id"
    parameters_type: "GetUserByIdAndEmailParams"  # Reuses existing struct
```

Only one struct definition is generated, shared by both functions.

### conditions_type: Diff-Based Conditional Parameters

For queries with conditional SQL (`$[...]` blocks), generate a struct and compare old vs new values to decide which clauses to include. Works with any query type (SELECT, UPDATE, DELETE, etc.).

**Basic Usage:**

```yaml
- name: update_user_fields_diff
  sql: "UPDATE users SET updated_at = NOW() $[, name = ${name?}] $[, email = ${email?}] WHERE id = ${user_id}"
  conditions_type: true  # Generates UpdateUserFieldsDiffParams
```

**Generated Code:**

```rust
pub struct UpdateUserFieldsDiffParams {
    pub name: String,
    pub email: String,
}

pub async fn update_user_fields_diff(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    old: &UpdateUserFieldsDiffParams,
    new: &UpdateUserFieldsDiffParams,
    user_id: i32
) -> Result<(), super::Error<UpdateUserFieldsDiffConstraints>>
```

**Usage:**

```rust
let old = UpdateUserFieldsDiffParams {
    name: "Alice".to_string(),
    email: "alice@example.com".to_string(),
};
let new = UpdateUserFieldsDiffParams {
    name: "Alicia".to_string(),  // Changed
    email: "alice@example.com".to_string(),  // Same
};
update_user_fields_diff(executor, &old, &new, 42).await?;
// Only executes: UPDATE users SET updated_at = NOW(), name = $1 WHERE id = $2
```

**How It Works:**
- The struct contains only conditional parameters (those ending with `?`)
- Non-conditional parameters remain as individual function parameters
- At runtime, the function compares `old.field != new.field`
- Only clauses where the field differs are included in the query

**Struct Reuse:**

```yaml
queries:
  - name: update_user_profile_diff
    sql: "UPDATE users SET updated_at = NOW() $[, name = ${name?}] $[, email = ${email?}] WHERE id = ${user_id}"
    conditions_type: true
  
  - name: update_user_metadata_diff
    sql: "UPDATE users SET updated_at = NOW() $[, name = ${name?}] $[, email = ${email?}] WHERE id = ${user_id}"
    conditions_type: "UpdateUserProfileDiffParams"  # Reuses existing diff struct
```

### return_type: Custom Return Type Names

Customize the name of return type structs (generated for multi-column SELECT queries) and enable struct reuse across queries.

**Basic Usage:**

```yaml
- name: get_user_summary
  sql: "SELECT id, name, email FROM users WHERE id = ${user_id}"
  return_type: "UserSummary"  # Custom name instead of GetUserSummaryItem
```

**Generated Code:**

```rust
#[derive(Debug, Clone)]
pub struct UserSummary {
    pub id: i32,
    pub name: String,
    pub email: String,
}

pub async fn get_user_summary(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    user_id: i32
) -> Result<UserSummary, super::ErrorReadOnly>
```

**Struct Reuse:**

Multiple queries returning the same columns can share the same struct:

```yaml
queries:
  - name: get_user_summary
    sql: "SELECT id, name, email FROM users WHERE id = ${user_id}"
    return_type: "UserSummary"  # Generates the struct
  
  - name: get_user_info_by_email
    sql: "SELECT id, name, email FROM users WHERE email = ${email}"
    return_type: "UserSummary"  # Reuses the struct
  
  - name: get_all_user_summaries
    sql: "SELECT id, name, email FROM users ORDER BY name"
    return_type: "UserSummary"  # Reuses the struct
```

Only one `UserSummary` struct is generated, shared by all three functions.

**Disable Custom Struct:**

Set to `false` to use the default `{QueryName}Item` naming:

```yaml
- name: get_user_count
  sql: "SELECT COUNT(*) as count FROM users"
  return_type: false  # Uses GetUserCountItem
```

### Cross-Struct Reuse

You can reuse struct names across queries. AutoModel will:
1. **Auto-generate** if the struct doesn't exist yet (from the first query that uses it)
2. **Reuse** if the struct already exists (from a previous query in the same module)
3. **Validate** that fields match exactly when reusing

```yaml
queries:
  # First use: generates UserInfo struct from return columns
  - name: get_user_info
    sql: "SELECT id, name, email FROM users WHERE id = ${user_id}"
    return_type: "UserInfo"
  
  # Second use: reuses existing UserInfo struct for parameters
  - name: update_user_info
    sql: "UPDATE users SET name = ${name}, email = ${email} WHERE id = ${id}"
    parameters_type: "UserInfo"  # Reuses the return type struct
```

**Usage:**

```rust
// Get user info
let user = get_user_info(executor, 42).await?;

// Modify and update using the same struct
let updated = UserInfo {
    name: "New Name".to_string(),
    ..user
};
update_user_info(executor, &updated).await?;
```

### Build-Time Validation

AutoModel validates struct field compatibility at build time:

1. **Auto-Generation**: If a named struct doesn't exist, AutoModel automatically generates it from the query
2. **Field Matching**: When reusing an existing struct, query parameters/columns must exactly match struct fields (names and types)
3. **Clear Error Messages**: Validation failures provide helpful guidance

**Example validation errors:**

```
Error: Query parameter 'age' not found in struct 'UserInfo'.
Available fields: id, name, email
```

```
Error: Type mismatch for parameter 'id' in struct 'UserInfo':
expected 'i64', but query requires 'i32'
```

### Struct Definition Sources

Structs can be generated from three sources:

1. **parameters_type: true** → `{QueryName}Params` (input parameters)
2. **conditions_type: true** → `{QueryName}Params` (conditional input parameters)
3. **return_type: "Name"** → Custom named struct (output columns)
4. **Multi-column SELECT** → `{QueryName}Item` (output columns, when return_type not specified)

### When to Use Each Option

**Use `parameters_type`:**
- Queries with 3+ parameters where individual params become unwieldy
- Building query parameters from existing structs or API input
- Reusing parameter sets with slight modifications
- Improving code organization and reducing function signature complexity

**Use `conditions_type`:**
- Conditional queries (`$[...]`) with state comparison logic
- UPDATE queries that should only modify changed fields
- SELECT queries with filters that should only apply when criteria changed
- Implementing PATCH-style REST endpoints
- Avoiding the verbosity of many `Option<T>` parameters

**Use `return_type`:**
- Multiple queries returning the same column structure
- Creating domain-specific struct names (e.g., `UserSummary` instead of `GetUserItem`)
- Reusing return types as input parameters for related queries
- Building consistent DTOs across your API

### Complete Example

```yaml
queries:
  # Define a common return type
  - name: get_user_summary
    sql: "SELECT id, name, email FROM users WHERE id = ${user_id}"
    return_type: "UserSummary"
  
  # Reuse it in other queries
  - name: search_users
    sql: "SELECT id, name, email FROM users WHERE name ILIKE ${pattern}"
    return_type: "UserSummary"
  
  # Use it as input parameters
  - name: update_user_contact
    sql: "UPDATE users SET name = ${name}, email = ${email} WHERE id = ${id}"
    parameters_type: "UserSummary"
  
  # Conditional update with custom struct
  - name: partial_update_user
    sql: "UPDATE users SET updated_at = NOW() $[, name = ${name?}] $[, email = ${email?}] WHERE id = ${user_id}"
    conditions_type: true  # Generates PartialUpdateUserParams
```

**Generated Code:**

```rust
// Single struct definition shared across queries
#[derive(Debug, Clone)]
pub struct UserSummary {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct PartialUpdateUserParams {
    pub name: String,
    pub email: String,
}

pub async fn get_user_summary(...) -> Result<UserSummary, super::ErrorReadOnly>
pub async fn search_users(...) -> Result<Vec<UserSummary>, super::ErrorReadOnly>
pub async fn update_user_contact(..., params: &UserSummary) -> Result<(), super::Error<UpdateUserContactConstraints>>
pub async fn partial_update_user(..., old: &PartialUpdateUserParams, new: &PartialUpdateUserParams, ...) -> Result<(), super::Error<PartialUpdateUserConstraints>>
```

### Notes

- **Auto-generation of named structs**: If a struct name is specified but doesn't exist yet, AutoModel generates it automatically
- **Struct reuse from previous queries**: You can reference structs generated by earlier queries in the same module
- **Exact field matching**: When reusing existing structs, all query parameters/columns must match struct fields exactly
- **No subset matching**: You cannot use a struct with extra fields; all fields must match
- **parameters_type ignored when conditions_type is enabled**: Diff-based queries already use structured parameters

## Batch Insert with UNNEST Pattern

AutoModel supports efficient batch inserts using PostgreSQL's `UNNEST` function, which allows you to insert multiple rows in a single query. This is much more efficient than inserting rows one at a time.

### Basic UNNEST Pattern

PostgreSQL's `UNNEST` function can expand multiple arrays into a set of rows:

```sql
INSERT INTO users (name, email, age)
SELECT * FROM UNNEST(
  ARRAY['Alice', 'Bob', 'Charlie'],
  ARRAY['alice@example.com', 'bob@example.com', 'charlie@example.com'],
  ARRAY[25, 30, 35]
)
RETURNING id, name, email, age, created_at;
```

### Using UNNEST with AutoModel

Define a batch insert query in your `queries.yaml`:

```yaml
- name: insert_users_batch
  sql: |
    INSERT INTO users (name, email, age)
    SELECT * FROM UNNEST(${name}::text[], ${email}::text[], ${age}::int4[])
    RETURNING id, name, email, age, created_at
  description: "Insert multiple users using UNNEST pattern"
  module: "users"
  expect: "multiple"
  multiunzip: true
```

**Key Points:**
- Use array parameters: `${name}::text[]`, `${email}::text[]`, etc.
- Include explicit type casts for proper type inference
- Set `expect: "multiple"` to return a vector of results
- Set `multiunzip: true` to enable the special batch insert mode

### The `multiunzip` Configuration Parameter

When `multiunzip: true` is set, AutoModel generates special code to handle batch inserts more ergonomically:

**Without `multiunzip`** (standard array parameters):
```rust
// You would need to pass separate arrays for each column
insert_users_batch(
    &client,
    vec!["Alice".to_string(), "Bob".to_string()],
    vec!["alice@example.com".to_string(), "bob@example.com".to_string()],
    vec![25, 30]
).await?;
```

**With `multiunzip: true`** (generates a record struct):
```rust
// AutoModel generates an InsertUsersBatchRecord struct
#[derive(Debug, Clone)]
pub struct InsertUsersBatchRecord {
    pub name: String,
    pub email: String,
    pub age: i32,
}

// Now you can pass a single vector of records
insert_users_batch(
    &client,
    vec![
        InsertUsersBatchRecord {
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            age: 25,
        },
        InsertUsersBatchRecord {
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            age: 30,
        },
    ]
).await?;
```

### How `multiunzip` Works

When `multiunzip: true` is enabled:

1. **Generates an input record struct** with fields matching your parameters
2. **Uses itertools::multiunzip()** to transform `Vec<Record>` into tuple of arrays `(Vec<name>, Vec<email>, Vec<age>)`
3. **Binds each array** to the corresponding SQL parameter

Generated function signature:
```rust
pub async fn insert_users_batch(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    items: Vec<InsertUsersBatchRecord>  // Single parameter instead of multiple arrays
) -> Result<Vec<InsertUsersBatchItem>, super::Error<InsertUsersBatchConstraints>>
```

Internal implementation:
```rust
use itertools::Itertools;

// Transform Vec<Record> into separate arrays
let (name, email, age): (Vec<_>, Vec<_>, Vec<_>) =
    items
        .into_iter()
        .map(|item| (item.name, item.email, item.age))
        .multiunzip();

// Bind each array to the query
let query = query.bind(name);
let query = query.bind(email);
let query = query.bind(age);
```

### Complete Example

**queries.yaml:**
```yaml
- name: insert_posts_batch
  sql: |
    INSERT INTO posts (title, content, author_id, published_at)
    SELECT * FROM UNNEST(
      ${title}::text[],
      ${content}::text[],
      ${author_id}::int4[],
      ${published_at}::timestamptz[]
    )
    RETURNING id, title, author_id, created_at
  description: "Batch insert multiple posts"
  module: "posts"
  expect: "multiple"
  multiunzip: true
```

**Usage:**
```rust
use crate::generated::posts::{insert_posts_batch, InsertPostsBatchRecord};

let posts = vec![
    InsertPostsBatchRecord {
        title: "First Post".to_string(),
        content: "Content 1".to_string(),
        author_id: 1,
        published_at: chrono::Utc::now(),
    },
    InsertPostsBatchRecord {
        title: "Second Post".to_string(),
        content: "Content 2".to_string(),
        author_id: 1,
        published_at: chrono::Utc::now(),
    },
];

let inserted = insert_posts_batch(&client, posts).await?;
println!("Inserted {} posts", inserted.len());

```
## Upsert Pattern (INSERT ... ON CONFLICT)

PostgreSQL's `ON CONFLICT` clause allows you to handle conflicts when inserting data, enabling "upsert" operations (insert if new, update if exists). AutoModel fully supports this pattern for both single-row and batch operations.

### Understanding EXCLUDED

In the `DO UPDATE` clause, `EXCLUDED` is a special table reference provided by PostgreSQL that contains the row that **would have been inserted** if there had been no conflict. This allows you to reference the attempted insert values.

```sql
INSERT INTO users (email, name, age)
VALUES ('alice@example.com', 'Alice', 25)
ON CONFLICT (email)
DO UPDATE SET
  name = EXCLUDED.name,      -- Use the name from the VALUES clause
  age = EXCLUDED.age,        -- Use the age from the VALUES clause
  updated_at = NOW()         -- Set updated_at to current timestamp
```

In this example:
- `EXCLUDED.name` refers to `'Alice'` (the value being inserted)
- `EXCLUDED.age` refers to `25` (the value being inserted)
- `users.name` and `users.age` refer to the existing row's values in the table

You can also mix both references:
```sql
-- Only update if the new age is greater than the existing age
DO UPDATE SET age = EXCLUDED.age WHERE users.age < EXCLUDED.age
```

### Single Row Upsert

Use `ON CONFLICT` to update existing rows when a conflict occurs:

**queries.yaml:**
```yaml
- name: upsert_user
  sql: |
    INSERT INTO users (email, name, age, profile)
    VALUES (${email}, ${name}, ${age}, ${profile})
    ON CONFLICT (email) 
    DO UPDATE SET 
      name = EXCLUDED.name,
      age = EXCLUDED.age,
      profile = EXCLUDED.profile,
      updated_at = NOW()
    RETURNING id, email, name, age, created_at, updated_at
  description: "Insert a new user or update if email already exists"
  module: "users"
  expect: "exactly_one"
  types:
    "profile": "crate::models::UserProfile"
```

**Usage:**
```rust
use crate::generated::users::upsert_user;
use crate::models::UserProfile;

// First insert - creates new user
let user = upsert_user(
    &client,
    "alice@example.com".to_string(),
    "Alice".to_string(),
    25,
    UserProfile { bio: "Developer".to_string() }
).await?;

// Second call with same email - updates existing user
let updated_user = upsert_user(
    &client,
    "alice@example.com".to_string(),
    "Alice Smith".to_string(),  // Updated name
    26,                          // Updated age
    UserProfile { bio: "Senior Developer".to_string() }
).await?;

// Same ID, but updated fields
assert_eq!(user.id, updated_user.id);
```

### Batch Upsert with UNNEST

Combine `UNNEST` with `ON CONFLICT` for efficient batch upserts:

**queries.yaml:**
```yaml
- name: upsert_users_batch
  sql: |
    INSERT INTO users (email, name, age)
    SELECT * FROM UNNEST(
      ${email}::text[],
      ${name}::text[],
      ${age}::int4[]
    )
    ON CONFLICT (email)
    DO UPDATE SET
      name = EXCLUDED.name,
      age = EXCLUDED.age,
      updated_at = NOW()
    RETURNING id, email, name, age, created_at, updated_at
  description: "Batch upsert users - insert new or update existing by email"
  module: "users"
  expect: "multiple"
  multiunzip: true
```

**Usage:**
```rust
use crate::generated::users::{upsert_users_batch, UpsertUsersBatchRecord};

let users = vec![
    UpsertUsersBatchRecord {
        email: "alice@example.com".to_string(),
        name: "Alice".to_string(),
        age: 25,
    },
    UpsertUsersBatchRecord {
        email: "bob@example.com".to_string(),
        name: "Bob".to_string(),
        age: 30,
    },
    UpsertUsersBatchRecord {
        email: "alice@example.com".to_string(),  // Duplicate - will update
        name: "Alice Updated".to_string(),
        age: 26,
    },
];

let results = upsert_users_batch(&client, users).await?;
// Returns 2 rows: Bob (new) and Alice (updated)
println!("Upserted {} users", results.len());
```

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

## Error Handling and Custom Error Types

AutoModel provides sophisticated error handling with automatic constraint extraction and type-safe error types. Different types of queries return different error types based on whether they can violate database constraints.

### Error Type Overview

AutoModel generates two types of error enums:

1. **`ErrorReadOnly`** - For SELECT queries that cannot violate constraints
2. **`Error<C>`** - For mutation queries (INSERT, UPDATE, DELETE) with constraint tracking

### ErrorReadOnly - For Read-Only Queries

All SELECT queries return `ErrorReadOnly`, a simple error enum without constraint violation variants:

**Generated Code:**
```rust
#[derive(Debug)]
pub enum ErrorReadOnly {
    Database(sqlx::Error),
    RowNotFound,
}

impl From<sqlx::Error> for ErrorReadOnly {
    fn from(err: sqlx::Error) -> Self {
        ErrorReadOnly::Database(err)
    }
}
```

**Example Usage:**
```yaml
- name: get_user_by_id
  sql: "SELECT id, name, email FROM users WHERE id = ${user_id}"
  expect: "exactly_one"
```

```rust
pub async fn get_user_by_id(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    user_id: i32
) -> Result<GetUserByIdItem, super::ErrorReadOnly>  // Returns ErrorReadOnly
```

### Error<C> - For Mutation Queries

Mutation queries (INSERT, UPDATE, DELETE) return `Error<C>` where `C` is a query-specific constraint enum. This provides type-safe handling of constraint violations.

### Automatic Constraint Extraction

AutoModel automatically extracts all constraints from your PostgreSQL database for each table referenced in mutation queries. This happens at build time by querying the PostgreSQL system catalogs.

**Extracted Constraint Information:**
- **Unique constraints** - Including primary keys and unique indexes
- **Foreign key constraints** - With referenced table and column information
- **Check constraints** - With constraint expression
- **NOT NULL constraints** - For columns that cannot be null

**Example:**
For a users table with:
```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    age INT CHECK (age >= 0),
    organization_id INT REFERENCES organizations(id)
);
```

AutoModel generates:
```rust
#[derive(Debug)]
pub enum InsertUserConstraints {
    UsersPkey,                    // PRIMARY KEY constraint
    UsersEmailKey,                // UNIQUE constraint on email
    UsersAgeCheck,                // CHECK constraint on age
    UsersOrganizationIdFkey,      // FOREIGN KEY to organizations
    UsersIdNotNull,               // NOT NULL constraint on id
    UsersEmailNotNull,            // NOT NULL constraint on email
}

impl TryFrom<ErrorConstraintInfo> for InsertUserConstraints {
    type Error = ();
    
    fn try_from(info: ErrorConstraintInfo) -> Result<Self, Self::Error> {
        match info.constraint_name.as_str() {
            "users_pkey" => Ok(InsertUserConstraints::UsersPkey),
            "users_email_key" => Ok(InsertUserConstraints::UsersEmailKey),
            "users_age_check" => Ok(InsertUserConstraints::UsersAgeCheck),
            "users_organization_id_fkey" => Ok(InsertUserConstraints::UsersOrganizationIdFkey),
            "users_id_not_null" => Ok(InsertUserConstraints::UsersIdNotNull),
            "users_email_not_null" => Ok(InsertUserConstraints::UsersEmailNotNull),
            _ => Err(()),  // Unknown constraints return error instead of panicking
        }
    }
}
```

The generic `Error<C>` type handles constraint violations gracefully:
```rust
pub enum Error<C: TryFrom<ErrorConstraintInfo>> {
    /// Contains Some(C) when constraint is recognized, None for unknown constraints
    /// The ErrorConstraintInfo always contains the raw constraint details from PostgreSQL
    ConstraintViolation(Option<C>, ErrorConstraintInfo),
    RowNotFound,
    PoolTimeout,
    InternalError(String, sqlx::Error),
}
```

### Custom Error Type Names with `error_type`

By default, AutoModel generates error type names based on the query name (e.g., `InsertUserConstraints`). You can customize this using the `error_type` configuration option.

**Basic Usage:**
```yaml
- name: insert_user
  sql: "INSERT INTO users (email, name, age) VALUES (${email}, ${name}, ${age}) RETURNING id"
  error_type: "UserError"  # Custom name instead of InsertUserConstraints
```

**Generated Code:**
```rust
#[derive(Debug)]
pub enum UserError {
    UsersPkey,
    UsersEmailKey,
    UsersAgeCheck,
    // ... other constraints
}

impl TryFrom<ErrorConstraintInfo> for UserError {
    type Error = ();
    fn try_from(info: ErrorConstraintInfo) -> Result<Self, Self::Error> {
        // ... conversion logic
    }
}

pub async fn insert_user(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    email: String,
    name: String,
    age: i32
) -> Result<i32, super::Error<UserError>>  // Uses custom UserError
```

### Error Type Reuse

Multiple queries that operate on the same table(s) can reuse the same error type. AutoModel validates at build time that the constraints match exactly.

**Example:**
```yaml
queries:
  # First query generates the error type
  - name: insert_user
    sql: "INSERT INTO users (email, name, age) VALUES (${email}, ${name}, ${age}) RETURNING id"
    error_type: "UserError"
  
  # Second query reuses the same error type
  - name: update_user_email
    sql: "UPDATE users SET email = ${email} WHERE id = ${user_id} RETURNING id"
    error_type: "UserError"  # Reuses UserError - constraints must match
  
  # Third query also reuses it
  - name: upsert_user
    sql: |
      INSERT INTO users (email, name, age) VALUES (${email}, ${name}, ${age})
      ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name, age = EXCLUDED.age
      RETURNING id
    error_type: "UserError"  # Reuses UserError
```

**Build-Time Validation:**

AutoModel ensures that when you reuse an error type:
1. The referenced error type exists (defined by a previous query)
2. The constraints extracted for the current query exactly match the constraints in the reused type
3. Both queries reference the same table(s)

## Supported PostgreSQL Types

AutoModel supports a comprehensive set of PostgreSQL types with automatic mapping to Rust types. All types support `Option<T>` for nullable columns.

### Boolean & Numeric Types

| PostgreSQL Type | Rust Type |
|----------------|-----------|
| `BOOL` | `bool` |
| `CHAR` | `i8` |
| `INT2` (SMALLINT) | `i16` |
| `INT4` (INTEGER) | `i32` |
| `INT8` (BIGINT) | `i64` |
| `FLOAT4` (REAL) | `f32` |
| `FLOAT8` (DOUBLE PRECISION) | `f64` |
| `NUMERIC`, `DECIMAL` | `rust_decimal::Decimal` |
| `OID`, `REGPROC`, `XID`, `CID` | `u32` |
| `XID8` | `u64` |
| `TID` | `(u32, u32)` |

### String & Text Types

| PostgreSQL Type | Rust Type |
|----------------|-----------|
| `TEXT` | `String` |
| `VARCHAR` | `String` |
| `CHAR(n)`, `BPCHAR` | `String` |
| `NAME` | `String` |
| `XML` | `String` |

### Binary & Bit Types

| PostgreSQL Type | Rust Type |
|----------------|-----------|
| `BYTEA` | `Vec<u8>` |
| `BIT`, `BIT(n)` | `bit_vec::BitVec` |
| `VARBIT` | `bit_vec::BitVec` |

### Date & Time Types

| PostgreSQL Type | Rust Type |
|----------------|-----------|
| `DATE` | `chrono::NaiveDate` |
| `TIME` | `chrono::NaiveTime` |
| `TIMETZ` | `sqlx::postgres::types::PgTimeTz` |
| `TIMESTAMP` | `chrono::NaiveDateTime` |
| `TIMESTAMPTZ` | `chrono::DateTime<chrono::Utc>` |
| `INTERVAL` | `sqlx::postgres::types::PgInterval` |

### Range Types

| PostgreSQL Type | Rust Type |
|----------------|-----------|
| `INT4RANGE` | `sqlx::postgres::types::PgRange<i32>` |
| `INT8RANGE` | `sqlx::postgres::types::PgRange<i64>` |
| `NUMRANGE` | `sqlx::postgres::types::PgRange<rust_decimal::Decimal>` |
| `TSRANGE` | `sqlx::postgres::types::PgRange<chrono::NaiveDateTime>` |
| `TSTZRANGE` | `sqlx::postgres::types::PgRange<chrono::DateTime<chrono::Utc>>` |
| `DATERANGE` | `sqlx::postgres::types::PgRange<chrono::NaiveDate>` |

### Multirange Types

| PostgreSQL Type | Rust Type |
|----------------|-----------|
| `INT4MULTIRANGE` | `serde_json::Value` |
| `INT8MULTIRANGE` | `serde_json::Value` |
| `NUMMULTIRANGE` | `serde_json::Value` |
| `TSMULTIRANGE` | `serde_json::Value` |
| `TSTZMULTIRANGE` | `serde_json::Value` |
| `DATEMULTIRANGE` | `serde_json::Value` |

### Network & Address Types

| PostgreSQL Type | Rust Type |
|----------------|-----------|
| `INET` | `std::net::IpAddr` |
| `CIDR` | `std::net::IpAddr` |
| `MACADDR` | `mac_address::MacAddress` |

### Geometric Types

| PostgreSQL Type | Rust Type |
|----------------|-----------|
| `POINT` | `sqlx::postgres::types::PgPoint` |
| `LINE` | `sqlx::postgres::types::PgLine` |
| `LSEG` | `sqlx::postgres::types::PgLseg` |
| `BOX` | `sqlx::postgres::types::PgBox` |
| `PATH` | `sqlx::postgres::types::PgPath` |
| `POLYGON` | `sqlx::postgres::types::PgPolygon` |
| `CIRCLE` | `sqlx::postgres::types::PgCircle` |

### JSON & Special Types

| PostgreSQL Type | Rust Type |
|----------------|-----------|
| `JSON` | `serde_json::Value` |
| `JSONB` | `serde_json::Value` |
| `JSONPATH` | `String` |
| `UUID` | `uuid::Uuid` |

### Array Types

All types support PostgreSQL arrays with automatic mapping to `Vec<T>`:

| PostgreSQL Array Type | Rust Type |
|----------------------|-----------|
| `BOOL[]` | `Vec<bool>` |
| `INT2[]`, `INT4[]`, `INT8[]` | `Vec<i16>`, `Vec<i32>`, `Vec<i64>` |
| `FLOAT4[]`, `FLOAT8[]` | `Vec<f32>`, `Vec<f64>` |
| `TEXT[]`, `VARCHAR[]` | `Vec<String>` |
| `BYTEA[]` | `Vec<Vec<u8>>` |
| `UUID[]` | `Vec<uuid::Uuid>` |
| `DATE[]`, `TIMESTAMP[]`, `TIMESTAMPTZ[]` | `Vec<chrono::NaiveDate>`, `Vec<chrono::NaiveDateTime>`, `Vec<chrono::DateTime<chrono::Utc>>` |
| `INT4RANGE[]`, `DATERANGE[]`, etc. | `Vec<sqlx::postgres::types::PgRange<T>>` |
| And many more... | See type mapping table above |

### Full-Text Search & System Types

| PostgreSQL Type | Rust Type |
|----------------|-----------|
| `TSQUERY` | `String` |
| `REGCONFIG`, `REGDICTIONARY`, `REGNAMESPACE`, `REGROLE`, `REGCOLLATION` | `u32` |
| `PG_LSN` | `u64` |
| `ACLITEM` | `String` |

### Custom Enum Types

PostgreSQL custom enums are automatically detected and mapped to generated Rust enums with proper encoding/decoding support. See the Configuration Options section for details on enum handling.

## Requirements

- PostgreSQL database (for actual code generation)
- Rust 1.70+
- tokio runtime

## License

MIT License - see LICENSE file for details.
