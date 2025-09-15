# AutoModel CLI

🔧 **Command-line interface for AutoModel - Generate type-safe Rust functions from SQL queries**

The AutoModel CLI provides powerful tools for validating SQL query definitions and generating type-safe Rust code. Perfect for CI/CD pipelines, development workflows, and standalone code generation.

## ✨ Features

- 📝 **YAML Validation** - Validate syntax, query names, and SQL queries
- 🛠️ **Code Generation** - Generate type-safe Rust functions with full type checking
- 🔌 **PostgreSQL Integration** - Full support for PostgreSQL types including enums
- 🎯 **Advanced Options** - Dry-run, custom output, module organization
- ⚡ **CI/CD Ready** - Perfect for automated validation and code generation
- 🏗️ **Module Support** - Generate organized code with separate modules

## 🚀 Installation

### From Source
```bash
git clone <repository-url>
cd automodel
cargo install --path automodel-cli
```

### Using Cargo (when published)
```bash
cargo install automodel-cli
```

## 📋 Commands

### Validate Queries

Validate YAML syntax, query names, and optionally SQL queries against your database:

```bash
# Basic validation (syntax and query names only)
automodel validate -f queries.yaml

# Full validation with database connection (validates SQL)
automodel validate -f queries.yaml -d postgresql://localhost/mydb
```

**Options:**
- `-f, --file <FILE>` - YAML file to validate (required)
- `-d, --database-url <URL>` - PostgreSQL database URL for SQL validation (optional)

### Generate Code

Generate type-safe Rust functions from your query definitions:

```bash
# Basic generation
automodel generate -d postgresql://localhost/mydb -f queries.yaml

# Custom output file
automodel generate -d postgresql://localhost/mydb -f queries.yaml -o src/database.rs

# Dry run (preview without writing files)
automodel generate -d postgresql://localhost/mydb -f queries.yaml --dry-run
```

**Options:**
- `-d, --database-url <URL>` - PostgreSQL database URL (required)
- `-f, --file <FILE>` - YAML file containing query definitions (required)  
- `-o, --output <FILE>` - Output file for generated code (optional, defaults to `generated.rs`)
- `-m, --module <NAME>` - Root module name for generated code (optional)
- `--dry-run` - Generate code but don't write to file (optional)

## 📄 Query Definition Format

### Basic Structure

```yaml
queries:
  - name: get_user_by_id
    sql: "SELECT id, name, email, created_at FROM users WHERE id = ${user_id}"
    description: "Get a user by their ID"
    module: "users"
    expect: "exactly_one"
  
  - name: find_users_by_name
    sql: "SELECT id, name FROM users WHERE name ILIKE ${pattern} AND (${min_age?} IS NULL OR age >= ${min_age?})"
    description: "Find users by name with optional age filter"
    module: "users"
    expect: "multiple"
    
  - name: get_user_profile
    sql: "SELECT id, name, profile FROM users WHERE id = ${user_id}"
    description: "Get user with JSON profile data"
    module: "users"
    expect: "possible_one"
    types:
      profile: "crate::models::UserProfile"
```

### Advanced Features

- **Named Parameters**: Use `${param_name}` instead of `$1`, `$2`
- **Optional Parameters**: Use `${param?}` for optional parameters
- **Custom Types**: Map JSON/JSONB fields to custom Rust types
- **Module Organization**: Organize functions into separate modules
- **Return Types**: Control result handling with `expect` field
- **PostgreSQL Enums**: Automatic detection and generation of Rust enums

### expect Field Options

- `exactly_one` - Returns `Result<T, Error>`, fails if 0 or >1 rows
- `possible_one` - Returns `Result<Option<T>, Error>`, None if no rows  
- `multiple` - Returns `Result<Vec<T>, Error>`, empty Vec if no rows
- `at_least_one` - Returns `Result<Vec<T>, Error>`, fails if no rows

## 💻 Examples

### Validation Example

```bash
$ automodel validate -f queries.yaml -d postgresql://localhost/mydb

AutoModel Query Validator
========================
YAML file: queries.yaml
Database: postgresql://localhost/mydb

✅ YAML file parsed successfully
✅ Found 4 queries across 2 modules
✅ All query names are valid Rust identifiers
✅ Database connection successful
✅ PostgreSQL enums detected: user_status, post_type
✅ All SQL queries validated successfully

Query Summary:
📁 users module (2 queries):
  1. get_user_by_id: Get a user by their ID
  2. find_users_by_name: Find users by name with optional age filter

📁 posts module (2 queries):  
  3. get_user_posts: Get all posts for a user
  4. create_post: Create a new post

✅ Validation completed - All queries are valid!
```

### Generation Example

