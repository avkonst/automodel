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

### Diff-Based Conditional Queries

For scenarios where you want to dynamically include clauses based on comparing old and new values, AutoModel provides the `conditional_diff` option. Instead of using `Option<T>` parameters, this generates a struct with the conditional fields and compares old vs new values to decide which clauses to include.

This feature works with **any query type** (SELECT, UPDATE, DELETE, etc.), not just UPDATEs.

**Configuration Example (UPDATE):**

```yaml
- name: update_user_fields_diff
  sql: "UPDATE users SET updated_at = NOW() $[, name = ${name?}] $[, email = ${email?}] $[, age = ${age?}] WHERE id = ${user_id} RETURNING id, name, email, age, updated_at"
  description: "Update user fields using diff-based conditional updates - compares old and new structs"
  conditional_diff: true
```

**Configuration Example (SELECT):**

```yaml
- name: search_users_diff
  sql: "SELECT id, name, email, age FROM users WHERE 1=1 $[AND name ILIKE ${name_pattern?}] $[AND email ILIKE ${email_pattern?}] $[AND age >= ${min_age?}] ORDER BY created_at DESC"
  description: "Search users by comparing filter criteria changes"
  conditional_diff: true
```

**Generated Struct and Function:**

```rust
pub struct UpdateUserFieldsDiffParams {
    pub name: String,
    pub email: String,
    pub age: i32,
}

pub async fn update_user_fields_diff(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    old: &UpdateUserFieldsDiffParams,
    new: &UpdateUserFieldsDiffParams,
    user_id: i32
) -> Result<UpdateUserFieldsDiffItem, sqlx::Error>
```

**Usage Examples:**

```rust
// Update only the name (by passing different old/new values)
let old = UpdateUserFieldsDiffParams {
    name: "Alice Cooper".to_string(),
    email: "alice@example.com".to_string(),
    age: 28,
};
let new = UpdateUserFieldsDiffParams {
    name: "Alice Smith".to_string(),  // Changed
    email: "alice@example.com".to_string(),  // Same
    age: 28,  // Same
};
update_user_fields_diff(executor, &old, &new, user_id).await?;
// SQL: "UPDATE users SET updated_at = NOW(), name = $1 WHERE id = $2 RETURNING ..."
// Params: ["Alice Smith", user_id]

// Update multiple fields
let old = UpdateUserFieldsDiffParams {
    name: "Alice Smith".to_string(),
    email: "alice@example.com".to_string(),
    age: 28,
};
let new = UpdateUserFieldsDiffParams {
    name: "Alicia Smith".to_string(),  // Changed
    email: "alicia@example.com".to_string(),  // Changed
    age: 28,  // Same
};
update_user_fields_diff(executor, &old, &new, user_id).await?;
// SQL: "UPDATE users SET updated_at = NOW(), name = $1, email = $2 WHERE id = $3 RETURNING ..."
// Params: ["Alicia Smith", "alicia@example.com", user_id]
```

**SELECT Query Example:**

```rust
// Generated for search_users_diff query
pub struct SearchUsersDiffParams {
    pub name_pattern: String,
    pub email_pattern: String,
    pub min_age: i32,
}

// Search with only name filter changed
let old_filters = SearchUsersDiffParams {
    name_pattern: "%smith%".to_string(),
    email_pattern: "%@example.com%".to_string(),
    min_age: 18,
};
let new_filters = SearchUsersDiffParams {
    name_pattern: "%johnson%".to_string(),  // Changed
    email_pattern: "%@example.com%".to_string(),  // Same
    min_age: 18,  // Same
};
search_users_diff(executor, &old_filters, &new_filters).await?;
// SQL: "SELECT id, name, email, age FROM users WHERE 1=1 AND name ILIKE $1 ORDER BY created_at DESC"
// Params: ["%johnson%"]

// Search with all filters changed
let old_filters = SearchUsersDiffParams {
    name_pattern: "%johnson%".to_string(),
    email_pattern: "%@example.com%".to_string(),
    min_age: 18,
};
let new_filters = SearchUsersDiffParams {
    name_pattern: "%williams%".to_string(),  // Changed
    email_pattern: "%@company.com%".to_string(),  // Changed
    min_age: 25,  // Changed
};
search_users_diff(executor, &old_filters, &new_filters).await?;
// SQL: "SELECT id, name, email, age FROM users WHERE 1=1 AND name ILIKE $1 AND email ILIKE $2 AND age >= $3 ORDER BY created_at DESC"
// Params: ["%williams%", "%@company.com%", 25]
```

**How It Works:**
- The struct contains only the conditional parameters (those ending with `?`)
- Non-conditional parameters remain as individual function parameters
- At runtime, the function compares `old.field != new.field` for each conditional parameter
- Only clauses where the field differs between old and new are included
- Values from the `new` struct are bound to the query parameters

**When to Use:**
- When you have a "current state" and want to apply changes dynamically
- For SELECT queries with filters that should only apply when criteria changed
- For UPDATE queries that should only modify changed fields
- When building APIs that track state changes (e.g., comparing previous filter vs new filter)
- When you want to avoid the verbosity of many `Option<T>` parameters
- When implementing PATCH-style REST endpoints that update only modified fields
- For any conditional query where diff-based logic is clearer than Option-based logic

### Parameter Binding and Performance

- **Sequential parameter binding**: AutoModel automatically renumbers parameters to ensure sequential binding ($1, $2, $3, etc.)
- **No SQL parsing overhead**: Parameter renumbering happens at function generation time, not runtime
- **Prepared statement compatibility**: Generated SQL is fully compatible with PostgreSQL prepared statements
- **Type safety**: All parameter types are validated at compile time

### Limitations

- **No nested conditionals**: `$[...]` blocks cannot be nested inside other conditional blocks
- **Parameter uniqueness**: Each optional parameter can only be used once per conditional block

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
) -> Result<Vec<InsertUsersBatchItem>, sqlx::Error>
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
