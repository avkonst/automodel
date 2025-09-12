use automodel::*;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get database URL from environment or use default
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/test".to_string());

    println!("Connecting to database: {}", database_url);

    // Create AutoModel instance
    let mut automodel = AutoModel::new(database_url);

    // Load queries from YAML file
    println!("Loading queries from YAML file...");
    automodel.load_queries_from_file("examples/user_queries.yaml").await?;

    println!("Found {} queries:", automodel.queries().len());
    for query in automodel.queries() {
        println!("  - {}: {}", query.name, query.description.as_deref().unwrap_or("No description"));
    }

    // Generate Rust code
    println!("\nGenerating Rust code...");
    match automodel.generate_code().await {
        Ok(code) => {
            println!("Generated code:");
            println!("{}", code);

            // Write the generated code to a file
            tokio::fs::write("examples/generated_functions.rs", &code).await?;
            println!("\nGenerated code written to examples/generated_functions.rs");
        }
        Err(e) => {
            eprintln!("Error generating code: {}", e);
            eprintln!("This might be because:");
            eprintln!("1. The database is not running");
            eprintln!("2. The database schema doesn't match the queries");
            eprintln!("3. Connection parameters are incorrect");
        }
    }

    Ok(())
}
