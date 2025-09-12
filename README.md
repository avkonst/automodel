# AutoModel

🚀 **Generate type-safe Rust functions from SQL queries with zero runtime overhead**

AutoModel is a powerful Rust library that automatically generates strongly-typed database functions from YAML-defined SQL queries. Perfect for PostgreSQL applications that want compile-time type safety without the complexity of full ORMs.

## ✨ Features

- 📝 **YAML Query Definitions** - Define SQL queries with names, descriptions, and parameters
- � **PostgreSQL Integration** - Full support for PostgreSQL types including enums and custom types
- 🛠️ **Build-time Code Generation** - Zero runtime overhead with compile-time type checking
- 🏗️ **Modular Organization** - Organize queries into separate modules for better code structure
- 🎯 **Advanced Type Support** - PostgreSQL enums, JSON/JSONB with custom types, optional parameters
- ⚡ **Smart Query Patterns** - Control return types (exactly_one, possible_one, multiple, at_least_one)
- 🔧 **Named Parameters** - Use `${param_name}` instead of positional parameters
- ✅ **Type Safety** - Catch type mismatches at compile time, not runtime

## 🚀 Quick Start

### 1. Add to Your Project

```toml
[build-dependencies]
automodel = "0.1"
tokio = { version = "1.0", features = ["rt"] }
anyhow = "1.0"

[dependencies]
tokio-postgres = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

### 2. Create a build.rs

```rust
use automodel::AutoModel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    AutoModel::generate_at_build_time("queries.yaml", "src/generated").await?;
    Ok(())
}
```

### 3. Define Your Queries

Create `queries.yaml`:

```yaml
queries:
  # Simple query with named parameter
  - name: get_user_by_id
    sql: "SELECT id, name, email, created_at FROM users WHERE id = ${user_id}"
    description: "Get a user by their ID"
    module: "users"
    expect: "exactly_one"
  
  # Query with optional parameter and custom return type  
  - name: find_users_by_name
    sql: "SELECT id, name, email FROM users WHERE name ILIKE ${pattern} AND (${min_age?} IS NULL OR age >= ${min_age?})"
    description: "Find users by name pattern with optional age filter"
    module: "users" 
    expect: "multiple"
  
  # Query with custom JSON type mapping
  - name: get_user_profile
    sql: "SELECT id, name, profile FROM users WHERE id = ${user_id}"
    description: "Get user with their profile data"
    module: "users"
    expect: "possible_one"
    types:
      profile: "crate::models::UserProfile"
  
  # PostgreSQL enum support
  - name: get_active_users
    sql: "SELECT id, name, status FROM users WHERE status = ${status}"
    description: "Get users by status"
    module: "users"
    expect: "multiple"
```

### 4. Use Generated Functions

```rust
mod generated;

use tokio_postgres::Client;

async fn example(client: &Client) -> Result<(), tokio_postgres::Error> {
    // Type-safe function calls with proper error handling
    let user = generated::users::get_user_by_id(client, 42).await?;
    
    // Optional parameters
    let users = generated::users::find_users_by_name(client, "%john%".to_string(), Some(18)).await?;
    
    // Custom types and nullable results
    if let Some(profile) = generated::users::get_user_profile(client, 42).await? {
        println!("User profile: {:?}", profile.profile);
    }
    
    Ok(())
}
```

## 📋 Query Configuration

### Basic Query Structure

```yaml
queries:
  - name: function_name           # Required: Rust function name
    sql: "SELECT ..."            # Required: SQL query with ${params}
    description: "..."           # Optional: Function documentation
    module: "module_name"        # Optional: Module to generate function in
    expect: "exactly_one"        # Optional: Return type pattern
    types:                       # Optional: Custom type mappings
      field_name: "CustomType"
```

### expect Field Options

Control how results are returned and what happens when no rows are found:

- **`exactly_one`** (default) - Returns `Result<T, Error>`, fails if 0 or >1 rows
- **`possible_one`** - Returns `Result<Option<T>, Error>`, None if no rows
- **`multiple`** - Returns `Result<Vec<T>, Error>`, empty Vec if no rows  
- **`at_least_one`** - Returns `Result<Vec<T>, Error>`, fails if no rows

### Named Parameters

Use descriptive parameter names instead of positional:

```yaml
sql: "SELECT * FROM users WHERE age >= ${min_age} AND city = ${city}"
# Generates: function_name(client: &Client, min_age: i32, city: String)
```

### Optional Parameters

Mark parameters as optional with `?`:

```yaml
sql: "SELECT * FROM users WHERE name = ${name} AND (${age?} IS NULL OR age = ${age?})"
# Generates: function_name(client: &Client, name: String, age: Option<i32>)
```

### Custom Type Mappings

Map JSON/JSONB fields to custom Rust types:

```yaml
queries:
  - name: get_user_with_profile
    sql: "SELECT id, name, profile, settings FROM users WHERE id = ${id}"
    types:
      profile: "crate::models::UserProfile"
      settings: "crate::models::UserSettings"