```bash
$ automodel generate -d postgresql://localhost/mydb -f queries.yaml

AutoModel Code Generator  
=======================
Database: postgresql://localhost/mydb
YAML file: queries.yaml

Loading queries from YAML...
✅ Successfully loaded 4 queries across 2 modules

Connecting to database...
✅ Database connection established
✅ PostgreSQL enums detected and processed:
   • UserStatus (active, inactive, suspended, pending)
   • PostType (article, tutorial, announcement)

Analyzing queries and generating code...
✅ Type analysis completed for all queries
✅ Generated 4 type-safe functions
✅ Generated 2 result structs  
✅ Generated 2 PostgreSQL enums

📁 Generated files:
   • src/generated/mod.rs (main module + enums)
   • src/generated/users.rs (users module)
   • src/generated/posts.rs (posts module)

✅ Code generation completed successfully!

Usage in your Rust code:
   mod generated;
   
   // Use generated functions
   let user = generated::users::get_user_by_id(client, 42).await?;
   let posts = generated::posts::get_user_posts(client, 42, PostType::Article).await?;
```

### Dry Run Example

```bash
$ automodel generate -d postgresql://localhost/mydb -f queries.yaml --dry-run

AutoModel Code Generator (Dry Run)
==================================
This is a preview of the code that would be generated.
No files will be written.

// ===== src/generated/mod.rs =====

//! Auto-generated database functions
//! Generated from queries.yaml

pub mod users;
pub mod posts;

use tokio_postgres::{types::ToSql, Client, Error, Row};

// PostgreSQL enum: UserStatus
#[derive(Debug, Clone, PartialEq, Eq)]  
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    Pending,
}

// ... (rest of generated code preview)

✅ Dry run completed - 287 lines of code would be generated
```

## 🔧 Integration Examples

### CI/CD Pipeline (GitHub Actions)

```yaml
# .github/workflows/validate-queries.yml
name: Validate SQL Queries
on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Setup database schema
        run: |
          psql -h localhost -U postgres -d postgres -f schema.sql
        env:
          PGPASSWORD: postgres
          
      - name: Install AutoModel CLI
        run: cargo install --path automodel-cli
          
      - name: Validate queries
        run: |
          automodel validate -f queries.yaml -d postgresql://postgres:postgres@localhost/postgres
```

### Build Script Integration

```rust
// build.rs
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=queries.yaml");
    
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        let output = Command::new("automodel")
            .args([
                "generate",
                "-d", &database_url,
                "-f", "queries.yaml", 
                "-o", "src/generated.rs"
            ])
            .output()
            .expect("Failed to run automodel CLI");
            
        if !output.status.success() {
            panic!("Query generation failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        
        println!("✅ Generated database functions");
    } else {
        println!("⚠️ DATABASE_URL not set, skipping code generation");
    }
}
```

### Development Workflow

```bash
# 1. Create/edit your queries
vim queries.yaml

# 2. Validate queries during development
automodel validate -f queries.yaml -d $DATABASE_URL

# 3. Generate code when ready
automodel generate -f queries.yaml -d $DATABASE_URL -o src/generated.rs

# 4. Build your project
cargo build
```

### Makefile Integration

```makefile
# Makefile
.PHONY: validate-queries generate-code

validate-queries:
	automodel validate -f queries.yaml -d $(DATABASE_URL)

generate-code:
	automodel generate -f queries.yaml -d $(DATABASE_URL) -o src/generated.rs

build: validate-queries generate-code
	cargo build

check: validate-queries
	cargo check
```

## 🚨 Error Handling

The CLI provides detailed error messages for common issues:

### File Errors
```bash
❌ Error: YAML file not found: queries.yaml
   Help: Check that the file path is correct
```

### YAML Syntax Errors  
```bash
❌ YAML parsing error at line 5, column 8:
   unexpected character '}'
   
   4 |   - name: get_user
   5 |     sql: "SELECT }" 
                        ^
   6 |     description: "..."
```

### Invalid Query Names
```bash
❌ Validation failed: Invalid query names
   • Query 'get-user-by-id' (line 3): Contains invalid character '-'
     Suggestion: Use 'get_user_by_id' instead
   • Query '123_users' (line 8): Cannot start with a number
     Suggestion: Use 'list_123_users' instead
```

### Database Connection Errors
```bash
❌ Database connection failed: 
   connection to server at "localhost" (127.0.0.1), port 5432 failed: 
   FATAL: database "nonexistent" does not exist
   
   Help: Check your database URL and ensure the database exists
```

### SQL Validation Errors
```bash
❌ SQL validation failed for query 'get_user_by_id':
   column "ide" does not exist at character 8
   
   Query: SELECT ide, name FROM users WHERE id = $1
                 ^^^
   
   Help: Check your column names against your database schema
```

## 🛠️ Development

### Building from Source

```bash
# Clone the repository
git clone <repository-url>
cd automodel

# Build the CLI
cargo build -p automodel-cli --release

# Run tests
cargo test -p automodel-lib

# Install locally
cargo install --path automodel-cli
```

### Testing

```bash
# Test CLI commands
cargo test -p automodel-cli

# Test with real database (requires DATABASE_URL)
DATABASE_URL=postgresql://localhost/test_db cargo test
```

## 📄 License

MIT License - see LICENSE file for details.
