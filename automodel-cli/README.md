# AutoModel CLI

Command-line interface for AutoModel - generate typed Rust functions from YAML-defined SQL queries.

## Installation

```bash
# From workspace root
cargo build -p automodel-cli

# The binary will be at target/debug/automodel
```

## Usage

### Basic Commands

```bash
# Show help
automodel --help

# Validate a YAML file
automodel validate -f queries.yaml

# Generate code from YAML
automodel generate -d postgresql://localhost/mydb -f queries.yaml
```

### Validate Command

Validates YAML syntax, query names, and optionally SQL queries:

```bash
# Basic validation (syntax and query names only)
automodel validate -f queries.yaml

# Advanced validation with database connection (also validates SQL)
automodel validate -f queries.yaml -d postgresql://localhost/mydb
```

**Options:**
- `-f, --file <FILE>` - YAML file containing query definitions (required)
- `-d, --database-url <URL>` - PostgreSQL database URL for SQL validation (optional)

### Generate Command

Generates Rust code from YAML query definitions:

```bash
# Basic generation (outputs to queries.rs)
automodel generate -d postgresql://localhost/mydb -f queries.yaml

# Custom output file
automodel generate -d postgresql://localhost/mydb -f queries.yaml -o src/database.rs

# Dry run (preview without writing files)
automodel generate -d postgresql://localhost/mydb -f queries.yaml --dry-run
```

**Options:**
- `-d, --database-url <URL>` - PostgreSQL database URL (required)
- `-f, --file <FILE>` - YAML file containing query definitions (required)
- `-o, --output <FILE>` - Output file for generated code (optional, defaults to `<yaml_file>.rs`)
- `-m, --module <NAME>` - Module name for generated code (optional)
- `--dry-run` - Generate code but don't write to file (optional)

## Examples

### Example YAML File

```yaml
queries:
  - name: get_user_by_id
    sql: "SELECT id, name, email FROM users WHERE id = $1"
    description: "Retrieve a user by their ID"
    
  - name: list_active_users
    sql: "SELECT id, name FROM users WHERE active = true"
    description: "List all active users"
    
  - name: create_user
    sql: "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id"
    description: "Create a new user"
```

### Validation Output

```bash
$ automodel validate -f queries.yaml

AutoModel Query Validator
========================
YAML file: queries.yaml

✓ YAML file parsed successfully
✓ Found 3 queries
✓ All query names are valid Rust identifiers
  1. get_user_by_id: Retrieve a user by their ID
  2. list_active_users: List all active users
  3. create_user: Create a new user

✓ Validation completed
```

### Generation Output

```bash
$ automodel generate -d postgresql://localhost/mydb -f queries.yaml

AutoModel Code Generator
=======================
Database URL: postgresql://localhost/mydb
YAML file: queries.yaml

Loading queries from YAML file...
✓ Successfully loaded 3 queries
  1. get_user_by_id: Retrieve a user by their ID
  2. list_active_users: List all active users
  3. create_user: Create a new user

Connecting to database and generating code...
✓ Successfully generated Rust code
✓ Generated code written to: queries.rs

You can now include this file in your Rust project:
  mod queries;
```

## Error Handling

The CLI provides helpful error messages for common issues:

- **File not found**: Clear message when YAML file doesn't exist
- **Invalid YAML**: Syntax error details with line numbers
- **Invalid query names**: Lists invalid identifiers with suggestions
- **Database connection**: Network and authentication error details
- **SQL errors**: Database-specific error messages for invalid queries

## Integration

### CI/CD Pipeline

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
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Setup database
        run: |
          psql -h localhost -U postgres -d postgres -f schema.sql
        env:
          PGPASSWORD: postgres
          
      - name: Validate queries
        run: |
          cargo run -p automodel-cli -- validate -f queries.yaml -d postgresql://postgres:postgres@localhost/postgres
```

### Build Script Integration

```rust
// build.rs
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=queries.yaml");
    
    let output = Command::new("cargo")
        .args(&[
            "run", "-p", "automodel-cli", "--",
            "generate",
            "-d", &std::env::var("DATABASE_URL").unwrap_or_default(),
            "-f", "queries.yaml",
            "-o", "src/generated.rs"
        ])
        .output()
        .expect("Failed to run automodel-cli");
        
    if !output.status.success() {
        panic!("Query generation failed: {}", String::from_utf8_lossy(&output.stderr));
    }
}
```

## Development

### Building from Source

```bash
# Clone the repository
git clone <repository-url>
cd automodel

# Build the CLI
cargo build -p automodel-cli

# Run tests
cargo test -p automodel-lib

# Install locally (optional)
cargo install --path automodel-cli
```

### Contributing

See the main repository README for contribution guidelines.