```

## 🏗️ Module Organization

Organize your functions into separate modules for better code structure:

```yaml
queries:
  # Users module (generates src/generated/users.rs)
  - name: get_user
    module: "users"
    sql: "..."
  
  # Admin module (generates src/generated/admin.rs)  
  - name: get_system_info
    module: "admin"
    sql: "..."
    
  # Main module (generates src/generated/mod.rs)
  - name: health_check
    sql: "..."
```

Generated structure:
```
src/generated/
├── mod.rs          # Main module functions
├── users.rs        # Users module functions
└── admin.rs        # Admin module functions
```

## 🎯 PostgreSQL Enum Support

AutoModel automatically detects and generates Rust enums for PostgreSQL enum types:

```sql
-- Database schema
CREATE TYPE user_status AS ENUM ('active', 'inactive', 'suspended', 'pending');
```

```yaml
# queries.yaml - AutoModel automatically detects the enum
queries:
  - name: get_users_by_status
    sql: "SELECT id, name, status FROM users WHERE status = ${user_status}"
    expect: "multiple"
```

```rust
// Generated Rust code
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserStatus {
    Active,
    Inactive, 
    Suspended,
    Pending,
}

// With full trait implementations for database integration
impl FromStr for UserStatus { ... }
impl Display for UserStatus { ... }
impl FromSql<'_> for UserStatus { ... }
impl ToSql for UserStatus { ... }

// Type-safe function
pub async fn get_users_by_status(
    client: &tokio_postgres::Client, 
    user_status: UserStatus
) -> Result<Vec<GetUsersByStatusResult>, tokio_postgres::Error>
```

## 📊 Supported PostgreSQL Types

| PostgreSQL Type | Rust Type | Notes |
|----------------|-----------|-------|
| `BOOL` | `bool` | |
| `INT2` | `i16` | |
| `INT4` | `i32` | |
| `INT8` | `i64` | |
| `FLOAT4` | `f32` | |
| `FLOAT8` | `f64` | |
| `TEXT`, `VARCHAR` | `String` | |
| `BYTEA` | `Vec<u8>` | |
| `TIMESTAMP` | `chrono::NaiveDateTime` | |
| `TIMESTAMPTZ` | `chrono::DateTime<chrono::Utc>` | |
| `DATE` | `chrono::NaiveDate` | |
| `TIME` | `chrono::NaiveTime` | |
| `UUID` | `uuid::Uuid` | |
| `JSON`, `JSONB` | `serde_json::Value` | Or custom types with `types` mapping |
| `INET` | `std::net::IpAddr` | |
| `NUMERIC` | `rust_decimal::Decimal` | |
| **Custom ENUMs** | **Generated Rust enums** | **Automatic detection & generation** |

All types support `Option<T>` for nullable columns.

## 🔧 Advanced Features

### Custom JSON Types

Map JSON/JSONB fields to your own types:

```rust
// Define your types
#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub preferences: UserPreferences,
}
```

```yaml
# Map in queries.yaml
queries:
  - name: get_user_profile
    sql: "SELECT id, profile FROM users WHERE id = ${id}"
    types:
      profile: "crate::models::UserProfile"
```

### Complex Queries

```yaml
queries:
  # Joins and complex queries work seamlessly
  - name: get_user_with_posts
    sql: |
      SELECT u.id, u.name, p.title, p.content, p.created_at
      FROM users u
      LEFT JOIN posts p ON u.id = p.user_id  
      WHERE u.id = ${user_id}
      ORDER BY p.created_at DESC
    expect: "multiple"
    
  # Aggregations
  - name: get_user_stats
    sql: |
      SELECT 
        u.name,
        COUNT(p.id) as post_count,
        MAX(p.created_at) as last_post_date
      FROM users u
      LEFT JOIN posts p ON u.id = p.user_id
      WHERE u.id = ${user_id}
      GROUP BY u.id, u.name
    expect: "exactly_one"
```

## 🏃‍♂️ Example Projects

Check out the `example-app/` directory for a complete working example that demonstrates:

- Build-time code generation with `build.rs`
- Module organization
- PostgreSQL enum support  
- Custom JSON type mappings
- All query patterns (exactly_one, possible_one, multiple, at_least_one)

## 🛠️ CLI Tool

AutoModel includes a powerful CLI for validation and code generation:

```bash
# Install CLI
cargo install --path automodel-cli

# Validate queries
automodel validate -f queries.yaml -d postgresql://localhost/mydb

# Generate code  
automodel generate -f queries.yaml -d postgresql://localhost/mydb -o generated.rs

# See all options
automodel --help
```

## 🤝 Contributing

We welcome contributions! Please feel free to submit issues and pull requests.

## 📄 License

MIT License - see LICENSE file for details.
