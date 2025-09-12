mod generated;
mod models;

use std::env;
use tokio_postgres::{Client, NoTls};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get database URL from environment
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "postgresql://localhost/test".to_string());

    println!("Example App - Using AutoModel Generated Functions");
    println!("==============================================");

    // Connect to database
    match connect_to_database(&database_url).await {
        Ok(client) => {
            println!("✓ Connected to database");
            run_examples(&client).await?;
        }
        Err(e) => {
            println!("✗ Failed to connect to database: {}", e);
            println!("To run this example:");
            println!("1. Start a PostgreSQL database");
            println!("2. Run the schema.sql file to create tables");
            println!("3. Set DATABASE_URL environment variable");
            println!("4. Run: cargo run");
        }
    }

    Ok(())
}

async fn connect_to_database(database_url: &str) -> Result<Client, Box<dyn std::error::Error>> {
    let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;

    // Spawn the connection in the background
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Database connection error: {}", e);
        }
    });

    Ok(client)
}

async fn run_examples(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nRunning example queries...");

    // Note: The actual generated functions would be used here
    // For now, we'll show how they would be called with the new module structure:

    println!("Generated functions organized by modules:");
    println!("Admin module:");
    println!("- generated::admin::get_current_time(client)");
    println!("- generated::admin::get_version(client)");
    println!("\nSetup module:");
    println!("- generated::setup::create_users_table(client)");
    println!("\nUsers module:");
    println!("- generated::users::insert_user(client, name, email, age, profile)");
    println!("- generated::users::get_all_users(client)");
    println!("- generated::users::find_user_by_email(client, email)");
    println!("- generated::users::update_user_profile(client, profile, user_id)");
    println!("\nMain module (queries without module specified):");
    println!("- generated::test_json_query(client, test_data)");

    // Example of how you would use the generated functions:

    // Admin functions
    match generated::admin::get_current_time(client).await {
        Ok(time) => println!("Current time: {:?}", time),
        Err(e) => println!("Error getting time: {}", e),
    }

    // Test JSON query with exactly_one
    let test_data = serde_json::json!({"message": "Hello from test", "value": 42});
    match generated::admin::test_json_query(client, test_data).await {
        Ok(result) => println!("JSON query result: {:?}", result),
        Err(e) => println!("Error in JSON query: {}", e),
    }

    // Setup functions
    match generated::setup::create_users_table(client).await {
        Ok(_) => println!("Users table created successfully"),
        Err(e) => println!("Error creating table: {}", e),
    }

    // Users functions
    match generated::users::get_all_users(client).await {
        Ok(users) => println!("All users: {:?}", users),
        Err(e) => println!("Error listing users: {}", e),
    }
    println!("\nTo see the actual generated code, check src/generated/ directory");
    println!("Functions are organized into modules: admin.rs, setup.rs, users.rs, and mod.rs");
    println!("The code is regenerated automatically when queries.yaml changes!");

    Ok(())
}
