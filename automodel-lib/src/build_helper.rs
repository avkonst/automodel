use crate::AutoModel;
use std::env;
use std::fs;

/// Build script helper for automatically generating code at build time.
///
/// This function should be called from your build.rs script. It will:
/// - Check if DATABASE_URL environment variable is set
/// - If set, generate code and fail the build if something goes wrong
/// - If not set, skip code generation silently (letting compilation fail if generated code is used)
///
/// # Arguments
///
/// * `yaml_file` - Path to the YAML file containing query definitions (relative to build.rs)
/// * `output_file` - Path to write the generated Rust code (relative to build.rs, typically "src/generated.rs")
///
/// # Example
///
/// ```rust,no_run
/// // build.rs
/// use automodel::generate_at_build_time;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     generate_at_build_time("queries.yaml", "src/generated.rs").await?;
///     
///     Ok(())
/// }
/// ```
pub async fn generate_at_build_time(
    yaml_file: &str,
    output_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Tell cargo to rerun if the input YAML file changes
    println!("cargo:rerun-if-changed={}", yaml_file);
    // Tell cargo to rerun if the output file is manually modified
    println!("cargo:rerun-if-changed={}", output_file);

    // Check if DATABASE_URL environment variable is set
    match env::var("DATABASE_URL") {
        Ok(database_url) => {
            // DATABASE_URL is set - generate code and fail build if something goes wrong
            println!("cargo:info=DATABASE_URL found, generating database functions...");
            generate_code(&database_url, yaml_file, output_file).await?;
            println!("cargo:info=Successfully generated database functions");
        }
        Err(_) => {
            // DATABASE_URL is not set - skip codegen silently
            println!("cargo:info=DATABASE_URL not set, skipping code generation");
            // Don't generate any code - let the app fail compilation if it tries to use generated functions
        }
    }

    Ok(())
}

async fn generate_code(
    database_url: &str,
    yaml_file: &str,
    output_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create AutoModel instance
    let mut automodel = AutoModel::new(database_url.to_string());

    // Load queries from YAML file
    automodel.load_queries_from_file(yaml_file).await?;

    // Generate Rust code
    let generated_code = automodel.generate_code().await?;

    // Write the generated code to the specified output file
    fs::write(output_file, &generated_code)?;

    Ok(())
}
