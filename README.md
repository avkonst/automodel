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
- ✅ Support for all common PostgreSQL types
- 🏗️ Generate result structs for multi-column queries
- ⚡ Build-time code generation with automatic regeneration when YAML changes
- 🎯 Advanced CLI with dry-run and flexible output options

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

## Configuration options


## CLI Features

### Commands

- **`generate`** - Generate Rust code from YAML definitions

### CLI Options

#### Validate Command
- `-f, --file <FILE>` - YAML file to validate
- `-d, --database-url <URL>` - (Optional) Database URL for SQL validation

#### Generate Command
- `-d, --database-url <URL>` - Database connection URL
- `-f, --file <FILE>` - YAML file with query definitions
- `-o, --output <FILE>` - Custom output file path
- `-m, --module <NAME>` - Module name for generated code
- `--dry-run` - Preview generated code without writing files

## Build-time vs Runtime Code Generation

### Build-time (Recommended)
- Code is generated during `cargo build`
- Zero runtime overhead
- Type-safe at compile time
- Automatically regenerates when YAML changes
- Works even if database is unavailable at runtime

### Runtime
- Use the library directly in your application
- Requires database connection at startup
- More flexible for dynamic scenarios

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
